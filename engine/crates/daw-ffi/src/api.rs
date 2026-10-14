//! Fungsi yang bisa dipanggil dari Dart.
//!
//! Setiap fungsi publik di modul ini dibaca oleh `flutter_rust_bridge_codegen`
//! dan diberi pasangan Dart di `app/lib/engine/bridge/` (ADR-0004). Modul ini
//! hanya lapisan tipis: logika tetap di `daw-core` (ADR-0002).

/// Versi mesin, mis. `"0.1.0"`.
///
/// Ditandai `sync`: Dart memanggilnya langsung di thread UI tanpa menunggu
/// `Future`. Aman karena fungsinya hanya membaca konstanta yang ditanam
/// compiler (lihat `daw_core::engine_version`).
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn engine_version() -> String {
    daw_core::engine_version().to_owned()
}

/// Angka kesehatan mesin untuk panel di layar (rencana §6.3, §11).
///
/// Nilai −1 berarti "tidak diketahui" (mis. perangkat tidak melaporkannya).
pub struct AudioHealth {
    /// Stream audio sedang berjalan.
    pub running: bool,
    /// Sample rate stream (Hz).
    pub sample_rate: i32,
    /// Ukuran burst perangkat (frame).
    pub frames_per_burst: i32,
    /// Ukuran buffer saat ini (frame); dinaikkan `LatencyTuner` bila underrun.
    pub buffer_size: i32,
    /// Jumlah underrun sejak stream dibuka.
    pub xruns: i32,
    /// Latensi keluaran yang dilaporkan Oboe (ms).
    pub latency_ms: f64,
    /// Beban callback rata-rata (waktu render ÷ periode buffer).
    pub load_avg: f64,
    /// Beban callback tertinggi.
    pub load_max: f64,
    /// Mode eksklusif didapat (bukan jatuh ke shared).
    pub exclusive: bool,
    /// API yang dipakai adalah `AAudio` (bukan `OpenSL ES`).
    pub aaudio: bool,
    /// Berapa kali stream dibuka ulang (mis. headset dicabut).
    pub restarts: i32,
    /// Suara yang sedang berbunyi.
    pub voices_active: u32,
    /// Ketukan yang sudah dimainkan.
    pub hits_played: u32,
    /// Ketukan yang dibuang karena antrean penuh.
    pub hits_dropped: u32,
}

/// Mulai mesin dan stream audio. Aman dipanggil berulang.
///
/// # Errors
/// Pesan galat bila stream tidak bisa dibuka (atau di luar Android).
pub fn audio_start() -> Result<(), String> {
    if crate::audio::kesehatan().running != 0 {
        return Ok(());
    }
    crate::audio::pasang_mesin();
    match crate::audio::mulai() {
        0 => Ok(()),
        kode => {
            crate::audio::lepas_mesin();
            Err(format!("stream audio gagal dibuka (kode Oboe {kode})"))
        }
    }
}

/// Hentikan stream audio dan bebaskan mesin (mis. saat aplikasi ke latar).
pub fn audio_stop() {
    crate::audio::berhenti();
    crate::audio::lepas_mesin();
}

/// Pad ditekan dengan kekuatan 0..=1. `false` bila mesin belum berjalan.
///
/// `sync`: dipanggil langsung dari event sentuh, hanya memasukkan event ke
/// antrean (mikrodetik), tanpa menunggu thread audio (rencana §7).
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn pad_hit(velocity: f32) -> bool {
    // M2 langkah 1: pad di layar masih satu, dan memainkan pad 1 (KICK).
    crate::audio::ketuk(0, velocity)
}

/// Angka kesehatan mesin saat ini.
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn audio_health() -> AudioHealth {
    let k = crate::audio::kesehatan();
    let (voices_active, hits_played, hits_dropped) = crate::audio::statistik();
    AudioHealth {
        running: k.running != 0,
        sample_rate: k.sample_rate,
        frames_per_burst: k.frames_per_burst,
        buffer_size: k.buffer_size,
        xruns: k.xruns,
        latency_ms: k.latency_ms,
        load_avg: k.load_avg,
        load_max: k.load_max,
        exclusive: k.exclusive != 0,
        aaudio: k.aaudio != 0,
        restarts: k.restarts,
        voices_active,
        hits_played,
        hits_dropped,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn versi_lewat_ffi_sama_dengan_versi_inti() {
        assert_eq!(super::engine_version(), daw_core::engine_version());
    }
}
