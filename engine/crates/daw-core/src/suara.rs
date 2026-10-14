//! Satu suara sampler: membaca sampel dengan pitch (Hermite), lalu amplop,
//! filter, drive, dan pan (rencana §9). Dipakai hanya oleh thread audio.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use core::f32::consts::FRAC_PI_4;

use crate::dsp::{AMBANG_DIAM, Adsr, RAMPA_MIN, Svf, SvfKoef, hermite};
use crate::pad::{PadParams, PlayMode};
use crate::sampel::Sampel;

/// Waktu peluruhan "ekor" saat suara dicuri atau dipotong (detik).
const WAKTU_EKOR: f32 = 0.002;

/// Suara sampler. Tidak memegang sampel; sampel dibaca dari slot pad setiap
/// blok, jadi mengganti sampel pad tidak pernah membebaskan memori di thread
/// audio (ADR-0014).
#[derive(Clone, Copy, Debug, Default)]
pub struct Suara {
    pad: u8,
    aktif: bool,
    umur: u32,
    /// Posisi baca (frame sampel, pecahan untuk interpolasi).
    pos: f64,
    langkah: f64,
    awal: f64,
    akhir: f64,
    mode: PlayMode,
    dilepas: bool,
    velocity: f32,
    adsr: Adsr,
    koef: SvfKoef,
    filter_l: Svf,
    filter_r: Svf,
    gain_l: f32,
    gain_r: f32,
    drive: f32,
    drive_norm: f32,
    inv_rampa_akhir: f32,
    /// Keluaran terakhir; menjadi ekor bila suara dicuri.
    terakhir: (f32, f32),
    /// Sisa bunyi suara sebelumnya yang dipudarkan cepat (tanpa klik).
    ekor: (f32, f32),
    k_ekor: f32,
}

impl Suara {
    /// Pad yang sedang dimainkan suara ini.
    #[must_use]
    pub fn pad(&self) -> usize {
        usize::from(self.pad)
    }

    /// Masih berbunyi (tidak termasuk ekor).
    #[must_use]
    pub fn aktif(&self) -> bool {
        self.aktif
    }

    /// Umur dalam sampel; suara tertua dicuri lebih dulu.
    #[must_use]
    pub fn umur(&self) -> u32 {
        self.umur
    }

    /// Ada yang masih terdengar (suara atau ekornya).
    #[must_use]
    pub fn terdengar(&self) -> bool {
        self.aktif || self.ekor.0.abs() > AMBANG_DIAM || self.ekor.1.abs() > AMBANG_DIAM
    }

    /// Mulai memainkan `pad`. Bila suara ini sedang berbunyi (dicuri), bunyi
    /// lamanya dipudarkan dalam ±2 ms, bukan diputus.
    pub fn picu(&mut self, pad: usize, velocity: f32, p: &PadParams, sampel: &Sampel, sr: f32) {
        if self.aktif {
            self.ekor.0 += self.terakhir.0;
            self.ekor.1 += self.terakhir.1;
        }
        self.pad = u8::try_from(pad).unwrap_or(0);
        self.aktif = sampel.frames() > 1;
        self.umur = 0;
        self.dilepas = false;
        self.velocity = velocity.clamp(0.0, 1.0);
        self.filter_l.reset();
        self.filter_r.reset();
        self.terakhir = (0.0, 0.0);
        self.siapkan(p, sampel, sr);
        self.pos = self.awal;
        self.adsr.mulai();
    }

    /// Hitung ulang koefisien dari parameter pad. Dipanggil setiap blok,
    /// sehingga kenop yang diputar langsung terdengar.
    pub fn siapkan(&mut self, p: &PadParams, sampel: &Sampel, sr: f32) {
        let n = sampel.frames() as f64;
        self.akhir = f64::from(p.end).min(n);
        self.awal = f64::from(p.start).min((self.akhir - 1.0).max(0.0));
        self.mode = p.play_mode;
        let rasio = f64::from(sampel.sample_rate()) / f64::from(sr);
        self.langkah = 2.0_f64.powf(f64::from(p.pitch) / 12.0) * rasio;
        self.adsr.atur(&p.envelope, sr);
        self.koef = SvfKoef::dari_cutoff(p.filter.mode, p.filter.cutoff, p.filter.resonance, sr);

        let gain = self.velocity * p.level;
        if sampel.channels() == 1 {
            // Pan daya-konstan; di tengah gain 1 di kedua kanal.
            let sudut = (p.pan + 1.0) * FRAC_PI_4;
            self.gain_l = gain * sudut.cos() * core::f32::consts::SQRT_2;
            self.gain_r = gain * sudut.sin() * core::f32::consts::SQRT_2;
        } else {
            // Sampel stereo: balance.
            self.gain_l = gain * (1.0 - p.pan).min(1.0);
            self.gain_r = gain * (1.0 + p.pan).min(1.0);
        }
        self.drive = 1.0 + 9.0 * p.drive;
        self.drive_norm = if p.drive > 0.0 {
            1.0 / self.drive.tanh()
        } else {
            1.0
        };
        self.inv_rampa_akhir = 1.0 / (RAMPA_MIN * sr);
        self.k_ekor = (-1.0 / (WAKTU_EKOR * sr)).exp();
    }

    /// Pad dilepas: gate/loop masuk release; one-shot tidak terpengaruh.
    pub fn lepas(&mut self) {
        self.dilepas = true;
        if self.mode != PlayMode::OneShot {
            self.adsr.lepas();
        }
    }

    /// Dipotong choke group (release 5 ms).
    pub fn cekik(&mut self) {
        self.adsr.cekik();
    }

    /// Berhenti sekarang; sisa bunyi dipudarkan lewat ekor.
    pub fn potong(&mut self) {
        if self.aktif {
            self.ekor.0 += self.terakhir.0;
            self.ekor.1 += self.terakhir.1;
            self.aktif = false;
        }
    }

    /// Tambahkan bunyi suara ini ke `campur` (stereo interleaved).
    pub fn tambah_ke(&mut self, sampel: &Sampel, campur: &mut [f32]) {
        for frame in campur.chunks_exact_mut(2) {
            let (mut l, mut r) = self.ekor;
            self.ekor.0 *= self.k_ekor;
            self.ekor.1 *= self.k_ekor;
            if self.aktif {
                let (sl, sr) = self.sampel_berikut(sampel);
                l += sl;
                r += sr;
            }
            frame[0] += l;
            frame[1] += r;
        }
        if !self.terdengar() {
            self.ekor = (0.0, 0.0);
        }
    }

    #[inline]
    #[allow(clippy::many_single_char_names)]
    fn sampel_berikut(&mut self, sampel: &Sampel) -> (f32, f32) {
        let i = self.pos as usize;
        let t = (self.pos - i as f64) as f32;
        let (am1, bm1) = sampel.frame(i.saturating_sub(1));
        let (a0, b0) = sampel.frame(i);
        let (a1, b1) = sampel.frame(i + 1);
        let (a2, b2) = sampel.frame(i + 2);
        let mut l = hermite(am1, a0, a1, a2, t);
        let mut r = hermite(bm1, b0, b1, b2, t);

        let mut g = self.adsr.berikut();
        if self.mode != PlayMode::Loop || self.dilepas {
            // Pudar 1 ms sebelum titik akhir supaya end di tengah bunyi tidak klik.
            let sisa = ((self.akhir - self.pos) / self.langkah) as f32;
            g *= (sisa * self.inv_rampa_akhir).min(1.0);
        }
        l = self.filter_l.proses(&self.koef, l * g);
        r = self.filter_r.proses(&self.koef, r * g);
        if self.drive > 1.0 {
            l = (l * self.drive).tanh() * self.drive_norm;
            r = (r * self.drive).tanh() * self.drive_norm;
        }
        l *= self.gain_l;
        r *= self.gain_r;

        self.umur = self.umur.saturating_add(1);
        self.pos += self.langkah;
        if self.pos >= self.akhir {
            let panjang = self.akhir - self.awal;
            if self.mode == PlayMode::Loop && !self.dilepas && panjang >= 2.0 {
                self.pos -= panjang;
            } else {
                self.aktif = false;
            }
        }
        if !self.adsr.aktif() {
            self.aktif = false;
        }
        self.terakhir = if self.aktif { (l, r) } else { (0.0, 0.0) };
        (l, r)
    }
}
