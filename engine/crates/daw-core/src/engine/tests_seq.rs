//! Tes sequencer di mesin: timing tepat-sampel, count-in, metronom, rekaman.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use super::{Engine, EventSender};
use crate::pad::PadParams;
use crate::sampel::Sampel;
use crate::seq::{Note, PPQN, Pattern, STEP_TICKS, SeqSettings, TICKS_PER_BAR};

const SR: u32 = 48_000;

/// Mesin dengan pad 0 berisi impuls: satu sampel bernilai 1, sisanya 0.
/// Posisi bunyi = posisi sampel pertama yang tidak nol.
fn mesin_impuls(bpm: f32) -> (Engine, EventSender) {
    let (m, mut k) = Engine::new(SR);
    let mut x = vec![0.0; 64];
    x[0] = 1.0;
    k.set_sample(0, "IMPULS", Sampel::from_f32(&x, 1, 48_000));
    k.set_settings(&SeqSettings {
        bpm,
        metronome: false,
        ..SeqSettings::default()
    });
    (m, k)
}

fn not(tick: u32, pad: u8) -> Note {
    Note {
        tick,
        pad,
        velocity: 1.0,
        length: 60,
    }
}

/// Render mono dengan blok berukuran `blok`.
fn render(m: &mut Engine, frames: usize, blok: usize) -> Vec<f32> {
    let mut x = vec![0.0; frames];
    for b in x.chunks_mut(blok) {
        m.render(b, 1);
    }
    x
}

/// Indeks awal setiap bunyi (sampel tidak nol sesudah sedikitnya 32 nol).
fn awal_bunyi(x: &[f32]) -> Vec<usize> {
    let mut hasil = Vec::new();
    let mut sunyi = 32;
    for (i, &v) in x.iter().enumerate() {
        if v == 0.0 {
            sunyi += 1;
        } else {
            if sunyi >= 32 {
                hasil.push(i);
            }
            sunyi = 0;
        }
    }
    hasil
}

/// Frame yang diharapkan untuk tick absolut: ⌈tick × 60 × sr / (bpm × 960)⌉.
fn frame_harapan(tick: f64, bpm: f64) -> usize {
    (tick * 60.0 * f64::from(SR) / (bpm * f64::from(PPQN))).ceil() as usize
}

#[test]
fn not_jatuh_tepat_di_sampelnya_setelah_100_bar() {
    let bpm = 123.4;
    let (mut m, mut k) = mesin_impuls(bpm as f32);
    k.set_pattern(Pattern {
        bars: 1,
        notes: vec![not(0, 0), not(1_000, 0), not(2_401, 0)],
        ..Pattern::default()
    });
    k.transport(true, false);
    let bar = 100;
    let frames = frame_harapan(f64::from(bar * TICKS_PER_BAR), bpm);
    let awal = awal_bunyi(&render(&mut m, frames, 192));
    assert_eq!(awal.len(), 3 * bar as usize);
    for (i, &f) in awal.iter().enumerate() {
        let putaran = (i / 3) as f64 * f64::from(TICKS_PER_BAR);
        let tick = putaran + [0.0, 1_000.0, 2_401.0][i % 3];
        assert_eq!(f, frame_harapan(tick, bpm), "not ke-{i} (tick {tick})");
    }
}

#[test]
fn ukuran_blok_tidak_mengubah_hasil() {
    let jalankan = |blok| {
        let (mut m, mut k) = Engine::new(SR);
        k.set_pattern(Pattern {
            bars: 2,
            notes: (0..32)
                .map(|i| not(i * STEP_TICKS, (i % 16) as u8))
                .collect(),
            ..Pattern::default()
        });
        k.transport(true, false);
        render(&mut m, 96_000, blok)
    };
    let (a, b) = (jalankan(37), jalankan(256));
    assert!(a.iter().zip(&b).all(|(x, y)| x.to_bits() == y.to_bits()));
    assert!(a.iter().any(|v| v.abs() > 0.1));
}

#[test]
fn swing_menggeser_langkah_kedua() {
    let bpm = 120.0;
    let (mut m, mut k) = mesin_impuls(bpm as f32);
    k.set_settings(&SeqSettings {
        bpm: bpm as f32,
        swing: 0.66,
        metronome: false,
        ..SeqSettings::default()
    });
    k.set_pattern(Pattern {
        bars: 1,
        notes: vec![not(0, 0), not(STEP_TICKS, 0)],
        ..Pattern::default()
    });
    k.transport(true, false);
    let awal = awal_bunyi(&render(&mut m, 24_000, 256));
    let geser = (0.66 - 0.5) * 2.0 * f64::from(STEP_TICKS);
    assert_eq!(awal, [0, frame_harapan(f64::from(STEP_TICKS) + geser, bpm)]);
}

#[test]
fn berhenti_dan_mulai_lagi_dari_awal() {
    let (mut m, mut k) = mesin_impuls(120.0);
    k.set_pattern(Pattern {
        bars: 1,
        notes: vec![not(0, 0)],
        ..Pattern::default()
    });
    k.transport(true, false);
    render(&mut m, 30_000, 256);
    assert!(m.stats().playing());
    k.transport(false, false);
    assert!(render(&mut m, 10_000, 256).iter().all(|&v| v == 0.0));
    assert!(!m.stats().playing());
    assert_eq!(m.stats().position_ticks(), 0);
    k.transport(true, false);
    assert_eq!(awal_bunyi(&render(&mut m, 1_000, 256)), [0]);
}

#[test]
fn count_in_satu_bar_dengan_metronom_lalu_pola() {
    let bpm = 120.0;
    let (mut m, mut k) = mesin_impuls(bpm as f32);
    k.set_settings(&SeqSettings {
        bpm: bpm as f32,
        metronome: true,
        count_in_bars: 1,
        ..SeqSettings::default()
    });
    k.set_pattern(Pattern {
        bars: 1,
        notes: vec![not(0, 0)],
        ..Pattern::default()
    });
    k.transport(true, true);
    // Satu bar count-in (2 s) + sedikit.
    let x = render(&mut m, 97_000, 256);
    assert!(m.stats().recording());
    // Empat klik count-in di 0, 0,5, 1, 1,5 s; pola baru mulai di 2 s
    // (bersamaan dengan klik ketukan pertama bar rekaman).
    let awal = awal_bunyi(&x);
    assert_eq!(awal, [0, 24_000, 48_000, 72_000, 96_000]);
    let puncak = |a: usize| x[a..a + 200].iter().fold(0.0_f32, |p, v| p.max(v.abs()));
    assert!(puncak(0) > puncak(24_000), "klik awal bar lebih keras");
}

#[test]
fn metronom_diam_saat_hanya_bermain() {
    let (mut m, mut k) = Engine::new(SR);
    k.transport(true, false);
    assert!(render(&mut m, 48_000, 256).iter().all(|&v| v == 0.0));
}

#[test]
fn rekaman_masuk_ke_pola_dengan_kompensasi_latensi() {
    let bpm = 120.0; // 1 ms = 1,92 tick
    let (mut m, mut k) = Engine::new(SR);
    k.set_settings(&SeqSettings {
        bpm,
        count_in_bars: 0,
        metronome: false,
        ..SeqSettings::default()
    });
    k.transport(true, true);
    render(&mut m, 24_000, 256); // tepat ketukan 2 (tick 960)
    k.hit(3, 0.7);
    render(&mut m, 12_000, 256); // + 1/8 (480 tick)
    k.release(3);
    render(&mut m, 256, 256);

    assert_eq!(k.poll_recording(25.0), 1);
    let n = k.sequence().pattern.notes[0];
    assert_eq!(n.pad, 3);
    assert!((n.velocity - 0.7).abs() < 1e-6);
    // 960 − 25 ms × 1,92 = 912.
    assert_eq!(n.tick, 912);
    assert_eq!(n.length, 480);

    // Not hasil rekaman dimainkan pada putaran berikutnya.
    let sebelum = m.stats().hits_played();
    render(&mut m, 96_000, 256);
    assert!(m.stats().hits_played() > sebelum);
}

#[test]
fn rekaman_di_ujung_pola_dibungkus_ke_awal() {
    let (mut m, mut k) = Engine::new(SR);
    k.set_settings(&SeqSettings {
        bpm: 120.0,
        count_in_bars: 0,
        metronome: false,
        ..SeqSettings::default()
    });
    k.transport(true, true);
    k.hit(0, 1.0); // tick 0, tetapi kompensasi memundurkannya ke bar sebelumnya
    render(&mut m, 256, 256);
    k.poll_recording(10.0);
    let n = k.sequence().pattern.notes[0];
    assert_eq!(n.tick, TICKS_PER_BAR - 19, "{n:?}"); // 10 ms ≈ 19,2 tick
}

#[test]
fn ketukan_saat_count_in_tidak_direkam() {
    let (mut m, mut k) = Engine::new(SR);
    k.transport(true, true); // count-in 1 bar (bawaan)
    k.hit(0, 1.0);
    render(&mut m, 256, 256);
    assert_eq!(k.poll_recording(0.0), 0);
}

#[test]
fn ganti_tempo_saat_bermain_tetap_sinkron() {
    let (mut m, mut k) = mesin_impuls(120.0);
    k.set_pattern(Pattern {
        bars: 1,
        notes: vec![not(0, 0), not(1_920, 0)],
        ..Pattern::default()
    });
    k.transport(true, false);
    render(&mut m, 24_000, 256); // tick 960
    k.set_settings(&SeqSettings {
        bpm: 60.0,
        metronome: false,
        ..SeqSettings::default()
    });
    // Di 60 BPM, 960 tick berikutnya = 48 000 frame.
    let x = render(&mut m, 48_100, 256);
    assert_eq!(awal_bunyi(&x), [48_000]);
}

#[test]
fn stop_melepas_pad_gate() {
    let (mut m, mut k) = Engine::new(SR);
    let p = PadParams {
        play_mode: crate::pad::PlayMode::Loop,
        ..*k.pad(1).unwrap()
    };
    k.set_pad(1, &p);
    k.set_pattern(Pattern {
        bars: 1,
        notes: vec![Note {
            length: TICKS_PER_BAR,
            ..not(0, 1)
        }],
        ..Pattern::default()
    });
    k.transport(true, false);
    render(&mut m, 10_000, 256);
    assert!(m.stats().voices_active() > 0);
    k.transport(false, false);
    render(&mut m, 48_000, 256);
    assert_eq!(m.stats().voices_active(), 0);
}

#[test]
fn model_sequencer_dibawa_ke_mesin_baru() {
    let (_m, mut k) = Engine::new(SR);
    k.set_settings(&SeqSettings {
        bpm: 97.0,
        ..SeqSettings::default()
    });
    k.set_pattern(Pattern {
        bars: 4,
        notes: vec![not(0, 0)],
        ..Pattern::default()
    });
    let (bank, seq) = k.into_model();
    let (_m, k) = Engine::with_model(SR, bank, seq);
    assert!((k.sequence().settings.bpm - 97.0).abs() < 1e-6);
    assert_eq!(k.sequence().pattern.bars, 4);
}
