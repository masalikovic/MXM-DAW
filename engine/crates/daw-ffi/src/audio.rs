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

use daw_core::engine::{Engine, EngineStats, EventSender};

/// Sample rate mesin; adapter meminta Oboe mengonversi bila perangkat berbeda.
pub(crate) const SAMPLE_RATE: u32 = 48_000;

static MESIN: AtomicPtr<Engine> = AtomicPtr::new(ptr::null_mut());
static SISI_UI: Mutex<Option<SisiUi>> = Mutex::new(None);

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
    let (mesin, pengirim) = Engine::new(SAMPLE_RATE);
    let stats = Arc::clone(pengirim.stats());
    let lama = MESIN.swap(Box::into_raw(Box::new(mesin)), Ordering::AcqRel);
    *SISI_UI.lock().unwrap_or_else(PoisonError::into_inner) = Some(SisiUi { pengirim, stats });
    bebaskan(lama);
}

/// Lepas mesin. Hanya boleh dipanggil saat tidak ada callback audio berjalan.
pub(crate) fn lepas_mesin() {
    let lama = MESIN.swap(ptr::null_mut(), Ordering::AcqRel);
    *SISI_UI.lock().unwrap_or_else(PoisonError::into_inner) = None;
    bebaskan(lama);
}

fn bebaskan(mesin: *mut Engine) {
    if !mesin.is_null() {
        // SAFETY: pointer berasal dari `Box::into_raw` di `pasang_mesin` dan
        // sudah dilepas dari `MESIN`, jadi tidak ada pemakai lain.
        drop(unsafe { Box::from_raw(mesin) });
    }
}

/// Kirim satu ketukan dari UI. `false` bila mesin belum berjalan atau antrean penuh.
pub(crate) fn ketuk(velocity: f32) -> bool {
    SISI_UI
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
        .is_some_and(|ui| ui.pengirim.hit(velocity))
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
    use super::{ketuk, lepas_mesin, mxm_engine_render, pasang_mesin, statistik};

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
        assert!(!ketuk(1.0));

        pasang_mesin();
        assert!(ketuk(1.0));
        let x = render(2_400);
        assert!(x.iter().any(|v| v.abs() > 0.3), "kick tidak berbunyi");
        assert_eq!(statistik().1, 1);

        lepas_mesin();
        assert!(!ketuk(1.0));
        assert!(render(64).iter().all(|&v| v == 0.0));
    }
}
