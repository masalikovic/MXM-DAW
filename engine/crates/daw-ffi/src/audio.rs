//! Lem antara adapter C++ (Oboe) dan mesin Rust (ADR-0003, ADR-0013).
//!
//! - Thread audio memanggil [`mxm_engine_render`] dari callback Oboe. Mesin
//!   dibaca lewat `AtomicPtr`, tanpa lock.
//! - Thread UI memegang `EventSender` di balik `Mutex`. Hanya UI yang
//!   memakainya, jadi kunci itu tidak pernah diperebutkan thread audio.
//! - Mesin dibebaskan di thread UI setelah adapter menutup stream; sesudah
//!   `close()` Oboe menjamin tidak ada callback lagi.

use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use daw_core::bank::PadBank;
use daw_core::engine::{Engine, EngineStats, EventSender};
use daw_core::pad::PadParams;
use daw_core::sampel::Sampel;
use daw_core::seq::{Pattern, SeqSettings, Sequence};

/// Sample rate mesin; adapter meminta Oboe mengonversi bila perangkat berbeda.
pub(crate) const SAMPLE_RATE: u32 = 48_000;

static MESIN: AtomicPtr<Engine> = AtomicPtr::new(ptr::null_mut());
static SISI_UI: Mutex<Option<SisiUi>> = Mutex::new(None);
/// Model 16 pad saat mesin tidak berjalan (mis. aplikasi di latar). Urutan
/// kunci selalu `SISI_UI` lalu `BANK`.
static BANK: Mutex<Option<PadBank>> = Mutex::new(None);
/// Pola dan pengaturan sequencer saat mesin tidak berjalan.
static SEKUENS: Mutex<Option<Sequence>> = Mutex::new(None);

struct SisiUi {
    pengirim: EventSender,
    stats: Arc<EngineStats>,
}

/// Dipanggil adapter C++ dari callback audio: isi `out` dengan
/// `frames × channels` sampel float interleaved.
///
/// # Safety
/// `out` harus menunjuk ke buffer berisi `frames × channels` `f32` yang boleh
/// ditulis, dan hanya satu thread audio yang memanggil fungsi ini.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mxm_engine_render(out: *mut f32, frames: i32, channels: i32) {
    let (Ok(frames), Ok(channels)) = (usize::try_from(frames), usize::try_from(channels)) else {
        return;
    };
    if out.is_null() || channels == 0 {
        return;
    }
    // SAFETY: dijamin pemanggil (adapter C++ meneruskan buffer Oboe).
    let buffer = unsafe { std::slice::from_raw_parts_mut(out, frames * channels) };
    let mesin = MESIN.load(Ordering::Acquire);
    if mesin.is_null() {
        buffer.fill(0.0);
        return;
    }
    // SAFETY: mesin hidup sampai `lepas_mesin`, yang baru dipanggil setelah
    // stream ditutup; hanya thread audio yang memakai `&mut`.
    unsafe { (*mesin).render(buffer, channels) };
}

/// Buat mesin baru dan pasang untuk thread audio. Mengganti mesin lama.
pub(crate) fn pasang_mesin() {
    let mut ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    let (bank, seq) = match ui.take() {
        Some(lama) => lama.pengirim.into_model(),
        None => (
            kunci_bank().take().unwrap_or_else(PadBank::factory),
            kunci_sekuens().take().unwrap_or_default(),
        ),
    };
    drop(ui);
    let (mesin, pengirim) = Engine::with_model(SAMPLE_RATE, bank, seq);
    let stats = Arc::clone(pengirim.stats());
    let lama = MESIN.swap(Box::into_raw(Box::new(mesin)), Ordering::AcqRel);
    *SISI_UI.lock().unwrap_or_else(PoisonError::into_inner) = Some(SisiUi { pengirim, stats });
    bebaskan(lama);
}

/// Lepas mesin. Hanya boleh dipanggil saat tidak ada callback audio berjalan.
/// Model tetap disimpan, supaya mesin berikutnya memainkan isi yang sama.
pub(crate) fn lepas_mesin() {
    let lama = MESIN.swap(ptr::null_mut(), Ordering::AcqRel);
    let ui = SISI_UI
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    if let Some(ui) = ui {
        let (bank, seq) = ui.pengirim.into_model();
        *kunci_bank() = Some(bank);
        *kunci_sekuens() = Some(seq);
    }
    bebaskan(lama);
}

fn kunci_bank() -> std::sync::MutexGuard<'static, Option<PadBank>> {
    BANK.lock().unwrap_or_else(PoisonError::into_inner)
}

fn kunci_sekuens() -> std::sync::MutexGuard<'static, Option<Sequence>> {
    SEKUENS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Baca model: dari mesin yang berjalan, atau dari bank yang disimpan.
pub(crate) fn baca<R>(f: impl FnOnce(&PadBank) -> R) -> R {
    let ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(ui) = ui.as_ref() {
        return f(ui.pengirim.bank());
    }
    let mut bank = kunci_bank();
    f(bank.get_or_insert_with(PadBank::factory))
}

/// Ubah parameter pad. Saat mesin berjalan, perubahan langsung terdengar.
pub(crate) fn atur_pad(pad: usize, params: &PadParams) -> bool {
    let mut ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(ui) = ui.as_mut() {
        return ui.pengirim.set_pad(pad, params);
    }
    let mut bank = kunci_bank();
    bank.get_or_insert_with(PadBank::factory)
        .set_params(pad, params)
        .is_some()
}

/// Baca model sequencer: dari mesin yang berjalan, atau yang disimpan.
pub(crate) fn baca_seq<R>(f: impl FnOnce(&Sequence) -> R) -> R {
    let ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(ui) = ui.as_ref() {
        return f(ui.pengirim.sequence());
    }
    let mut seq = kunci_sekuens();
    f(seq.get_or_insert_with(Sequence::default))
}

/// Ganti pola dan/atau pengaturan sequencer.
pub(crate) fn atur_seq(pola: Option<Pattern>, pengaturan: Option<SeqSettings>) -> bool {
    let mut ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(ui) = ui.as_mut() {
        let mut ok = true;
        if let Some(p) = pengaturan {
            ok &= ui.pengirim.set_settings(&p);
        }
        if let Some(p) = pola {
            ok &= ui.pengirim.set_pattern(p);
        }
        return ok;
    }
    let mut seq = kunci_sekuens();
    let seq = seq.get_or_insert_with(Sequence::default);
    if let Some(p) = pengaturan {
        seq.settings = p.sanitized();
    }
    if let Some(p) = pola {
        seq.pattern = p;
    }
    true
}

/// Perintah transport; `false` bila mesin belum berjalan.
pub(crate) fn transport(main: bool, rekam: bool) -> bool {
    SISI_UI
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
        .is_some_and(|ui| ui.pengirim.transport(main, rekam))
}

/// Rekam hidup/mati sambil bermain (overdub).
pub(crate) fn atur_rekam(rekam: bool) -> bool {
    SISI_UI
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
        .is_some_and(|ui| ui.pengirim.set_recording(rekam))
}

/// Status transport: (bermain, merekam, posisi tick). Sekaligus memasukkan
/// ketukan yang direkam ke pola, dikompensasi latensi keluaran dari Oboe.
pub(crate) fn status_transport() -> (bool, bool, i64) {
    let latensi = kesehatan().latency_ms.max(0.0);
    let mut ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(ui) = ui.as_mut() else {
        return (false, false, 0);
    };
    ui.pengirim.poll_recording(latensi);
    let s = &ui.stats;
    (s.playing(), s.recording(), s.position_ticks())
}

/// Ganti sampel pad.
pub(crate) fn atur_sampel(pad: usize, nama: &str, sampel: Sampel) -> bool {
    let mut ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(ui) = ui.as_mut() {
        return ui.pengirim.set_sample(pad, nama, sampel);
    }
    let mut bank = kunci_bank();
    bank.get_or_insert_with(PadBank::factory)
        .set_sample(pad, nama, std::sync::Arc::new(sampel))
        .is_some()
}

fn bebaskan(mesin: *mut Engine) {
    if !mesin.is_null() {
        // SAFETY: pointer berasal dari `Box::into_raw` di `pasang_mesin` dan
        // sudah dilepas dari `MESIN`, jadi tidak ada pemakai lain.
        drop(unsafe { Box::from_raw(mesin) });
    }
}

/// Kirim satu ketukan dari UI. `false` bila mesin belum berjalan, pad tidak
/// ada, atau antrean penuh.
pub(crate) fn ketuk(pad: usize, velocity: f32) -> bool {
    SISI_UI
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
        .is_some_and(|ui| ui.pengirim.hit(pad, velocity))
}

/// Lepas pad dari UI. `false` bila mesin belum berjalan atau pad tidak ada.
pub(crate) fn lepas(pad: usize) -> bool {
    SISI_UI
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
        .is_some_and(|ui| ui.pengirim.release(pad))
}

/// Bitmask pad yang sedang berbunyi (0 bila mesin belum berjalan).
pub(crate) fn pad_berbunyi() -> u32 {
    SISI_UI
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map_or(0, |ui| ui.stats.pads_sounding())
}

/// Nama suara ke-16 pad.
pub(crate) fn nama_pad() -> Vec<String> {
    baca(|b| {
        (0..daw_core::pad::PAD_COUNT)
            .map(|i| b.name(i).unwrap_or_default().to_owned())
            .collect()
    })
}

/// Statistik mesin: (suara aktif, ketukan dimainkan, ketukan dibuang).
pub(crate) fn statistik() -> (u32, u32, u32) {
    SISI_UI
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map_or((0, 0, 0), |ui| {
            (
                ui.stats.voices_active(),
                ui.stats.hits_played(),
                ui.stats.hits_dropped(),
            )
        })
}

/// Angka kesehatan dari adapter C++. Tata letak sama dengan
/// `Adapter::Kesehatan` di `mxm_engine_adapter.cpp`.
#[repr(C)]
#[derive(Default)]
pub(crate) struct KesehatanC {
    pub running: i32,
    pub sample_rate: i32,
    pub channels: i32,
    pub frames_per_burst: i32,
    pub buffer_size: i32,
    pub buffer_capacity: i32,
    pub xruns: i32,
    pub exclusive: i32,
    pub aaudio: i32,
    pub restarts: i32,
    pub latency_ms: f64,
    pub load_avg: f64,
    pub load_max: f64,
}

#[cfg(target_os = "android")]
mod adapter {
    use super::KesehatanC;

    unsafe extern "C" {
        fn mxm_audio_start() -> i32;
        fn mxm_audio_stop();
        fn mxm_audio_health(out: *mut KesehatanC);
    }

    pub(crate) fn mulai() -> i32 {
        // SAFETY: fungsi adapter tanpa prasyarat; aman dari thread mana pun.
        unsafe { mxm_audio_start() }
    }

    pub(crate) fn berhenti() {
        // SAFETY: seperti di atas.
        unsafe { mxm_audio_stop() }
    }

    pub(crate) fn kesehatan() -> KesehatanC {
        let mut k = KesehatanC::default();
        // SAFETY: `k` adalah struct #[repr(C)] yang cocok dengan sisi C++.
        unsafe { mxm_audio_health(&raw mut k) };
        k
    }
}

/// Di luar Android (tes host) tidak ada perangkat audio.
#[cfg(not(target_os = "android"))]
mod adapter {
    use super::KesehatanC;

    pub(crate) fn mulai() -> i32 {
        -1
    }

    pub(crate) fn berhenti() {}

    pub(crate) fn kesehatan() -> KesehatanC {
        KesehatanC {
            latency_ms: -1.0,
            xruns: -1,
            ..KesehatanC::default()
        }
    }
}

pub(crate) use adapter::{berhenti, kesehatan, mulai};

#[cfg(test)]
mod tests {
    use super::{
        Sampel, atur_pad, atur_sampel, baca, ketuk, lepas, lepas_mesin, mxm_engine_render,
        nama_pad, pad_berbunyi, pasang_mesin, statistik,
    };

    /// 0,1 detik sinus 441 Hz, 44,1 kHz mono 16-bit.
    fn wav_pendek() -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let spek = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::new(&mut buf, spek).unwrap();
        for i in 0..4_410 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
            let v =
                ((i as f32 * 441.0 / 44_100.0 * core::f32::consts::TAU).sin() * 20_000.0) as i16;
            w.write_sample(v).unwrap();
        }
        w.finalize().unwrap();
        buf.into_inner()
    }

    fn render(frames: usize) -> Vec<f32> {
        let mut buf = vec![1.0_f32; frames * 2];
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        // SAFETY: buffer berukuran tepat frames × 2.
        unsafe {
            mxm_engine_render(buf.as_mut_ptr(), frames as i32, 2);
        }
        buf
    }

    // Satu tes saja: state global (MESIN) dipakai bersama oleh semua tes.
    #[test]
    fn alur_lengkap_lewat_ffi() {
        // Tanpa mesin: buffer diisi hening, ketukan ditolak.
        assert!(render(64).iter().all(|&v| v == 0.0));
        assert!(!ketuk(0, 1.0));
        assert!(!lepas(0));
        assert_eq!(pad_berbunyi(), 0);
        assert_eq!(nama_pad()[0], "KICK", "nama kit pabrik walau mesin mati");

        pasang_mesin();
        assert!(ketuk(0, 1.0));
        assert!(ketuk(13, 1.0));
        assert!(!ketuk(16, 1.0), "pad 17 tidak ada");
        let x = render(2_400);
        assert!(x.iter().any(|v| v.abs() > 0.3), "kick tidak berbunyi");
        assert_eq!(statistik().1, 2);
        assert_eq!(pad_berbunyi(), (1 << 0) | (1 << 13));
        assert!(lepas(13));
        assert_eq!(nama_pad().len(), 16);

        // Suntingan saat berjalan bertahan setelah mesin dilepas dan dipasang lagi.
        let mut p = baca(|b| *b.params(3).unwrap());
        p.pitch = 5.0;
        assert!(atur_pad(3, &p));
        lepas_mesin();
        assert!(!ketuk(0, 1.0));
        assert!(render(64).iter().all(|&v| v == 0.0));
        assert!((baca(|b| b.params(3).unwrap().pitch) - 5.0).abs() < f32::EPSILON);
        assert!(atur_sampel(
            4,
            "BARU",
            Sampel::from_f32(&[0.5; 480], 1, 48_000)
        ));
        pasang_mesin();
        assert_eq!(nama_pad()[4], "BARU");
        assert!((baca(|b| b.params(3).unwrap().pitch) - 5.0).abs() < f32::EPSILON);

        // Impor WAV lewat API Dart: nama dari berkas, end = panjang sampel.
        let p = crate::api::pad_import_wav(7, "Vokal Saya.wav".to_owned(), wav_pendek()).unwrap();
        assert_eq!((p.start, p.end), (0, 4_410));
        assert_eq!(nama_pad()[7], "VOKAL-SA");
        assert!(crate::api::pad_import_wav(7, "x.wav".to_owned(), vec![1, 2, 3]).is_err());
        assert!(crate::api::pad_import_wav(16, "x.wav".to_owned(), wav_pendek()).is_err());
        assert!(ketuk(7, 1.0));
        assert!(
            render(512).iter().any(|v| v.abs() > 0.1),
            "sampel impor berbunyi"
        );
        lepas_mesin();
    }
}
