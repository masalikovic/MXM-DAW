#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use super::{EVENT_CAPACITY, Engine, EventSender, MAX_VOICES};
use crate::pad::{FilterMode, PadParams, PlayMode};
use crate::sampel::Sampel;

const SR: u32 = 48_000;
const KICK: usize = 0;
const HAT_T: usize = 3;
const HAT_B: usize = 4;

fn mesin() -> (Engine, EventSender) {
    Engine::new(SR)
}

fn render(engine: &mut Engine, detik: f32, channels: usize) -> Vec<f32> {
    let frames = (detik * 48_000.0) as usize;
    let mut buf = vec![0.0; frames * channels];
    for blok in buf.chunks_mut(192 * channels) {
        engine.render(blok, channels);
    }
    buf
}

fn puncak(x: &[f32]) -> f32 {
    x.iter().fold(0.0_f32, |m, v| m.max(v.abs()))
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

fn kiri(x: &[f32]) -> Vec<f32> {
    x.chunks_exact(2).map(|f| f[0]).collect()
}

/// Sinus 1 detik, untuk tes yang butuh sampel yang bisa ditebak.
fn sinus(hz: f32) -> Sampel {
    let x: Vec<f32> = (0..48_000)
        .map(|i| 0.5 * (core::f32::consts::TAU * hz * i as f32 / 48_000.0).sin())
        .collect();
    Sampel::from_f32(&x, 1, 48_000)
}

#[test]
fn tanpa_ketukan_hening() {
    let (mut m, _k) = mesin();
    assert!(render(&mut m, 0.5, 2).iter().all(|&v| v == 0.0));
}

#[test]
fn setiap_pad_kit_berbunyi() {
    for pad in 0..16 {
        let (mut m, mut k) = mesin();
        assert!(k.hit(pad, 1.0));
        let x = render(&mut m, 0.2, 2);
        assert!(puncak(&x) > 0.1, "pad {pad} ({:?})", k.name(pad));
        assert!(x.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
    }
}

#[test]
fn kick_tanpa_klik_di_awal() {
    let (mut m, mut k) = mesin();
    k.hit(KICK, 1.0);
    let x = kiri(&render(&mut m, 0.05, 2));
    let awal = puncak(&x[..10]);
    assert!(awal < 0.05, "10 sampel pertama memuncak di {awal}");
    assert!(puncak(&x) > 0.5);
}

#[test]
fn pad_di_luar_jangkauan_ditolak() {
    let (_m, mut k) = mesin();
    assert!(!k.hit(16, 1.0));
    assert!(!k.release(99));
    assert!(!k.set_pad(16, &PadParams::default()));
}

#[test]
fn velocity_mengatur_kekerasan() {
    let puncak_dengan = |v: f32| {
        let (mut m, mut k) = mesin();
        k.hit(KICK, v);
        puncak(&render(&mut m, 0.05, 2))
    };
    let rasio = puncak_dengan(0.5) / puncak_dengan(1.0);
    assert!((rasio - 0.5).abs() < 0.01, "rasio {rasio}");
}

#[test]
fn hat_tertutup_memotong_hat_terbuka() {
    let energi_setelah = |potong: bool| {
        let (mut m, mut k) = mesin();
        k.hit(HAT_B, 1.0);
        render(&mut m, 0.1, 2);
        if potong {
            k.hit(HAT_T, 0.0); // velocity 0: hanya memotong, tidak terdengar
        }
        rms(&render(&mut m, 0.1, 2)[4_800..])
    };
    let (tanpa, dengan) = (energi_setelah(false), energi_setelah(true));
    assert!(tanpa > 0.01, "hat terbuka masih berbunyi: {tanpa}");
    assert!(dengan < tanpa / 100.0, "{tanpa} → {dengan}");
}

#[test]
fn mode_gate_berhenti_saat_dilepas() {
    let (mut m, mut k) = mesin();
    k.set_sample(0, "SINUS", sinus(440.0));
    let p = PadParams {
        play_mode: PlayMode::Gate,
        ..*k.pad(0).unwrap()
    };
    k.set_pad(0, &p);
    k.hit(0, 1.0);
    assert!(rms(&render(&mut m, 0.1, 2)) > 0.1);
    k.release(0);
    render(&mut m, 0.1, 2); // release bawaan 20 ms
    assert!(puncak(&render(&mut m, 0.05, 2)) < 1e-3);
}

#[test]
fn one_shot_tidak_peduli_dilepas() {
    let (mut m, mut k) = mesin();
    k.set_sample(0, "SINUS", sinus(440.0));
    k.hit(0, 1.0);
    render(&mut m, 0.05, 2);
    k.release(0);
    assert!(rms(&render(&mut m, 0.2, 2)) > 0.1);
}

#[test]
fn mode_loop_mengulang_melewati_akhir_sampel() {
    let (mut m, mut k) = mesin();
    k.set_sample(0, "SINUS", sinus(440.0));
    let p = PadParams {
        play_mode: PlayMode::Loop,
        start: 0,
        end: 4_800, // 100 ms
        ..*k.pad(0).unwrap()
    };
    k.set_pad(0, &p);
    k.hit(0, 1.0);
    let x = render(&mut m, 0.5, 2);
    assert!(
        rms(&x[2 * 20_000..]) > 0.1,
        "masih berbunyi setelah 4 putaran"
    );
    k.release(0);
    render(&mut m, 0.2, 2);
    assert!(puncak(&render(&mut m, 0.05, 2)) < 1e-3);
}

#[test]
fn start_end_membatasi_panjang() {
    let (mut m, mut k) = mesin();
    k.set_sample(0, "SINUS", sinus(440.0));
    let p = PadParams {
        start: 12_000,
        end: 24_000, // 250 ms
        ..*k.pad(0).unwrap()
    };
    k.set_pad(0, &p);
    k.hit(0, 1.0);
    let x = kiri(&render(&mut m, 0.4, 2));
    assert!(rms(&x[..9_600]) > 0.1);
    assert!(puncak(&x[12_480..]) < 1e-3, "berhenti setelah 250 ms");
}

#[test]
fn pitch_satu_oktaf_menggandakan_kecepatan() {
    let panjang_bunyi = |pitch: f32| {
        let (mut m, mut k) = mesin();
        k.set_sample(0, "SINUS", sinus(440.0));
        let p = PadParams {
            pitch,
            ..*k.pad(0).unwrap()
        };
        k.set_pad(0, &p);
        k.hit(0, 1.0);
        let x = kiri(&render(&mut m, 1.2, 2));
        x.iter().rposition(|v| v.abs() > 1e-3).unwrap()
    };
    let (normal, oktaf) = (panjang_bunyi(0.0), panjang_bunyi(12.0));
    assert!(normal.abs_diff(48_000) < 100, "{normal}");
    assert!(oktaf.abs_diff(24_000) < 100, "{oktaf}");
}

#[test]
fn pan_kiri_membungkam_kanan() {
    let (mut m, mut k) = mesin();
    let p = PadParams {
        pan: -1.0,
        ..*k.pad(KICK).unwrap()
    };
    k.set_pad(KICK, &p);
    k.hit(KICK, 1.0);
    let x = render(&mut m, 0.1, 2);
    let kanan: Vec<f32> = x.chunks_exact(2).map(|f| f[1]).collect();
    assert!(puncak(&kiri(&x)) > 0.5 && puncak(&kanan) < 1e-6);
}

#[test]
fn lowpass_meredam_hat() {
    let energi = |cutoff: f32| {
        let (mut m, mut k) = mesin();
        let mut p = *k.pad(HAT_B).unwrap();
        p.filter.mode = FilterMode::LowPass;
        p.filter.cutoff = cutoff;
        k.set_pad(HAT_B, &p);
        k.hit(HAT_B, 1.0);
        rms(&render(&mut m, 0.2, 2))
    };
    assert!(energi(0.3) < energi(1.0) / 10.0);
}

#[test]
fn parameter_berubah_saat_berbunyi() {
    let (mut m, mut k) = mesin();
    k.set_sample(0, "SINUS", sinus(440.0));
    k.hit(0, 1.0);
    let sebelum = rms(&render(&mut m, 0.1, 2));
    let p = PadParams {
        level: 0.25,
        ..*k.pad(0).unwrap()
    };
    k.set_pad(0, &p);
    let sesudah = rms(&render(&mut m, 0.1, 2));
    assert!(
        (sesudah / sebelum - 0.25).abs() < 0.02,
        "{sebelum} → {sesudah}"
    );
}

#[test]
fn mencuri_suara_tanpa_klik() {
    let (mut m, mut k) = mesin();
    k.set_sample(0, "SINUS", sinus(110.0));
    let p = PadParams {
        play_mode: PlayMode::Loop,
        level: 0.05,
        ..*k.pad(0).unwrap()
    };
    k.set_pad(0, &p);
    for _ in 0..MAX_VOICES {
        k.hit(0, 1.0);
        render(&mut m, 0.003, 2);
    }
    k.hit(0, 1.0); // suara ke-33: mencuri yang tertua
    let x = kiri(&render(&mut m, 0.01, 2));
    let lompatan = x
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0, f32::max);
    assert!(lompatan < 0.05, "lompatan {lompatan}");
    assert_eq!(m.stats().voices_active() as usize, MAX_VOICES);
}

#[test]
fn ketukan_beruntun_aman() {
    let (mut m, mut k) = mesin();
    let mut blok = vec![0.0; 192 * 2];
    for i in 0..200 {
        k.hit(i % 16, 1.0);
        m.render(&mut blok, 2);
        assert!(blok.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
    }
    assert_eq!(m.stats().hits_played(), 200);
    assert!(m.stats().voices_active() as usize <= MAX_VOICES);
}

#[test]
fn led_menunjukkan_pad_yang_berbunyi() {
    let (mut m, mut k) = mesin();
    k.hit(KICK, 1.0);
    k.hit(5, 1.0);
    render(&mut m, 0.01, 2);
    assert_eq!(m.stats().pads_sounding(), (1 << KICK) | (1 << 5));
    render(&mut m, 2.5, 2);
    assert_eq!(m.stats().pads_sounding(), 0);
}

#[test]
fn ganti_sampel_mengembalikan_yang_lama_ke_ui() {
    let (mut m, mut k) = mesin();
    let lama = std::sync::Arc::downgrade(k.sample(0).unwrap());
    assert!(k.set_sample(0, "SINUS", sinus(220.0)));
    assert_eq!(k.name(0), Some("SINUS"));
    assert_eq!(k.pad(0).unwrap().end, 48_000);
    render(&mut m, 0.01, 2);
    assert!(
        lama.upgrade().is_some(),
        "belum dibebaskan sebelum UI membuang"
    );
    assert_eq!(k.empty_trash(), 1);
    assert!(lama.upgrade().is_none(), "dibebaskan di thread UI");
}

#[test]
fn mono_dan_stereo() {
    let (mut m, mut k) = mesin();
    k.hit(KICK, 1.0);
    let mono = render(&mut m, 0.05, 1);
    assert!(puncak(&mono) > 0.5);
    let (mut m, mut k) = mesin();
    k.hit(KICK, 1.0);
    let x = render(&mut m, 0.05, 2);
    for f in x.chunks_exact(2) {
        assert_eq!(f[0].to_bits(), f[1].to_bits(), "pan tengah: kiri = kanan");
    }
}

#[test]
fn deterministik() {
    let jalankan = || {
        let (mut m, mut k) = mesin();
        for pad in 0..16 {
            k.hit(pad, 0.8);
        }
        render(&mut m, 0.3, 2)
    };
    let (a, b) = (jalankan(), jalankan());
    assert!(a.iter().zip(&b).all(|(x, y)| x.to_bits() == y.to_bits()));
}

#[test]
fn antrean_penuh_dibuang_dan_dihitung() {
    let (_m, mut k) = mesin();
    for _ in 0..EVENT_CAPACITY {
        assert!(k.hit(KICK, 1.0));
    }
    assert!(!k.hit(KICK, 1.0));
    assert_eq!(k.stats().hits_dropped(), 1);
}
