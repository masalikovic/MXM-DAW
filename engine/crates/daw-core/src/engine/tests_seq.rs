//! Tes sequencer di mesin: timing tepat-sampel, count-in, metronom, rekaman.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::sync::Arc;

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
        pitch: 0,
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
    k.set_pattern(
        0,
        Pattern {
            bars: 1,
            notes: vec![not(0, 0), not(1_000, 0), not(2_401, 0)],
            ..Pattern::default()
        },
    );
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
        k.set_pattern(
            0,
            Pattern {
                bars: 2,
                notes: (0..32)
                    .map(|i| not(i * STEP_TICKS, (i % 16) as u8))
                    .collect(),
                ..Pattern::default()
            },
        );
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
    k.set_pattern(
        0,
        Pattern {
            bars: 1,
            notes: vec![not(0, 0), not(STEP_TICKS, 0)],
            ..Pattern::default()
        },
    );
    k.transport(true, false);
    let awal = awal_bunyi(&render(&mut m, 24_000, 256));
    let geser = (0.66 - 0.5) * 2.0 * f64::from(STEP_TICKS);
    assert_eq!(awal, [0, frame_harapan(f64::from(STEP_TICKS) + geser, bpm)]);
}

#[test]
fn berhenti_dan_mulai_lagi_dari_awal() {
    let (mut m, mut k) = mesin_impuls(120.0);
    k.set_pattern(
        0,
        Pattern {
            bars: 1,
            notes: vec![not(0, 0)],
            ..Pattern::default()
        },
    );
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
    k.set_pattern(
        0,
        Pattern {
            bars: 1,
            notes: vec![not(0, 0)],
            ..Pattern::default()
        },
    );
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
    let n = k.sequence().patterns[0].notes[0];
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
    let n = k.sequence().patterns[0].notes[0];
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
    k.set_pattern(
        0,
        Pattern {
            bars: 1,
            notes: vec![not(0, 0), not(1_920, 0)],
            ..Pattern::default()
        },
    );
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
    k.set_pattern(
        0,
        Pattern {
            bars: 1,
            notes: vec![Note {
                length: TICKS_PER_BAR,
                pitch: 0,
                ..not(0, 1)
            }],
            ..Pattern::default()
        },
    );
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
    k.set_pattern(
        0,
        Pattern {
            bars: 4,
            notes: vec![not(0, 0)],
            ..Pattern::default()
        },
    );
    let (bank, seq) = k.into_model();
    let (_m, k) = Engine::with_model(SR, bank, seq);
    assert!((k.sequence().settings.bpm - 97.0).abs() < 1e-6);
    assert_eq!(k.sequence().patterns[0].bars, 4);
}

#[test]
fn empat_grup_berputar_dengan_panjang_sendiri_dan_mute() {
    let (mut m, mut k) = mesin_impuls(120.0);
    // Grup A: 1 bar, not di awal. Grup B: 2 bar, not di awal (pad 16..).
    let mut x = vec![0.0; 64];
    x[0] = 1.0;
    k.set_sample(16, "IMPULS", Sampel::from_f32(&x, 1, 48_000));
    k.set_pattern(
        0,
        Pattern {
            bars: 1,
            notes: vec![not(0, 0)],
            ..Pattern::default()
        },
    );
    k.set_pattern(
        1,
        Pattern {
            bars: 2,
            notes: vec![not(960, 0)],
            ..Pattern::default()
        },
    );
    k.transport(true, false);
    // 120 BPM: 1 bar = 96 000 frame. Dalam 4 bar: A berbunyi 4×, B 2×.
    let awal = awal_bunyi(&render(&mut m, 4 * 96_000, 256));
    assert_eq!(awal, [0, 24_000, 96_000, 192_000, 216_000, 288_000]);

    // Mute grup B: polanya diam, A tetap.
    k.set_mute(1, true);
    let awal = awal_bunyi(&render(&mut m, 4 * 96_000, 256));
    assert_eq!(awal.len(), 4);
    assert_eq!(k.sequence().mutes, [false, true, false, false]);
}

#[test]
fn rekaman_masuk_ke_pola_grupnya() {
    let (mut m, mut k) = Engine::new(SR);
    k.set_settings(&SeqSettings {
        count_in_bars: 0,
        metronome: false,
        ..SeqSettings::default()
    });
    k.transport(true, true);
    k.hit(16 * 2 + 5, 1.0); // grup C, pad 6
    render(&mut m, 256, 256);
    k.poll_recording(0.0);
    assert!(k.sequence().patterns[0].notes.is_empty());
    assert_eq!(k.sequence().patterns[2].notes[0].pad, 5);
}

#[test]
fn keys_transpose_satu_oktaf_menggandakan_kecepatan() {
    let panjang = |nada: i8| {
        let (mut m, mut k) = Engine::new(SR);
        let x: Vec<f32> = (0..48_000)
            .map(|i| 0.5 * (core::f32::consts::TAU * 220.0 * i as f32 / 48_000.0).sin())
            .collect();
        k.set_sample(0, "SINUS", Sampel::from_f32(&x, 1, 48_000));
        k.hit_note(0, 1.0, nada);
        let y = render(&mut m, 60_000, 256);
        y.iter().rposition(|v| v.abs() > 1e-3).unwrap()
    };
    assert!(panjang(0).abs_diff(48_000) < 100);
    assert!(panjang(12).abs_diff(24_000) < 100);
    assert!(
        panjang(-12) > 59_000,
        "satu oktaf lebih rendah = dua kali lebih lama"
    );
}

#[test]
fn lepas_satu_nada_tidak_memutus_akor() {
    let (mut m, mut k) = Engine::new(SR);
    let p = PadParams {
        play_mode: crate::pad::PlayMode::Gate,
        ..*k.pad(13).unwrap()
    };
    k.set_pad(13, &p);
    k.hit_note(13, 1.0, 0);
    k.hit_note(13, 1.0, 7);
    render(&mut m, 2_000, 256);
    assert_eq!(m.stats().voices_active(), 2);
    k.release_note(13, 7);
    render(&mut m, 9_600, 256);
    assert_eq!(m.stats().voices_active(), 1, "nada 0 masih ditahan");
}

#[test]
fn rekaman_keys_menyimpan_transpose_dan_dimainkan_ulang() {
    let (mut m, mut k) = Engine::new(SR);
    k.set_settings(&SeqSettings {
        count_in_bars: 0,
        metronome: false,
        ..SeqSettings::default()
    });
    k.transport(true, true);
    k.hit_note(13, 1.0, 5);
    render(&mut m, 256, 256);
    k.release_note(13, 5);
    render(&mut m, 2_400, 256);
    k.poll_recording(0.0);
    let n = k.sequence().patterns[0].notes[0];
    assert_eq!((n.pad, n.pitch), (13, 5));
    assert!(n.length > 1, "lepas pada nada yang sama menutup not");
}

#[test]
fn note_repeat_saat_berhenti_mengikuti_jam_bebas() {
    // 120 BPM: 1/16 = 240 tick = 6 000 frame.
    let (mut m, mut k) = mesin_impuls(120.0);
    render(&mut m, 1_000, 256);
    k.hit(0, 1.0);
    k.repeat(0, 1.0, 0, 240);
    let x = render(&mut m, 30_000, 256); // frame absolut 1 000 … 31 000
    let awal: Vec<usize> = awal_bunyi(&x).iter().map(|f| f + 1_000).collect();
    assert_eq!(awal, [1_000, 6_000, 12_000, 18_000, 24_000, 30_000]);

    k.repeat(0, 1.0, 0, 0);
    assert!(
        awal_bunyi(&render(&mut m, 30_000, 256)).is_empty(),
        "berhenti"
    );
}

#[test]
fn note_repeat_saat_merekam_masuk_ke_pola() {
    let (mut m, mut k) = mesin_impuls(120.0);
    k.set_settings(&SeqSettings {
        bpm: 120.0,
        count_in_bars: 0,
        metronome: false,
        ..SeqSettings::default()
    });
    k.transport(true, true);
    k.hit(0, 0.8);
    k.repeat(0, 0.8, 0, 960); // tiap ketukan
    render(&mut m, 96_000, 256); // satu bar
    k.repeat(0, 0.8, 0, 0);
    k.poll_recording(0.0);
    let mut tick: Vec<u32> = k.sequence().patterns[0]
        .notes
        .iter()
        .map(|n| n.tick)
        .collect();
    tick.sort_unstable();
    assert_eq!(tick, [0, 960, 1_920, 2_880]);
}

#[test]
fn terapkan_model_lama_memulihkan_bunyi_dan_pola() {
    let (mut m, mut k) = mesin_impuls(120.0);
    let (bank0, seq0) = (k.bank().clone(), k.sequence().clone());
    // Ubah: pola grup A, pitch pad 0, mute grup B, tempo.
    k.set_pattern(
        0,
        Pattern {
            bars: 1,
            notes: vec![not(0, 0)],
            ..Pattern::default()
        },
    );
    let mut p = *k.pad(0).unwrap();
    p.level = 0.0;
    k.set_pad(0, &p);
    k.set_mute(1, true);
    // Kembali ke model awal (seperti undo).
    assert!(k.apply_model(bank0, seq0.clone()));
    assert_eq!(k.sequence(), &seq0);
    assert!((k.pad(0).unwrap().level - 1.0).abs() < f32::EPSILON);
    k.transport(true, false);
    render(&mut m, 9_600, 256);
    assert_eq!(m.stats().hits_played(), 0, "pola kosong lagi");
}

#[test]
fn salin_pad_dan_grup() {
    let (mut m, mut k) = mesin_impuls(120.0);
    assert!(k.copy_pad(0, 20));
    assert_eq!(k.name(20), Some("IMPULS"));
    assert!(
        Arc::ptr_eq(k.sample(0).unwrap(), k.sample(20).unwrap()),
        "sampel dibagi"
    );
    k.set_pattern(
        0,
        Pattern {
            bars: 1,
            notes: vec![not(0, 0)],
            ..Pattern::default()
        },
    );
    assert!(k.copy_group(0, 3));
    assert_eq!(k.sequence().patterns[3], k.sequence().patterns[0]);
    assert_eq!(k.name(48), Some("IMPULS"));
    // Grup D memainkan salinan polanya: dua not (A dan D) di awal.
    k.transport(true, false);
    render(&mut m, 256, 256);
    assert_eq!(m.stats().hits_played(), 2);
    assert!(!k.copy_group(0, 4));
}
