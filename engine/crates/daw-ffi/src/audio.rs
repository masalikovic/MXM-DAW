//! Lem antara adapter C++ (Oboe) dan mesin Rust (ADR-0003, ADR-0013).
//!
//! - Thread audio memanggil [`mxm_engine_render`] dari callback Oboe. Mesin
//!   dibaca lewat `AtomicPtr`, tanpa lock.
//! - Thread UI memegang `EventSender` di balik `Mutex`. Hanya UI yang
//!   memakainya, jadi kunci itu tidak pernah diperebutkan thread audio.
//! - Mesin dibebaskan di thread UI setelah adapter menutup stream; sesudah
//!   `close()` Oboe menjamin tidak ada callback lagi.

use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use daw_core::bank::PadBank;
use daw_core::engine::{Engine, EngineStats, EventSender};
use daw_core::pad::{GROUP_COUNT, PAD_COUNT, PadParams};
use daw_core::riwayat::History;
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
/// Perubahan model yang terjadi tanpa mesin, ditambah perubahan dari mesin
/// yang sudah dilepas. Bersama `EventSender::changes` membentuk revisi yang
/// hanya naik (sinyal simpan otomatis).
static REVISI_DASAR: AtomicU64 = AtomicU64::new(0);
/// Simpan diblokir: proyek di disk dibuat versi aplikasi yang lebih baru dan
/// tidak boleh ditimpa.
static SIMPAN_DIBLOKIR: AtomicBool = AtomicBool::new(false);

fn naikkan_revisi() {
    REVISI_DASAR.fetch_add(1, Ordering::Relaxed);
}

/// Riwayat undo/redo; hidup lebih lama daripada mesin, seperti model.
static RIWAYAT: Mutex<Option<History<(PadBank, Sequence)>>> = Mutex::new(None);
/// Waktu perubahan terakhir, untuk menggabungkan satu gerakan (mis. memutar
/// kenop) menjadi satu langkah undo.
static UBAH_TERAKHIR: Mutex<Option<std::time::Instant>> = Mutex::new(None);
/// Perubahan yang berjarak kurang dari ini digabung ke langkah undo yang sama.
const JEDA_GABUNG: std::time::Duration = std::time::Duration::from_millis(700);

/// Simpan keadaan sebelum perubahan sebagai langkah undo. Bila `paksa`
/// salah, perubahan yang datang beruntun (< 0,7 s) digabung. Harus dipanggil
/// sebelum mengunci `SISI_UI`.
fn checkpoint(paksa: bool) {
    let sekarang = std::time::Instant::now();
    let mut terakhir = UBAH_TERAKHIR.lock().unwrap_or_else(PoisonError::into_inner);
    let gabung = !paksa && terakhir.is_some_and(|t| sekarang.duration_since(t) < JEDA_GABUNG);
    // Aksi tunggal (tempel, impor, rekam) tidak menelan gerakan sesudahnya.
    *terakhir = if paksa { None } else { Some(sekarang) };
    drop(terakhir);
    if gabung {
        return;
    }
    let model = salinan_model();
    RIWAYAT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get_or_insert_with(History::default)
        .checkpoint(model);
}

/// Pasang seluruh model (hasil undo/redo atau salin), di mesin bila berjalan.
fn terapkan_model(bank: PadBank, seq: Sequence) -> bool {
    let mut ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(ui) = ui.as_mut() {
        return ui.pengirim.apply_model(bank, seq);
    }
    drop(ui);
    naikkan_revisi();
    *kunci_bank() = Some(bank);
    *kunci_sekuens() = Some(seq);
    true
}

/// Undo (`maju` salah) atau redo (`maju` benar). `false` bila tidak ada.
pub(crate) fn undo_redo(maju: bool) -> bool {
    let sekarang = salinan_model();
    let hasil = {
        let mut r = RIWAYAT.lock().unwrap_or_else(PoisonError::into_inner);
        let r = r.get_or_insert_with(History::default);
        if maju {
            r.redo(sekarang)
        } else {
            r.undo(sekarang)
        }
    };
    let Some((bank, seq)) = hasil else {
        return false;
    };
    // Perubahan berikutnya membuka langkah undo baru, tidak digabung.
    *UBAH_TERAKHIR.lock().unwrap_or_else(PoisonError::into_inner) = None;
    terapkan_model(bank, seq)
}

/// (jumlah undo, jumlah redo).
pub(crate) fn jumlah_riwayat() -> (usize, usize) {
    RIWAYAT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map_or((0, 0), History::counts)
}

/// Salin suara pad (nomor global).
pub(crate) fn salin_pad(dari: usize, ke: usize) -> bool {
    checkpoint(true);
    let (mut bank, seq) = salinan_model();
    bank.copy_pad(dari, ke) && terapkan_model(bank, seq)
}

/// Salin grup (16 suara + pola).
pub(crate) fn salin_grup(dari: usize, ke: usize) -> bool {
    if dari >= GROUP_COUNT || ke >= GROUP_COUNT {
        return false;
    }
    checkpoint(true);
    let (mut bank, mut seq) = salinan_model();
    for i in 0..PAD_COUNT {
        bank.copy_pad(dari * PAD_COUNT + i, ke * PAD_COUNT + i);
    }
    seq.patterns[ke] = seq.patterns[dari].clone();
    terapkan_model(bank, seq)
}

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
        REVISI_DASAR.fetch_add(ui.pengirim.changes(), Ordering::Relaxed);
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
    checkpoint(false);
    let mut ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(ui) = ui.as_mut() {
        return ui.pengirim.set_pad(pad, params);
    }
    naikkan_revisi();
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

/// Ganti pola satu grup dan/atau pengaturan sequencer.
pub(crate) fn atur_seq(pola: Option<(usize, Pattern)>, pengaturan: Option<SeqSettings>) -> bool {
    checkpoint(false);
    let mut ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(ui) = ui.as_mut() {
        let mut ok = true;
        if let Some(p) = pengaturan {
            ok &= ui.pengirim.set_settings(&p);
        }
        if let Some((g, p)) = pola {
            ok &= ui.pengirim.set_pattern(g, p);
        }
        return ok;
    }
    naikkan_revisi();
    let mut seq = kunci_sekuens();
    let seq = seq.get_or_insert_with(Sequence::default);
    if let Some(p) = pengaturan {
        seq.settings = p.sanitized();
    }
    if let Some((g, p)) = pola {
        let Some(tujuan) = seq.patterns.get_mut(g) else {
            return false;
        };
        *tujuan = p;
    }
    true
}

/// Revisi model: naik setiap kali pad, sampel, pola, mute, atau pengaturan
/// berubah (termasuk rekaman).
pub(crate) fn revisi() -> u64 {
    let ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    REVISI_DASAR.load(Ordering::Relaxed) + ui.as_ref().map_or(0, |ui| ui.pengirim.changes())
}

/// Salinan model untuk disimpan. Murah: sampel dibagi lewat `Arc`.
pub(crate) fn salinan_model() -> (PadBank, Sequence) {
    let ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(ui) = ui.as_ref() {
        return (ui.pengirim.bank().clone(), ui.pengirim.sequence().clone());
    }
    drop(ui);
    let bank = kunci_bank().get_or_insert_with(PadBank::factory).clone();
    let seq = kunci_sekuens()
        .get_or_insert_with(Sequence::default)
        .clone();
    (bank, seq)
}

/// Pasang model hasil membuka proyek. Hanya saat mesin belum berjalan, supaya
/// tidak ada dua sumber kebenaran.
pub(crate) fn pasang_model(bank: PadBank, seq: Sequence) -> bool {
    let ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    if ui.is_some() {
        return false;
    }
    *kunci_bank() = Some(bank);
    *kunci_sekuens() = Some(seq);
    // Riwayat proyek lain tidak berlaku untuk proyek yang baru dibuka.
    *RIWAYAT.lock().unwrap_or_else(PoisonError::into_inner) = None;
    true
}

/// Blokir atau izinkan simpan.
pub(crate) fn blokir_simpan(blokir: bool) {
    SIMPAN_DIBLOKIR.store(blokir, Ordering::Relaxed);
}

pub(crate) fn simpan_diblokir() -> bool {
    SIMPAN_DIBLOKIR.load(Ordering::Relaxed)
}

/// Perintah transport; `false` bila mesin belum berjalan.
pub(crate) fn transport(main: bool, rekam: bool) -> bool {
    if main && rekam {
        // Satu sesi rekaman = satu langkah undo.
        checkpoint(true);
    }
    SISI_UI
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
        .is_some_and(|ui| ui.pengirim.transport(main, rekam))
}

/// Rekam hidup/mati sambil bermain (overdub).
pub(crate) fn atur_rekam(rekam: bool) -> bool {
    if rekam {
        checkpoint(true);
    }
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
    checkpoint(true);
    let mut ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(ui) = ui.as_mut() {
        return ui.pengirim.set_sample(pad, nama, sampel);
    }
    naikkan_revisi();
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

/// Ketukan dengan transpose (mode KEYS).
pub(crate) fn ketuk_nada(pad: usize, velocity: f32, nada: i8) -> bool {
    SISI_UI
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
        .is_some_and(|ui| ui.pengirim.hit_note(pad, velocity, nada))
}

/// Lepas satu nada pad.
pub(crate) fn lepas_nada(pad: usize, nada: i8) -> bool {
    SISI_UI
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
        .is_some_and(|ui| ui.pengirim.release_note(pad, nada))
}

/// Mulai/hentikan note repeat.
pub(crate) fn ulang(pad: usize, velocity: f32, nada: i8, rate: u32) -> bool {
    SISI_UI
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
        .is_some_and(|ui| ui.pengirim.repeat(pad, velocity, nada, rate))
}

/// Bitmask 16 pad grup `grup` yang sedang berbunyi (0 bila mesin belum
/// berjalan).
pub(crate) fn pad_berbunyi(grup: usize) -> u32 {
    let semua = SISI_UI
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map_or(0, |ui| ui.stats.pads_sounding());
    #[allow(clippy::cast_possible_truncation)]
    let bit = (semua >> (grup.min(3) * daw_core::pad::PAD_COUNT)) as u32 & 0xFFFF;
    bit
}

/// Mute atau buka grup.
pub(crate) fn atur_mute(grup: usize, mute: bool) -> bool {
    checkpoint(true);
    let mut ui = SISI_UI.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(ui) = ui.as_mut() {
        return ui.pengirim.set_mute(grup, mute);
    }
    naikkan_revisi();
    let mut seq = kunci_sekuens();
    seq.get_or_insert_with(Sequence::default)
        .mutes
        .get_mut(grup)
        .map(|m| *m = mute)
        .is_some()
}

/// Nama suara ke-64 pad (nomor global).
pub(crate) fn nama_pad() -> Vec<String> {
    baca(|b| {
        (0..daw_core::pad::TOTAL_PADS)
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
        Sampel, atur_pad, atur_sampel, baca, jumlah_riwayat, ketuk_nada, lepas_mesin, lepas_nada,
        mxm_engine_render, nama_pad, pad_berbunyi, pasang_mesin, salin_grup, salin_pad, statistik,
        undo_redo,
    };
    use crate::api::{
        NoteData, SeqConfig, group_mutes, project_load, project_revision, project_save, seq_config,
        set_group_mute, set_pattern_notes, set_seq_config, transport_play, transport_state,
        transport_stop,
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

    // Satu tes panjang, bukan banyak tes pendek: state global (MESIN, BANK,
    // SEKUENS) dipakai bersama, dan tes Rust berjalan paralel.
    #[test]
    #[allow(clippy::too_many_lines)]
    fn alur_lengkap_lewat_ffi() {
        // Tanpa mesin: buffer diisi hening, ketukan ditolak.
        assert!(render(64).iter().all(|&v| v == 0.0));
        assert!(!ketuk_nada(0, 1.0, 0));
        assert!(!lepas_nada(0, 0));
        assert_eq!(pad_berbunyi(0), 0);
        assert_eq!(nama_pad()[0], "KICK", "nama kit pabrik walau mesin mati");

        pasang_mesin();
        assert!(ketuk_nada(0, 1.0, 0));
        assert!(ketuk_nada(13, 1.0, 0));
        assert!(ketuk_nada(16, 1.0, 0), "pad 1 grup B");
        assert!(!ketuk_nada(64, 1.0, 0), "pad 65 tidak ada");
        let x = render(2_400);
        assert!(x.iter().any(|v| v.abs() > 0.3), "kick tidak berbunyi");
        assert_eq!(statistik().1, 3);
        assert_eq!(pad_berbunyi(0), (1 << 0) | (1 << 13));
        assert!(lepas_nada(13, 0));
        assert_eq!(nama_pad().len(), 64);

        // Suntingan saat berjalan bertahan setelah mesin dilepas dan dipasang lagi.
        let mut p = baca(|b| *b.params(3).unwrap());
        p.pitch = 5.0;
        assert!(atur_pad(3, &p));
        lepas_mesin();
        assert!(!ketuk_nada(0, 1.0, 0));
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
        assert!(crate::api::pad_import_wav(64, "x.wav".to_owned(), wav_pendek()).is_err());
        assert!(ketuk_nada(7, 1.0, 0));
        assert!(
            render(512).iter().any(|v| v.abs() > 0.1),
            "sampel impor berbunyi"
        );

        // Sequencer lewat API Dart: pengaturan dan not grup C, main, rekam.
        let c = set_seq_config(
            2,
            SeqConfig {
                bpm: 120.0,
                swing: 0.5,
                metronome: false,
                count_in_bars: 0,
                bars: 2,
                quantize_grid: 240,
                quantize_strength: 0.0,
            },
        );
        assert_eq!(c.bars, 2);
        assert_eq!(seq_config(0).bars, 1, "grup A tidak ikut berubah");
        let not = |tick| NoteData {
            tick,
            pad: 0,
            velocity: 1.0,
            length: 240,
            pitch: 0,
        };
        assert!(set_pattern_notes(2, vec![not(0), not(99_999)]));
        assert_eq!(transport_state(2).note_count, 1, "not di luar pola dibuang");
        assert!(!transport_state(2).playing);
        assert!(transport_play(true));
        render(24_000); // 120 BPM: 24 000 frame = 1 ketukan
        let st = transport_state(2);
        assert!(st.playing && st.recording);
        assert_eq!((st.bar, st.beat, st.step), (1, 2, 4));
        assert!(ketuk_nada(2 * 16 + 5, 0.8, 0)); // grup C, direkam di tick 960
        render(256);
        assert_eq!(transport_state(2).note_count, 2);
        assert_eq!(transport_state(0).note_count, 0);
        assert!(set_group_mute(2, true));
        assert_eq!(group_mutes(), [false, false, true, false]);
        assert!(!set_group_mute(4, true));
        assert!(transport_stop());
        render(256);
        assert!(!transport_state(2).playing);

        // Model sequencer bertahan setelah mesin dilepas.
        lepas_mesin();
        assert_eq!(transport_state(2).note_count, 2);
        assert_eq!(seq_config(2).bars, 2);
        assert_eq!(group_mutes(), [false, false, true, false]);
        assert!(!transport_play(false), "tanpa mesin tidak bisa main");

        // Undo/redo: tempel = satu langkah, satu gerakan kenop = satu langkah,
        // baik saat mesin mati maupun berjalan.
        let pitch3 = || baca(|b| b.params(3).unwrap().pitch);
        let (u0, _) = jumlah_riwayat();
        assert!(salin_pad(7, 8));
        assert_eq!(nama_pad()[8], "VOKAL-SA");
        let mut p = baca(|b| *b.params(3).unwrap());
        for i in 1..=3_u8 {
            p.pitch = 5.0 + f32::from(i);
            assert!(atur_pad(3, &p));
        }
        assert_eq!(jumlah_riwayat(), (u0 + 2, 0));
        assert!(undo_redo(false));
        assert!(
            (pitch3() - 5.0).abs() < f32::EPSILON,
            "tiga putaran batal sekaligus"
        );
        assert!(undo_redo(false));
        assert_ne!(nama_pad()[8], "VOKAL-SA");
        pasang_mesin();
        assert!(undo_redo(true), "redo saat mesin berjalan");
        assert_eq!(nama_pad()[8], "VOKAL-SA");
        assert!(ketuk_nada(8, 1.0, 0));
        assert!(
            render(512).iter().any(|v| v.abs() > 0.1),
            "suara tempelan berbunyi"
        );
        assert!(salin_grup(2, 3));
        assert_eq!(transport_state(3).note_count, 2, "pola ikut tersalin");
        assert!(undo_redo(false));
        assert_eq!(transport_state(3).note_count, 0);
        assert!(!salin_grup(2, 4));
        lepas_mesin();
        assert_eq!(
            jumlah_riwayat().1,
            1,
            "aksi baru menghapus redo; tersisa redo salin grup"
        );
        assert!((pitch3() - 5.0).abs() < f32::EPSILON);

        // Proyek: simpan, ubah, buka lagi → kembali ke yang disimpan.
        let dir = std::env::temp_dir().join(format!("mxm-daw-ffi-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let d = dir.to_string_lossy().into_owned();
        let r0 = project_revision();
        assert!(project_save(d.clone()).is_ok());
        let mut p = baca(|b| *b.params(3).unwrap());
        p.pitch = -9.0;
        assert!(atur_pad(3, &p));
        assert!(
            project_revision() > r0,
            "perubahan tanpa mesin menaikkan revisi"
        );
        assert_eq!(project_load(d.clone()), Ok(Vec::new()));
        assert!((baca(|b| b.params(3).unwrap().pitch) - 5.0).abs() < f32::EPSILON);
        assert_eq!(seq_config(2).bars, 2, "pola grup C ikut pulih");
        assert_eq!(nama_pad()[7], "VOKAL-SA", "sampel impor ikut pulih");

        // Revisi tetap naik saat mesin berjalan dan sesudah dilepas.
        pasang_mesin();
        let r1 = project_revision();
        assert!(atur_pad(3, &p));
        assert!(project_revision() > r1);
        assert!(
            project_load(d.clone()).is_err(),
            "tidak boleh saat mesin berjalan"
        );
        lepas_mesin();
        assert!(
            project_revision() > r1,
            "revisi tidak mundur setelah mesin dilepas"
        );

        // Proyek dari versi lebih baru: ditolak, dan simpan diblokir.
        std::fs::write(dir.join("project.json"), b"{\"schema\": 99}").unwrap();
        assert!(project_load(d.clone()).unwrap_err().contains("lebih baru"));
        assert!(project_save(d).unwrap_err().contains("diblokir"));
        assert_eq!(
            std::fs::read(dir.join("project.json")).unwrap(),
            b"{\"schema\": 99}",
            "berkas versi baru tidak tertimpa"
        );
        crate::audio::blokir_simpan(false);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
