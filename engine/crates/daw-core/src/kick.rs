//! Kick sintetis: gelombang sinus dengan nada yang turun cepat dan volume yang
//! meluruh eksponensial. Ini suara pertama MXM-DAW (M1); kit pabrik lengkap
//! disintesis dengan cara serupa di M2 (ADR-0009 butir 6).
//!
//! Semua perhitungan per sampel hanya perkalian dan satu `sin`, tanpa alokasi,
//! jadi aman dipanggil dari thread audio (rencana §8.4).

use core::f32::consts::TAU;

/// Nada awal dan akhir sapuan pitch (Hz).
const NADA_AWAL: f32 = 150.0;
const NADA_AKHIR: f32 = 48.0;
/// Konstanta waktu sapuan pitch dan peluruhan volume (detik).
const TAU_NADA: f32 = 0.035;
const TAU_VOLUME: f32 = 0.18;
/// Rampa awal 1 ms supaya tidak ada klik (rencana §9: rampa minimum 1 ms).
const RAMPA_AWAL: f32 = 0.001;
/// Suara berhenti saat volume di bawah −80 dB.
const AMBANG_DIAM: f32 = 1.0e-4;

/// Satu suara kick yang sedang berbunyi atau diam.
#[derive(Clone, Copy, Debug)]
pub struct KickVoice {
    aktif: bool,
    /// Sampel sejak dipicu; dipakai untuk memilih suara tertua saat dicuri.
    umur: u32,
    fase: f32,
    amplop_nada: f32,
    amplop_volume: f32,
    rampa: f32,
    gain: f32,
}

/// Koefisien per sampel yang bergantung pada sample rate; dihitung sekali.
#[derive(Clone, Copy, Debug)]
pub struct KickParams {
    pengali_nada: f32,
    pengali_volume: f32,
    langkah_rampa: f32,
    detik_per_sampel: f32,
}

impl KickParams {
    /// Koefisien untuk `sample_rate` tertentu.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn new(sample_rate: u32) -> Self {
        let sr = sample_rate as f32;
        Self {
            pengali_nada: (-1.0 / (TAU_NADA * sr)).exp(),
            pengali_volume: (-1.0 / (TAU_VOLUME * sr)).exp(),
            langkah_rampa: 1.0 / (RAMPA_AWAL * sr),
            detik_per_sampel: 1.0 / sr,
        }
    }
}

impl Default for KickVoice {
    fn default() -> Self {
        Self {
            aktif: false,
            umur: 0,
            fase: 0.0,
            amplop_nada: 0.0,
            amplop_volume: 0.0,
            rampa: 0.0,
            gain: 0.0,
        }
    }
}

impl KickVoice {
    /// Mulai berbunyi dari awal dengan kekuatan `velocity` (0..=1).
    pub fn trigger(&mut self, velocity: f32) {
        *self = Self {
            aktif: true,
            umur: 0,
            fase: 0.0,
            amplop_nada: 1.0,
            amplop_volume: 1.0,
            rampa: 0.0,
            gain: velocity.clamp(0.0, 1.0),
        };
    }

    /// Apakah suara ini masih berbunyi.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.aktif
    }

    /// Umur suara dalam sampel.
    #[must_use]
    pub fn age(&self) -> u32 {
        self.umur
    }

    /// Sampel berikutnya (mono). Mengembalikan 0 bila suara diam.
    pub fn next_sample(&mut self, p: &KickParams) -> f32 {
        if !self.aktif {
            return 0.0;
        }
        let nada = NADA_AKHIR + (NADA_AWAL - NADA_AKHIR) * self.amplop_nada;
        let keluaran = self.fase.sin() * self.amplop_volume * self.rampa * self.gain;

        self.fase += TAU * nada * p.detik_per_sampel;
        if self.fase >= TAU {
            self.fase -= TAU;
        }
        self.amplop_nada *= p.pengali_nada;
        self.amplop_volume *= p.pengali_volume;
        self.rampa = (self.rampa + p.langkah_rampa).min(1.0);
        self.umur = self.umur.saturating_add(1);
        if self.amplop_volume < AMBANG_DIAM {
            self.aktif = false;
        }
        keluaran
    }
}
