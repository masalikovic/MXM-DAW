//! Parameter satu pad (rencana §4.1, §10). Nama dan satuannya mengikuti
//! format proyek JSON di rencana §10, supaya M4 tinggal menyimpannya.

/// Jumlah pad per grup.
pub const PAD_COUNT: usize = 16;
/// Jumlah choke group (1..=8; 0 berarti tanpa choke).
pub const CHOKE_GROUPS: u8 = 8;

/// Cara pad dimainkan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayMode {
    /// Berbunyi sampai sampel/amplop selesai, walaupun pad sudah dilepas.
    #[default]
    OneShot,
    /// Berbunyi selama pad ditekan; dilepas = release.
    Gate,
    /// Mengulang start..end selama pad ditekan.
    Loop,
}

/// Jenis filter per pad.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FilterMode {
    /// Tanpa filter (paling hemat).
    #[default]
    Off,
    /// Low-pass: meredam nada tinggi.
    LowPass,
    /// High-pass: meredam nada rendah.
    HighPass,
    /// Band-pass: hanya sekitar cutoff.
    BandPass,
}

/// Amplop ADSR. Waktu dalam detik, `sustain` 0..1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Envelope {
    /// Waktu naik (detik, minimal 1 ms).
    pub attack: f32,
    /// Waktu turun 60 dB menuju sustain (detik).
    pub decay: f32,
    /// Level tahan 0..1.
    pub sustain: f32,
    /// Waktu turun 60 dB setelah dilepas (detik).
    pub release: f32,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            attack: 0.001,
            decay: 0.0,
            sustain: 1.0,
            release: 0.02,
        }
    }
}

/// Filter per pad.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Filter {
    /// Jenis filter.
    pub mode: FilterMode,
    /// 0..1, dipetakan eksponensial 20 Hz … 20 kHz.
    pub cutoff: f32,
    /// 0..1.
    pub resonance: f32,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            mode: FilterMode::Off,
            cutoff: 1.0,
            resonance: 0.0,
        }
    }
}

/// Semua parameter satu pad. `Copy`, supaya bisa dikirim lewat antrean ke
/// thread audio tanpa alokasi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadParams {
    /// Level 0..1.
    pub level: f32,
    /// Pan −1 (kiri) … 1 (kanan).
    pub pan: f32,
    /// Pitch dalam semitone (−24..24); pecahan = cent.
    pub pitch: f32,
    /// Frame awal sampel.
    pub start: u32,
    /// Frame akhir sampel (eksklusif); dibatasi ke panjang sampel.
    pub end: u32,
    /// Amplop.
    pub envelope: Envelope,
    /// Filter.
    pub filter: Filter,
    /// Saturasi 0..1 (0 = bersih).
    pub drive: f32,
    /// Cara main.
    pub play_mode: PlayMode,
    /// Choke group 1..=8, atau 0 tanpa choke.
    pub choke: u8,
}

impl Default for PadParams {
    fn default() -> Self {
        Self {
            level: 1.0,
            pan: 0.0,
            pitch: 0.0,
            start: 0,
            end: u32::MAX,
            envelope: Envelope::default(),
            filter: Filter::default(),
            drive: 0.0,
            play_mode: PlayMode::OneShot,
            choke: 0,
        }
    }
}

/// `x` di dalam `[min, max]`; NaN menjadi `bawaan`.
fn batasi(x: f32, min: f32, max: f32, bawaan: f32) -> f32 {
    if x.is_nan() {
        bawaan
    } else {
        x.clamp(min, max)
    }
}

impl PadParams {
    /// Salinan dengan semua nilai di dalam jangkauan yang sah. Dipanggil di
    /// sisi UI sebelum dikirim, jadi thread audio tidak perlu memeriksa lagi.
    #[must_use]
    pub fn sanitized(&self) -> Self {
        let b = Self::default();
        let e = &self.envelope;
        Self {
            level: batasi(self.level, 0.0, 1.0, b.level),
            pan: batasi(self.pan, -1.0, 1.0, 0.0),
            pitch: batasi(self.pitch, -24.0, 24.0, 0.0),
            start: self.start.min(self.end),
            end: self.end.max(self.start),
            envelope: Envelope {
                attack: batasi(e.attack, 0.001, 10.0, 0.001),
                decay: batasi(e.decay, 0.0, 10.0, 0.0),
                sustain: batasi(e.sustain, 0.0, 1.0, 1.0),
                release: batasi(e.release, 0.001, 10.0, 0.02),
            },
            filter: Filter {
                mode: self.filter.mode,
                cutoff: batasi(self.filter.cutoff, 0.0, 1.0, 1.0),
                resonance: batasi(self.filter.resonance, 0.0, 1.0, 0.0),
            },
            drive: batasi(self.drive, 0.0, 1.0, 0.0),
            play_mode: self.play_mode,
            choke: if self.choke <= CHOKE_GROUPS {
                self.choke
            } else {
                0
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PadParams;

    #[test]
    fn nilai_liar_dibatasi() {
        let mut p = PadParams {
            level: 3.0,
            pan: f32::NAN,
            pitch: -100.0,
            start: 500,
            end: 100,
            drive: -1.0,
            choke: 99,
            ..PadParams::default()
        };
        p.envelope.attack = 0.0;
        p.filter.cutoff = f32::INFINITY;
        let s = p.sanitized();
        assert_eq!(
            (s.level, s.pan, s.pitch, s.drive, s.choke),
            (1.0, 0.0, -24.0, 0.0, 0)
        );
        assert!(s.start <= s.end);
        assert!((s.envelope.attack - 0.001).abs() < f32::EPSILON);
        assert!((s.filter.cutoff - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn bawaan_sudah_sah() {
        assert_eq!(PadParams::default().sanitized(), PadParams::default());
    }
}
