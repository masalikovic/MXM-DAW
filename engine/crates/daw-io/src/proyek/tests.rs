use std::path::PathBuf;
use std::sync::Arc;

use daw_core::bank::{PadBank, SampleSource};
use daw_core::pad::{FilterMode, PadParams, PlayMode};
use daw_core::sampel::Sampel;
use daw_core::seq::{Note, Pattern, SeqSettings, Sequence};

use super::{LoadError, PROJECT_FILE, load, save};

/// Folder kosong yang unik untuk satu tes.
fn folder(nama: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mxm-daw-proyek-{nama}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn sinus() -> Sampel {
    #[allow(clippy::cast_precision_loss)]
    let x: Vec<f32> = (0..4_410)
        .map(|i| (i as f32 * 0.0627).sin() * 0.7)
        .collect();
    Sampel::from_f32(&x, 1, 44_100)
}

/// Model yang menyentuh semua bagian format: pad pabrik yang disunting,
/// sampel impor di dua grup, pola di grup A dan C, mute, pengaturan.
fn model_contoh() -> (PadBank, Sequence) {
    let mut bank = PadBank::factory();
    let mut p = *bank.params(2).unwrap();
    p.pitch = -4.5;
    p.filter.mode = FilterMode::BandPass;
    p.filter.cutoff = 0.4;
    p.play_mode = PlayMode::Loop;
    p.choke = 3;
    bank.set_params(2, &p);
    bank.set_sample(0, "VOKAL", Arc::new(sinus()));
    bank.set_sample(
        40,
        "STEREO",
        Arc::new(Sampel::from_f32(&[0.1, -0.2, 0.3, -0.4], 2, 48_000)),
    );
    let mut seq = Sequence {
        settings: SeqSettings {
            bpm: 97.5,
            swing: 0.58,
            metronome: false,
            count_in_bars: 2,
        },
        ..Sequence::default()
    };
    seq.mutes[1] = true;
    seq.patterns[0] = Pattern {
        bars: 4,
        quantize_grid: 240,
        quantize_strength: 0.5,
        notes: vec![
            Note {
                tick: 0,
                pad: 0,
                velocity: 1.0,
                length: 240,
            },
            Note {
                tick: 1_013,
                pad: 2,
                velocity: 0.7,
                length: 480,
            },
        ],
    };
    seq.patterns[2].notes.push(Note {
        tick: 960,
        pad: 8,
        velocity: 0.4,
        length: 60,
    });
    (bank, seq)
}

fn sama(a: &PadBank, b: &PadBank) {
    for pad in 0..64 {
        assert_eq!(a.params(pad), b.params(pad), "params pad {pad}");
        assert_eq!(a.name(pad), b.name(pad), "nama pad {pad}");
        assert_eq!(a.source(pad), b.source(pad), "asal pad {pad}");
        assert_eq!(
            a.sample(pad).map(|s| s.as_i16().to_vec()),
            b.sample(pad).map(|s| s.as_i16().to_vec()),
            "sampel pad {pad} bit demi bit"
        );
    }
}

#[test]
fn simpan_lalu_buka_sama_persis() {
    let d = folder("bolak-balik");
    let (bank, seq) = model_contoh();
    save(&d, &bank, &seq).unwrap();
    let hasil = load(&d).unwrap().unwrap();
    assert!(hasil.warnings.is_empty(), "{:?}", hasil.warnings);
    sama(&bank, &hasil.bank);
    assert_eq!(hasil.seq, seq);
}

#[test]
fn belum_ada_proyek() {
    assert!(load(&folder("kosong")).unwrap().is_none());
}

#[test]
fn kit_pabrik_tidak_disalin_ke_disk() {
    let d = folder("pabrik");
    save(&d, &PadBank::factory(), &Sequence::default()).unwrap();
    let wav = std::fs::read_dir(d.join("samples")).unwrap().count();
    assert_eq!(wav, 0);
    let json = std::fs::read_to_string(d.join(PROJECT_FILE)).unwrap();
    assert!(json.contains("\"factory:KICK\""), "{json}");
    assert!(json.contains("\"schema\": 1"));
}

#[test]
fn sampel_lama_dihapus_setelah_diganti() {
    let d = folder("gc");
    let (mut bank, seq) = model_contoh();
    save(&d, &bank, &seq).unwrap();
    assert!(d.join("samples/s1.wav").exists());
    bank.set_sample(0, "BARU", Arc::new(sinus()));
    save(&d, &bank, &seq).unwrap();
    assert!(!d.join("samples/s1.wav").exists(), "tidak dirujuk lagi");
    assert!(d.join("samples/s3.wav").exists());
    let hasil = load(&d).unwrap().unwrap();
    assert_eq!(hasil.bank.source(0), Some(SampleSource::Imported(3)));
    // Nomor berikutnya melanjutkan, bukan menimpa berkas yang ada.
    let mut b = hasil.bank;
    b.set_sample(5, "LAGI", Arc::new(sinus()));
    assert_eq!(b.source(5), Some(SampleSource::Imported(4)));
}

#[test]
fn berkas_sementara_sisa_mati_paksa_diabaikan() {
    let d = folder("tmp");
    let (bank, seq) = model_contoh();
    save(&d, &bank, &seq).unwrap();
    // Seperti aplikasi dimatikan saat menulis: tmp setengah jadi tertinggal.
    std::fs::write(d.join("project.tmp"), b"{\"schema\": 1, \"bpm\": 1").unwrap();
    let hasil = load(&d).unwrap().unwrap();
    assert_eq!(hasil.seq, seq);
}

#[test]
fn proyek_versi_lebih_baru_ditolak_dan_tidak_disentuh() {
    let d = folder("baru");
    std::fs::write(d.join(PROJECT_FILE), b"{\"schema\": 99, \"bpm\": 120}").unwrap();
    assert_eq!(load(&d).unwrap_err(), LoadError::TooNew(99));
    assert!(d.join(PROJECT_FILE).exists(), "berkas tetap di tempatnya");
}

#[test]
fn proyek_rusak_dipindah_bukan_ditimpa() {
    let d = folder("rusak");
    std::fs::write(d.join(PROJECT_FILE), b"{\"schema\": 1, \"bpm\": ").unwrap();
    let galat = load(&d).unwrap_err();
    let LoadError::Corrupt { moved_to, .. } = &galat else {
        panic!("{galat:?}");
    };
    assert_eq!(moved_to, "project.rusak.json");
    assert!(!d.join(PROJECT_FILE).exists());
    assert!(d.join("project.rusak.json").exists());
    assert!(galat.to_string().contains("rusak"));
}

#[test]
fn sampel_hilang_menjadi_peringatan() {
    let d = folder("hilang");
    let (bank, seq) = model_contoh();
    save(&d, &bank, &seq).unwrap();
    std::fs::remove_file(d.join("samples/s1.wav")).unwrap();
    let hasil = load(&d).unwrap().unwrap();
    assert_eq!(hasil.warnings.len(), 1, "{:?}", hasil.warnings);
    assert!(hasil.bank.sample(0).is_none());
    assert_eq!(hasil.bank.source(0), Some(SampleSource::Empty));
    assert_eq!(hasil.bank.name(0), Some("VOKAL"), "parameter lain tetap");
}

#[test]
fn berkas_skema_1_dari_repo_tetap_terbaca() {
    // Penjaga format: berkas ini tidak boleh berhenti terbaca. Bila format
    // berubah, tambahkan kolom dengan nilai bawaan, jangan ubah yang ada.
    let d = folder("fixture");
    std::fs::write(
        d.join(PROJECT_FILE),
        include_bytes!("../../tests/fixtures/proyek-skema-1.json"),
    )
    .unwrap();
    let hasil = load(&d).unwrap().unwrap();
    assert!((hasil.seq.settings.bpm - 96.0).abs() < 1e-6);
    assert_eq!(hasil.seq.patterns[0].bars, 2);
    assert_eq!(hasil.seq.patterns[0].notes.len(), 3);
    assert_eq!(hasil.bank.params(1).unwrap().play_mode, PlayMode::Gate);
    assert_eq!(
        hasil.bank.name(17),
        Some("SNARE"),
        "pad yang tidak disebut tetap pabrik"
    );
    // Kolom yang tidak dikenal (dari versi lain) diabaikan.
    assert!(hasil.warnings.is_empty(), "{:?}", hasil.warnings);
    let _ = PadParams::default();
}
