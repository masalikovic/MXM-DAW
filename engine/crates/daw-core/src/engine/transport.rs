//! Posisi transport dan klik metronom, bagian thread audio (ADR-0015).
//!
//! Posisi **tidak** dijumlahkan per blok. Ia dihitung dari jangkar:
//! `tick = jangkar_tick + frame × tick_per_frame`, dengan `frame` bilangan bulat.
//! Jadi galat pembulatan tidak menumpuk walau dimainkan ribuan bar (rencana §9).

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use core::f32::consts::TAU;

use crate::seq::PPQN;

/// Posisi dan status transport.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Transport {
    pub main: bool,
    pub rekam: bool,
    pub metronom: bool,
    /// Frame sejak jangkar (bilangan bulat, tidak pernah dibulatkan).
    frame: u64,
    jangkar_tick: f64,
    tick_per_frame: f64,
    sr: f64,
}

impl Transport {
    pub fn new(sr: f32, bpm: f32) -> Self {
        let mut t = Self {
            main: false,
            rekam: false,
            metronom: true,
            frame: 0,
            jangkar_tick: 0.0,
            tick_per_frame: 0.0,
            sr: f64::from(sr),
        };
        t.atur_tempo(bpm);
        t
    }

    /// Tick pada `frame` relatif terhadap jangkar.
    pub fn tick_di(&self, frame: u64) -> f64 {
        self.jangkar_tick + frame as f64 * self.tick_per_frame
    }

    /// Tick di awal blok berikutnya.
    pub fn tick(&self) -> f64 {
        self.tick_di(self.frame)
    }

    /// Frame pertama yang tick-nya ≥ `tick`.
    pub fn frame_untuk(&self, tick: f64) -> u64 {
        let f = (tick - self.jangkar_tick) / self.tick_per_frame;
        // Toleransi kecil supaya tick yang tepat jatuh di frame bulat tidak
        // terdorong ke frame berikutnya karena galat pembulatan.
        (f - 1e-7).ceil().max(0.0) as u64
    }

    pub fn tick_per_frame(&self) -> f64 {
        self.tick_per_frame
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn maju(&mut self, frames: u64) {
        if self.main {
            self.frame += frames;
        }
    }

    /// Mulai dari `tick_awal` (negatif saat count-in).
    pub fn mulai(&mut self, tick_awal: f64, rekam: bool) {
        self.main = true;
        self.rekam = rekam;
        self.frame = 0;
        self.jangkar_tick = tick_awal;
    }

    pub fn berhenti(&mut self) {
        self.main = false;
        self.rekam = false;
        self.frame = 0;
        self.jangkar_tick = 0.0;
    }

    /// Ganti tempo tanpa melompat: jangkar dipindah ke posisi sekarang.
    pub fn atur_tempo(&mut self, bpm: f32) {
        self.jangkar_tick = self.tick();
        self.frame = 0;
        self.tick_per_frame = bpm_tepat(bpm) * f64::from(PPQN) / (60.0 * self.sr);
    }
}

/// BPM sebagai f64 tanpa sisa f32: 123,4 dalam f32 sebenarnya 123,40000153,
/// dan selisih sekecil itu menggeser not satu sampel setelah puluhan bar. BPM
/// selalu bernilai satu desimal (rencana §4.3), jadi dibulatkan ke sana.
pub(crate) fn bpm_tepat(bpm: f32) -> f64 {
    (f64::from(bpm) * 10.0).round() / 10.0
}

/// Bunyi klik metronom: sinus pendek yang meluruh, dibuat di tempat (tanpa
/// sampel). Ketukan pertama bar lebih tinggi dan lebih keras.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Klik {
    fase: f32,
    langkah: f32,
    amp: f32,
    k: f32,
}

impl Klik {
    pub fn picu(&mut self, awal_bar: bool, sr: f32) {
        let hz = if awal_bar { 1_600.0 } else { 1_000.0 };
        self.langkah = TAU * hz / sr;
        // Mulai satu langkah setelah nol, supaya sampel pertama sudah berbunyi
        // dan klik jatuh tepat di sampelnya.
        self.fase = self.langkah;
        self.amp = if awal_bar { 0.5 } else { 0.32 };
        self.k = (-1.0 / (0.015 * sr)).exp();
    }

    #[inline]
    pub fn berikut(&mut self) -> f32 {
        if self.amp < 1e-4 {
            return 0.0;
        }
        let y = self.fase.sin() * self.amp;
        self.fase = (self.fase + self.langkah) % TAU;
        self.amp *= self.k;
        y
    }
}

#[cfg(test)]
mod tests {
    use super::Transport;

    #[test]
    fn posisi_dari_jangkar_tidak_menumpuk_galat() {
        let mut t = Transport::new(48_000.0, 123.4);
        t.mulai(0.0, false);
        // Satu juta blok aneh (97 frame) = ±33 menit.
        for _ in 0..1_000_000 {
            t.maju(97);
        }
        let harapan = 97_000_000.0 * 123.4 * 960.0 / (60.0 * 48_000.0);
        assert!(
            (t.tick() - harapan).abs() < 1e-6,
            "{} vs {harapan}",
            t.tick()
        );
    }

    #[test]
    fn frame_untuk_tick_tepat() {
        let mut t = Transport::new(48_000.0, 120.0);
        t.mulai(0.0, false);
        // 120 BPM: 960 tick = 0,5 s = 24 000 frame.
        assert_eq!(t.frame_untuk(960.0), 24_000);
        assert_eq!(t.frame_untuk(0.0), 0);
        assert_eq!(t.frame_untuk(0.5), 13, "dibulatkan ke atas");
    }

    #[test]
    fn ganti_tempo_tanpa_lompatan() {
        let mut t = Transport::new(48_000.0, 120.0);
        t.mulai(0.0, false);
        t.maju(24_000);
        let sebelum = t.tick();
        t.atur_tempo(90.0);
        assert!((t.tick() - sebelum).abs() < 1e-9);
        t.maju(32_000); // 90 BPM: 960 tick = 32 000 frame
        assert!((t.tick() - sebelum - 960.0).abs() < 1e-6);
    }

    #[test]
    fn berhenti_tidak_maju() {
        let mut t = Transport::new(48_000.0, 120.0);
        t.maju(1_000);
        assert!(t.tick().abs() < f64::EPSILON);
    }
}
