//! Tes golden (rencana §12): beat demo 4 bar dirender lewat sequencer, lalu
//! "sidik jari" RMS per 100 ms dibandingkan dengan `golden/demo-rms.txt`.
//!
//! Kenapa RMS, bukan hash: hasil `sin`/`exp` bisa berbeda satu bit antar
//! sistem, dan hash akan gagal karena hal yang tidak terdengar. RMS per
//! jendela tetap menangkap yang penting: not yang hilang, bergeser, atau
//! berubah keras-pelannya.
//!
//! Bila perubahan bunyi memang disengaja, perbarui berkasnya:
//! `MXM_GOLDEN_UPDATE=1 cargo test -p daw-render --test golden`, lalu dengarkan
//! `daw-render demo demo.wav` sebelum commit.

use std::fmt::Write as _;
use std::process::Command;

const JENDELA: usize = 4_800;
const TOLERANSI: f64 = 2e-3;

fn sidik_jari(berkas: &std::path::Path) -> Vec<(f64, f64)> {
    let mut wav = hound::WavReader::open(berkas).unwrap();
    let x: Vec<f32> = wav.samples::<f32>().map(Result::unwrap).collect();
    x.chunks(JENDELA * 2)
        .map(|j| {
            let n = f64::from(u32::try_from(j.len() / 2).unwrap().max(1));
            let (l, r) = j.chunks_exact(2).fold((0.0, 0.0), |(l, r), f| {
                (l + f64::from(f[0]).powi(2), r + f64::from(f[1]).powi(2))
            });
            ((l / n).sqrt(), (r / n).sqrt())
        })
        .collect()
}

#[test]
fn demo_sama_dengan_golden() {
    let wav = std::env::temp_dir().join("mxm-daw-golden-demo.wav");
    let hasil = Command::new(env!("CARGO_BIN_EXE_daw-render"))
        .args(["demo", wav.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(hasil.status.success(), "{hasil:?}");
    let sekarang = sidik_jari(&wav);
    std::fs::remove_file(&wav).unwrap();

    let jalur = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/demo-rms.txt");
    if std::env::var_os("MXM_GOLDEN_UPDATE").is_some() {
        let mut teks = String::new();
        for (l, r) in &sekarang {
            writeln!(teks, "{l:.6} {r:.6}").unwrap();
        }
        std::fs::write(jalur, teks).unwrap();
        return;
    }
    let golden: Vec<(f64, f64)> = std::fs::read_to_string(jalur)
        .expect("golden/demo-rms.txt ada (buat dengan MXM_GOLDEN_UPDATE=1)")
        .lines()
        .map(|b| {
            let mut a = b.split_whitespace().map(|v| v.parse::<f64>().unwrap());
            (a.next().unwrap(), a.next().unwrap())
        })
        .collect();

    assert_eq!(sekarang.len(), golden.len(), "panjang render berubah");
    for (i, ((l, r), (gl, gr))) in sekarang.iter().zip(&golden).enumerate() {
        assert!(
            (l - gl).abs() < TOLERANSI && (r - gr).abs() < TOLERANSI,
            "jendela {i} ({} ms): ({l:.4}, {r:.4}) ≠ golden ({gl:.4}, {gr:.4})",
            i * 100
        );
    }
    // Penjaga: demo memang berbunyi (golden yang isinya hening tidak berguna).
    assert!(golden.iter().filter(|(l, _)| *l > 0.05).count() > 50);
}
