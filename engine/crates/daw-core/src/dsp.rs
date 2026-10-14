//! Blok DSP kecil yang dipakai suara sampler dan sintesis kit: interpolasi
//! Hermite, amplop ADSR, dan filter state-variable (rencana §9).
//!
//! Semua fungsi di sini hanya berhitung: tanpa alokasi, tanpa lock, sehingga
//! aman dipanggil dari thread audio (rencana §8.4).

// Konversi angka antara frame (bilangan bulat) dan detik/Hz (float) memang
// pekerjaan modul ini; nilainya selalu jauh di dalam jangkauan tipe.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use core::f32::consts::PI;

use crate::pad::{Envelope, FilterMode};

/// `ln(1000)`: waktu decay/release diartikan sebagai waktu turun 60 dB.
const LN_1000: f32 = 6.907_755;
/// Di bawah −80 dB amplop dianggap selesai.
pub const AMBANG_DIAM: f32 = 1.0e-4;
/// Rampa minimum supaya tidak ada klik (rencana §9).
pub const RAMPA_MIN: f32 = 0.001;
/// Release cepat untuk choke group (detik).
const WAKTU_CEKIK: f32 = 0.005;

/// Interpolasi Hermite 4-titik antara `x0` dan `x1` pada posisi `t` (0..1).
///
/// Lebih halus daripada linear saat sampel dimainkan dengan pitch berbeda,
/// dengan biaya hanya beberapa perkalian.
/// Nama satu huruf mengikuti rumus aslinya (Laurent de Soras).
#[inline]
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn hermite(xm1: f32, x0: f32, x1: f32, x2: f32, t: f32) -> f32 {
    let c = (x1 - xm1) * 0.5;
    let v = x0 - x1;
    let w = c + v;
    let a = w + v + (x2 - x0) * 0.5;
    let b = w + a;
    ((a * t - b) * t + c) * t + x0
}

/// Pengali per sampel supaya nilai turun 60 dB dalam `detik`.
#[inline]
fn pengali_60db(detik: f32, sr: f32) -> f32 {
    (-LN_1000 / (detik.max(RAMPA_MIN) * sr)).exp()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tahap {
    Diam,
    Serang,
    Luruh,
    Lepas,
    Cekik,
}

/// Amplop ADSR eksponensial. Attack naik linear (minimal 1 ms), decay dan
/// release turun eksponensial, jadi tidak ada lompatan nilai yang terdengar
/// sebagai klik.
#[derive(Clone, Copy, Debug)]
pub struct Adsr {
    tahap: Tahap,
    nilai: f32,
    langkah_serang: f32,
    k_luruh: f32,
    k_lepas: f32,
    k_cekik: f32,
    sustain: f32,
}

impl Default for Adsr {
    fn default() -> Self {
        Self {
            tahap: Tahap::Diam,
            nilai: 0.0,
            langkah_serang: 1.0,
            k_luruh: 0.0,
            k_lepas: 0.0,
            k_cekik: 0.0,
            sustain: 1.0,
        }
    }
}

impl Adsr {
    /// Hitung koefisien dari parameter. Boleh dipanggil setiap blok, jadi
    /// perubahan parameter langsung terdengar pada suara yang sedang berbunyi.
    pub fn atur(&mut self, e: &Envelope, sr: f32) {
        self.langkah_serang = 1.0 / (e.attack.max(RAMPA_MIN) * sr);
        self.k_luruh = pengali_60db(e.decay, sr);
        self.k_lepas = pengali_60db(e.release, sr);
        self.k_cekik = pengali_60db(WAKTU_CEKIK, sr);
        self.sustain = e.sustain.clamp(0.0, 1.0);
    }

    /// Mulai dari nol.
    pub fn mulai(&mut self) {
        self.tahap = Tahap::Serang;
        self.nilai = 0.0;
    }

    /// Pad dilepas: masuk ke release.
    pub fn lepas(&mut self) {
        if matches!(self.tahap, Tahap::Serang | Tahap::Luruh) {
            self.tahap = Tahap::Lepas;
        }
    }

    /// Dipotong choke group: release cepat 5 ms.
    pub fn cekik(&mut self) {
        if self.tahap != Tahap::Diam {
            self.tahap = Tahap::Cekik;
        }
    }

    /// Apakah amplop masih menghasilkan suara.
    #[must_use]
    pub fn aktif(&self) -> bool {
        self.tahap != Tahap::Diam
    }

    /// Nilai berikutnya (0..=1).
    #[inline]
    pub fn berikut(&mut self) -> f32 {
        match self.tahap {
            Tahap::Diam => return 0.0,
            Tahap::Serang => {
                self.nilai += self.langkah_serang;
                if self.nilai >= 1.0 {
                    self.nilai = 1.0;
                    self.tahap = Tahap::Luruh;
                }
            }
            Tahap::Luruh => {
                self.nilai = self.sustain + (self.nilai - self.sustain) * self.k_luruh;
            }
            Tahap::Lepas => self.nilai *= self.k_lepas,
            Tahap::Cekik => self.nilai *= self.k_cekik,
        }
        if self.nilai < AMBANG_DIAM && self.tahap != Tahap::Serang {
            self.tahap = Tahap::Diam;
            self.nilai = 0.0;
        }
        self.nilai
    }
}

/// Koefisien filter state-variable TPT (topology-preserving transform).
///
/// Bentuk ini stabil walaupun cutoff diubah cepat, jadi aman untuk kenop yang
/// diputar saat suara berbunyi (rencana §9).
#[derive(Clone, Copy, Debug)]
pub struct SvfKoef {
    mode: FilterMode,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl Default for SvfKoef {
    fn default() -> Self {
        Self::dari_hz(FilterMode::Off, 1_000.0, 0.0, 48_000.0)
    }
}

impl SvfKoef {
    /// Koefisien dari frekuensi dalam Hz. `resonansi` 0..1 (1 hampir berosilasi).
    #[must_use]
    pub fn dari_hz(mode: FilterMode, hz: f32, resonansi: f32, sr: f32) -> Self {
        let hz = hz.clamp(10.0, sr * 0.45);
        let g = (PI * hz / sr).tan();
        let k = 2.0 - 1.95 * resonansi.clamp(0.0, 1.0);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        Self {
            mode,
            a1,
            a2,
            a3,
            k,
        }
    }

    /// Koefisien dari `cutoff` 0..1, dipetakan eksponensial 20 Hz … 20 kHz.
    #[must_use]
    pub fn dari_cutoff(mode: FilterMode, cutoff: f32, resonansi: f32, sr: f32) -> Self {
        Self::dari_hz(mode, cutoff_ke_hz(cutoff), resonansi, sr)
    }
}

/// Cutoff 0..1 → Hz (20 Hz … 20 kHz, eksponensial seperti telinga).
#[must_use]
pub fn cutoff_ke_hz(cutoff: f32) -> f32 {
    20.0 * 1000.0_f32.powf(cutoff.clamp(0.0, 1.0))
}

/// Keadaan satu kanal filter.
#[derive(Clone, Copy, Debug, Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
}

impl Svf {
    /// Kosongkan keadaan (suara baru).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Proses satu sampel.
    #[inline]
    pub fn proses(&mut self, k: &SvfKoef, x: f32) -> f32 {
        if k.mode == FilterMode::Off {
            return x;
        }
        let v3 = x - self.ic2;
        let v1 = k.a1 * self.ic1 + k.a2 * v3;
        let v2 = self.ic2 + k.a2 * self.ic1 + k.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        match k.mode {
            FilterMode::Off => x,
            FilterMode::LowPass => v2,
            FilterMode::BandPass => v1,
            FilterMode::HighPass => x - k.k * v1 - v2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Adsr, Svf, SvfKoef, hermite};
    use crate::pad::{Envelope, FilterMode};

    const SR: f32 = 48_000.0;

    #[test]
    fn hermite_melewati_titik_dan_garis_lurus() {
        assert!((hermite(0.0, 1.0, 2.0, 3.0, 0.0) - 1.0).abs() < 1e-6);
        assert!((hermite(0.0, 1.0, 2.0, 3.0, 1.0) - 2.0).abs() < 1e-6);
        assert!((hermite(0.0, 1.0, 2.0, 3.0, 0.25) - 1.25).abs() < 1e-6);
    }

    fn amplop(attack: f32, decay: f32, sustain: f32, release: f32) -> Adsr {
        let mut a = Adsr::default();
        a.atur(
            &Envelope {
                attack,
                decay,
                sustain,
                release,
            },
            SR,
        );
        a.mulai();
        a
    }

    #[test]
    fn adsr_naik_tanpa_lompatan_lalu_tahan_di_sustain() {
        let mut a = amplop(0.01, 0.05, 0.5, 0.1);
        let mut sebelum = 0.0_f32;
        for _ in 0..480 {
            let v = a.berikut();
            assert!((v - sebelum).abs() < 0.0025, "lompatan {sebelum} → {v}");
            sebelum = v;
        }
        for _ in 0..48_000 {
            a.berikut();
        }
        assert!((a.berikut() - 0.5).abs() < 1e-3);
    }

    #[test]
    fn adsr_release_selesai_dan_diam() {
        let mut a = amplop(0.001, 0.0, 1.0, 0.1);
        for _ in 0..1_000 {
            a.berikut();
        }
        a.lepas();
        // Turun 60 dB dalam 0,1 s; diam di −80 dB, jadi ±0,13 s.
        for _ in 0..7_000 {
            a.berikut();
        }
        assert!(!a.aktif());
        assert!(a.berikut().abs() < f32::EPSILON);
    }

    #[test]
    fn adsr_sustain_nol_selesai_sendiri() {
        let mut a = amplop(0.001, 0.2, 0.0, 0.1);
        for _ in 0..20_000 {
            a.berikut();
        }
        assert!(!a.aktif());
    }

    #[test]
    fn cekik_lebih_cepat_daripada_release() {
        let mut a = amplop(0.001, 0.0, 1.0, 2.0);
        for _ in 0..100 {
            a.berikut();
        }
        a.cekik();
        for _ in 0..480 {
            a.berikut();
        }
        assert!(!a.aktif(), "choke harus selesai dalam 10 ms");
    }

    fn rms_sinus(k: &SvfKoef, hz: f32) -> f32 {
        let mut f = Svf::default();
        let mut jumlah = 0.0;
        for i in 0..9_600 {
            #[allow(clippy::cast_precision_loss)]
            let x = (core::f32::consts::TAU * hz * i as f32 / SR).sin();
            let y = f.proses(k, x);
            if i >= 4_800 {
                jumlah += y * y;
            }
        }
        (jumlah / 4_800.0).sqrt()
    }

    #[test]
    fn lowpass_meredam_nada_tinggi_highpass_sebaliknya() {
        let lp = SvfKoef::dari_hz(FilterMode::LowPass, 500.0, 0.0, SR);
        let hp = SvfKoef::dari_hz(FilterMode::HighPass, 500.0, 0.0, SR);
        assert!(rms_sinus(&lp, 100.0) > 0.6 && rms_sinus(&lp, 8_000.0) < 0.05);
        assert!(rms_sinus(&hp, 8_000.0) > 0.6 && rms_sinus(&hp, 100.0) < 0.05);
    }

    #[test]
    fn bandpass_lolos_di_tengah() {
        let bp = SvfKoef::dari_hz(FilterMode::BandPass, 1_000.0, 0.5, SR);
        let tengah = rms_sinus(&bp, 1_000.0);
        assert!(tengah > 4.0 * rms_sinus(&bp, 100.0), "{tengah}");
        assert!(tengah > 4.0 * rms_sinus(&bp, 10_000.0), "{tengah}");
    }
}
