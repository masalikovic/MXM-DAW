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

/// Pad `pad` (0..16) ditekan dengan kekuatan 0..=1. `false` bila mesin belum
/// berjalan atau nomor pad salah.
///
/// `sync`: dipanggil langsung dari event sentuh, hanya memasukkan event ke
/// antrean (mikrodetik), tanpa menunggu thread audio (rencana §7).
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn pad_hit(pad: u8, velocity: f32) -> bool {
    crate::audio::ketuk(usize::from(pad), velocity)
}

/// Pad `pad` dilepas (penting untuk mode gate dan loop).
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn pad_release(pad: u8) -> bool {
    crate::audio::lepas(usize::from(pad))
}

/// Bit ke-`n` menyala bila pad `n` sedang berbunyi; untuk LED pad. Murah
/// (satu baca atomik), jadi aman dipanggil 30 kali per detik.
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn pads_sounding() -> u32 {
    crate::audio::pad_berbunyi()
}

/// Nama suara ke-16 pad, mis. `"KICK"`.
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn pad_names() -> Vec<String> {
    crate::audio::nama_pad()
}

/// Cara pad dimainkan (cermin `daw_core::pad::PlayMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayKind {
    /// Berbunyi sampai selesai walau dilepas.
    OneShot,
    /// Berbunyi selama ditekan.
    Gate,
    /// Mengulang selama ditekan.
    Loop,
}

/// Jenis filter (cermin `daw_core::pad::FilterMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterKind {
    /// Tanpa filter.
    Off,
    /// Low-pass.
    LowPass,
    /// High-pass.
    HighPass,
    /// Band-pass.
    BandPass,
}

/// Parameter satu pad untuk layar Sound. Satuan sama dengan rencana §10.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadSettings {
    /// Level 0..1.
    pub level: f32,
    /// Pan −1..1.
    pub pan: f32,
    /// Pitch dalam semitone (−24..24), pecahan = cent.
    pub pitch: f32,
    /// Frame awal.
    pub start: u32,
    /// Frame akhir (eksklusif).
    pub end: u32,
    /// Attack (detik).
    pub attack: f32,
    /// Decay (detik).
    pub decay: f32,
    /// Sustain 0..1.
    pub sustain: f32,
    /// Release (detik).
    pub release: f32,
    /// Jenis filter.
    pub filter: FilterKind,
    /// Cutoff 0..1 (20 Hz … 20 kHz).
    pub cutoff: f32,
    /// Resonansi 0..1.
    pub resonance: f32,
    /// Drive 0..1.
    pub drive: f32,
    /// Cara main.
    pub play: PlayKind,
    /// Choke group 1..=8, atau 0.
    pub choke: u8,
}

impl From<&daw_core::pad::PadParams> for PadSettings {
    fn from(p: &daw_core::pad::PadParams) -> Self {
        use daw_core::pad::{FilterMode, PlayMode};
        Self {
            level: p.level,
            pan: p.pan,
            pitch: p.pitch,
            start: p.start,
            end: p.end,
            attack: p.envelope.attack,
            decay: p.envelope.decay,
            sustain: p.envelope.sustain,
            release: p.envelope.release,
            filter: match p.filter.mode {
                FilterMode::Off => FilterKind::Off,
                FilterMode::LowPass => FilterKind::LowPass,
                FilterMode::HighPass => FilterKind::HighPass,
                FilterMode::BandPass => FilterKind::BandPass,
            },
            cutoff: p.filter.cutoff,
            resonance: p.filter.resonance,
            drive: p.drive,
            play: match p.play_mode {
                PlayMode::OneShot => PlayKind::OneShot,
                PlayMode::Gate => PlayKind::Gate,
                PlayMode::Loop => PlayKind::Loop,
            },
            choke: p.choke,
        }
    }
}

impl From<&PadSettings> for daw_core::pad::PadParams {
    fn from(s: &PadSettings) -> Self {
        use daw_core::pad::{Envelope, Filter, FilterMode, PlayMode};
        Self {
            level: s.level,
            pan: s.pan,
            pitch: s.pitch,
            start: s.start,
            end: s.end,
            envelope: Envelope {
                attack: s.attack,
                decay: s.decay,
                sustain: s.sustain,
                release: s.release,
            },
            filter: Filter {
                mode: match s.filter {
                    FilterKind::Off => FilterMode::Off,
                    FilterKind::LowPass => FilterMode::LowPass,
                    FilterKind::HighPass => FilterMode::HighPass,
                    FilterKind::BandPass => FilterMode::BandPass,
                },
                cutoff: s.cutoff,
                resonance: s.resonance,
            },
            drive: s.drive,
            play_mode: match s.play {
                PlayKind::OneShot => PlayMode::OneShot,
                PlayKind::Gate => PlayMode::Gate,
                PlayKind::Loop => PlayMode::Loop,
            },
            choke: s.choke,
        }
    }
}

/// Parameter pad saat ini, atau `None` bila nomor pad salah.
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn pad_settings(pad: u8) -> Option<PadSettings> {
    crate::audio::baca(|b| b.params(usize::from(pad)).map(PadSettings::from))
}

/// Simpan parameter pad (nilai dibatasi ke jangkauan sah). Langsung terdengar
/// bila mesin berjalan. Mengembalikan nilai yang benar-benar tersimpan.
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn set_pad_settings(pad: u8, settings: PadSettings) -> Option<PadSettings> {
    let params = daw_core::pad::PadParams::from(&settings);
    if !crate::audio::atur_pad(usize::from(pad), &params) {
        return None;
    }
    pad_settings(pad)
}

/// Panjang sampel pad (frame) dan sample rate-nya; (0, 0) bila kosong.
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn pad_sample_info(pad: u8) -> (u32, u32) {
    crate::audio::baca(|b| {
        b.sample(usize::from(pad)).map_or((0, 0), |s| {
            (
                u32::try_from(s.frames()).unwrap_or(u32::MAX),
                s.sample_rate(),
            )
        })
    })
}

/// Ringkasan gelombang sampel pad: `points` pasangan (min, max) berurutan
/// `[min0, max0, min1, max1, …]`, untuk digambar sekali lalu disimpan.
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn pad_peaks(pad: u8, points: u32) -> Vec<f32> {
    let points = usize::try_from(points).unwrap_or(0).min(4_096);
    crate::audio::baca(|b| {
        b.sample(usize::from(pad))
            .map(|s| {
                s.peaks(points)
                    .into_iter()
                    .flat_map(|(lo, hi)| [lo, hi])
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// Impor WAV ke pad `pad`. `file_name` dipakai untuk nama pad (huruf besar,
/// maksimal 8 karakter). Byte dibaca Kotlin lewat Storage Access Framework,
/// jadi aplikasi tidak butuh izin penyimpanan.
///
/// Tidak `sync`: membaca WAV besar bisa memakan puluhan milidetik, jadi
/// dijalankan di thread pekerja, bukan thread UI.
///
/// # Errors
/// Pesan yang bisa ditampilkan ke pengguna bila WAV tidak bisa dibaca.
// flutter_rust_bridge meneruskan argumen Dart sebagai nilai milik sendiri.
#[allow(clippy::needless_pass_by_value)]
pub fn pad_import_wav(pad: u8, file_name: String, bytes: Vec<u8>) -> Result<PadSettings, String> {
    let sampel = daw_io::decode_wav(&bytes)?;
    drop(bytes);
    let nama = daw_io::pad_name_from_file(&file_name);
    if !crate::audio::atur_sampel(usize::from(pad), &nama, sampel) {
        return Err("pad tidak ada atau mesin sibuk; coba lagi".to_owned());
    }
    pad_settings(pad).ok_or_else(|| "pad tidak ada".to_owned())
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
    use super::{FilterKind, PadSettings, PlayKind};

    #[test]
    fn versi_lewat_ffi_sama_dengan_versi_inti() {
        assert_eq!(super::engine_version(), daw_core::engine_version());
    }

    #[test]
    fn pengaturan_pad_bolak_balik_tanpa_kehilangan() {
        let mut p = daw_core::pad::PadParams {
            pitch: -3.25,
            end: 1234,
            choke: 2,
            ..Default::default()
        };
        p.envelope.release = 0.5;
        p.filter.mode = daw_core::pad::FilterMode::BandPass;
        p.play_mode = daw_core::pad::PlayMode::Loop;
        let s = PadSettings::from(&p);
        assert_eq!((s.filter, s.play), (FilterKind::BandPass, PlayKind::Loop));
        assert_eq!(daw_core::pad::PadParams::from(&s), p);
    }
}
