//! Kit pabrik: 16 suara yang **disintesis** oleh kode ini, tanpa rekaman dari
//! pihak lain (ADR-0009 butir 6, ADR-0010: CC0).
//!
//! Setiap suara dihitung sekali saat mesin dibuat (di thread UI), lalu
//! disimpan sebagai [`Sampel`] biasa. Jadi kit pabrik dan sampel impor
//! dimainkan oleh jalur yang sama. Hasilnya deterministik: angka acak berasal
//! dari generator xorshift dengan benih tetap.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use core::f32::consts::TAU;

use crate::dsp::{Svf, SvfKoef};
use crate::pad::{Envelope, FilterMode, PAD_COUNT, PadParams};
use crate::sampel::Sampel;

/// Sample rate kit pabrik.
pub const KIT_SAMPLE_RATE: u32 = 48_000;
const SR: f32 = 48_000.0;

/// Nama ke-16 suara kit pabrik, urut pad 1..16.
pub const FACTORY_NAMES: [&str; PAD_COUNT] = [
    "KICK", "SNARE", "CLAP", "HAT-T", "HAT-B", "RIM", "TOM-R", "TOM-S", "TOM-T", "COWBELL",
    "SHAKER", "CRASH", "SUB", "BAS", "BLIP", "ZAP",
];

/// Satu suara kit: nama pendek (untuk layar segmen), sampel, parameter awal.
#[derive(Debug, Clone)]
pub struct KitSound {
    /// Nama pendek, huruf besar, ≤ 8 karakter.
    pub name: &'static str,
    /// Audio mono 48 kHz.
    pub sample: Sampel,
    /// Parameter awal pad.
    pub params: PadParams,
}

/// Generator acak xorshift32: cukup untuk bising, dan selalu sama.
struct Acak(u32);

impl Acak {
    fn bising(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Isi `detik` audio dengan `f(t, n)`; `t` dalam detik, `n` nomor sampel.
fn sintesis(detik: f32, mut f: impl FnMut(f32, usize) -> f32) -> Vec<f32> {
    let n = (detik * SR) as usize;
    (0..n).map(|i| f(i as f32 / SR, i)).collect()
}

/// Rampa masuk 1 ms, pudar keluar 10 ms, lalu skala ke `puncak`.
fn rapikan(mut x: Vec<f32>, puncak: f32) -> Vec<f32> {
    let masuk = (0.001 * SR) as usize;
    let keluar = (0.01 * SR) as usize;
    for (i, v) in x.iter_mut().take(masuk).enumerate() {
        *v *= i as f32 / masuk as f32;
    }
    for (i, v) in x.iter_mut().rev().take(keluar).enumerate() {
        *v *= i as f32 / keluar as f32;
    }
    let maks = x.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
    if maks > 0.0 {
        let skala = puncak / maks;
        for v in &mut x {
            *v *= skala;
        }
    }
    x
}

/// Osilator sinus dengan frekuensi yang berubah tiap sampel.
struct Fase(f32);

impl Fase {
    fn sinus(&mut self, hz: f32) -> f32 {
        let y = self.0.sin();
        self.0 = (self.0 + TAU * hz / SR) % TAU;
        y
    }
}

/// Filter satu kanal dengan koefisien tetap.
fn saring(x: &mut [f32], mode: FilterMode, hz: f32, res: f32) {
    let k = SvfKoef::dari_hz(mode, hz, res, SR);
    let mut f = Svf::default();
    for v in x {
        *v = f.proses(&k, *v);
    }
}

/// Logam: enam gelombang kotak tak harmonis dijumlahkan (dasar hi-hat,
/// crash, cowbell).
fn logam(t: f32) -> f32 {
    const FREKUENSI: [f32; 6] = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];
    FREKUENSI
        .iter()
        .map(|&hz| {
            if (t * hz * 7.3).fract() < 0.5 {
                1.0
            } else {
                -1.0
            }
        })
        .sum::<f32>()
        / 6.0
}

fn kick() -> Vec<f32> {
    let mut o = Fase(0.0);
    sintesis(1.2, |t, _| {
        let hz = 48.0 + 102.0 * (-t / 0.035).exp();
        o.sinus(hz) * (-t / 0.18).exp()
    })
}

fn snare() -> Vec<f32> {
    let (mut a, mut b) = (Fase(0.0), Fase(0.0));
    let mut acak = Acak(0x5EED_0001);
    let mut bising = sintesis(0.5, |_, _| acak.bising());
    saring(&mut bising, FilterMode::HighPass, 1_500.0, 0.1);
    sintesis(0.5, |t, i| {
        let nada = a.sinus(185.0) * (-t / 0.05).exp() + 0.5 * b.sinus(330.0) * (-t / 0.03).exp();
        0.6 * nada + 0.9 * bising[i] * (-t / 0.12).exp()
    })
}

fn clap() -> Vec<f32> {
    let mut acak = Acak(0x5EED_0002);
    let mut x = sintesis(0.6, |t, _| {
        let ledakan = [0.0, 0.011, 0.023]
            .iter()
            .filter(|&&m| t >= m)
            .map(|&m| (-(t - m) / 0.006).exp())
            .sum::<f32>();
        let ekor = if t >= 0.03 {
            0.6 * (-(t - 0.03) / 0.12).exp()
        } else {
            0.0
        };
        acak.bising() * (ledakan + ekor)
    });
    saring(&mut x, FilterMode::BandPass, 1_200.0, 0.3);
    x
}

fn hat(detik: f32, tau: f32) -> Vec<f32> {
    let mut acak = Acak(0x5EED_0003);
    let mut x = sintesis(detik, |t, _| {
        (0.7 * logam(t) + 0.3 * acak.bising()) * (-t / tau).exp()
    });
    saring(&mut x, FilterMode::HighPass, 7_000.0, 0.2);
    x
}

fn rim() -> Vec<f32> {
    let mut o = Fase(0.0);
    let mut acak = Acak(0x5EED_0004);
    let mut x = sintesis(0.12, |t, _| {
        o.sinus(1_700.0) * (-t / 0.008).exp() + 0.4 * acak.bising() * (-t / 0.004).exp()
    });
    saring(&mut x, FilterMode::HighPass, 400.0, 0.0);
    x
}

fn tom(dasar: f32) -> Vec<f32> {
    let mut o = Fase(0.0);
    let mut acak = Acak(0x5EED_0005);
    sintesis(0.8, |t, _| {
        let hz = dasar * (1.0 + 0.5 * (-t / 0.03).exp());
        o.sinus(hz) * (-t / 0.25).exp() + 0.15 * acak.bising() * (-t / 0.005).exp()
    })
}

fn cowbell() -> Vec<f32> {
    let mut x = sintesis(0.5, |t, _| {
        let a = if (t * 540.0).fract() < 0.5 { 1.0 } else { -1.0 };
        let b = if (t * 800.0).fract() < 0.5 { 1.0 } else { -1.0 };
        (a + b) * (0.6 * (-t / 0.02).exp() + 0.4 * (-t / 0.15).exp())
    });
    saring(&mut x, FilterMode::BandPass, 900.0, 0.5);
    x
}

fn shaker() -> Vec<f32> {
    let mut acak = Acak(0x5EED_0006);
    let mut x = sintesis(0.25, |t, _| {
        acak.bising() * (t / 0.015).min(1.0) * (-t / 0.06).exp()
    });
    saring(&mut x, FilterMode::HighPass, 5_000.0, 0.1);
    x
}

fn crash() -> Vec<f32> {
    let mut acak = Acak(0x5EED_0007);
    let mut x = sintesis(2.0, |t, _| {
        (0.5 * logam(t * 1.4) + 0.5 * acak.bising()) * (-t / 0.7).exp()
    });
    saring(&mut x, FilterMode::HighPass, 4_000.0, 0.1);
    x
}

fn sub() -> Vec<f32> {
    let mut o = Fase(0.0);
    sintesis(2.0, |t, _| {
        let hz = 45.0 + 60.0 * (-t / 0.02).exp();
        (2.0 * o.sinus(hz)).tanh() * (-t / 0.6).exp()
    })
}

fn bas() -> Vec<f32> {
    let mut f = Svf::default();
    let mut k = SvfKoef::default();
    sintesis(1.0, |t, i| {
        if i % 32 == 0 {
            let hz = 200.0 + 1_800.0 * (-t / 0.08).exp();
            k = SvfKoef::dari_hz(FilterMode::LowPass, hz, 0.4, SR);
        }
        let gergaji = 2.0 * (t * 55.0).fract() - 1.0;
        f.proses(&k, gergaji) * (-t / 0.35).exp()
    })
}

fn blip() -> Vec<f32> {
    let (mut pembawa, mut modulator) = (0.0_f32, 0.0_f32);
    sintesis(0.5, |t, _| {
        let indeks = 3.0 * (-t / 0.05).exp();
        let y = (pembawa + indeks * modulator.sin()).sin();
        pembawa = (pembawa + TAU * 880.0 / SR) % TAU;
        modulator = (modulator + TAU * 1_320.0 / SR) % TAU;
        y * (-t / 0.15).exp()
    })
}

fn zap() -> Vec<f32> {
    let mut o = Fase(0.0);
    sintesis(0.4, |t, _| {
        let hz = 80.0 + 2_000.0 * (-t / 0.02).exp();
        o.sinus(hz) * (-t / 0.1).exp()
    })
}

/// Bangun 16 suara kit pabrik. Memakan beberapa milidetik; panggil di luar
/// thread audio.
#[must_use]
pub fn factory_kit() -> Vec<KitSound> {
    // Choke group 1: hi-hat tertutup memotong hi-hat terbuka.
    let hat_choke = PadParams {
        choke: 1,
        ..PadParams::default()
    };
    let bersih = PadParams::default();
    let daftar: [(&str, Vec<f32>, f32, PadParams); PAD_COUNT] = [
        ("KICK", kick(), 0.95, bersih),
        ("SNARE", snare(), 0.8, bersih),
        ("CLAP", clap(), 0.75, bersih),
        ("HAT-T", hat(0.25, 0.035), 0.5, hat_choke),
        ("HAT-B", hat(1.0, 0.3), 0.45, hat_choke),
        ("RIM", rim(), 0.6, bersih),
        ("TOM-R", tom(90.0), 0.85, bersih),
        ("TOM-S", tom(130.0), 0.8, bersih),
        ("TOM-T", tom(180.0), 0.75, bersih),
        ("COWBELL", cowbell(), 0.55, bersih),
        ("SHAKER", shaker(), 0.45, bersih),
        ("CRASH", crash(), 0.5, bersih),
        ("SUB", sub(), 0.9, bersih),
        (
            "BAS",
            bas(),
            0.8,
            PadParams {
                play_mode: crate::pad::PlayMode::Gate,
                envelope: Envelope {
                    release: 0.08,
                    ..Envelope::default()
                },
                ..PadParams::default()
            },
        ),
        ("BLIP", blip(), 0.6, bersih),
        ("ZAP", zap(), 0.6, bersih),
    ];
    daftar
        .into_iter()
        .map(|(name, audio, puncak, params)| {
            let sample = Sampel::from_f32(&rapikan(audio, puncak), 1, KIT_SAMPLE_RATE);
            let end = u32::try_from(sample.frames()).unwrap_or(u32::MAX);
            KitSound {
                name,
                sample,
                params: PadParams { end, ..params },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{KIT_SAMPLE_RATE, factory_kit};
    use crate::pad::PAD_COUNT;

    #[test]
    fn enam_belas_suara_sah() {
        let kit = factory_kit();
        assert_eq!(kit.len(), PAD_COUNT);
        for s in &kit {
            let n = s.sample.frames();
            assert!(!s.name.is_empty() && s.name.len() <= 8, "{}", s.name);
            assert!(n > 1_000 && n <= 2 * KIT_SAMPLE_RATE as usize, "{}", s.name);
            assert_eq!(s.params.end as usize, n);
            let puncak = (0..n)
                .map(|i| s.sample.frame(i).0.abs())
                .fold(0.0, f32::max);
            assert!(puncak > 0.4 && puncak <= 1.0, "{}: {puncak}", s.name);
            // Mulai dan berakhir hampir hening: tanpa klik.
            assert!(s.sample.frame(0).0.abs() < 0.01, "{} awal", s.name);
            assert!(s.sample.frame(n - 1).0.abs() < 0.01, "{} akhir", s.name);
        }
    }

    #[test]
    fn nama_unik_dan_sama_dengan_daftar() {
        let kit = factory_kit();
        for (s, nama) in kit.iter().zip(super::FACTORY_NAMES) {
            assert_eq!(s.name, nama);
        }
        for (i, a) in kit.iter().enumerate() {
            assert!(kit[i + 1..].iter().all(|b| b.name != a.name), "{}", a.name);
        }
    }

    #[test]
    fn deterministik() {
        let (a, b) = (factory_kit(), factory_kit());
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(x.sample, y.sample, "{}", x.name);
        }
    }

    #[test]
    fn hat_berbagi_choke_group() {
        let kit = factory_kit();
        assert_eq!(kit[3].params.choke, 1);
        assert_eq!(kit[4].params.choke, 1);
        assert_eq!(kit.iter().filter(|s| s.params.choke != 0).count(), 2);
    }
}
