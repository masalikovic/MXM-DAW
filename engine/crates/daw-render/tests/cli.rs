//! Tes ujung ke ujung: menjalankan biner `daw-render` yang sungguhan, sama
//! seperti CI dan manusia memakainya.

use std::process::Command;

fn daw_render(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_daw-render"))
        .args(args)
        .output()
        .expect("biner daw-render bisa dijalankan")
}

#[test]
fn version_berhasil_dan_mencetak_versi_mesin() {
    let hasil = daw_render(&["--version"]);
    assert!(hasil.status.success(), "{hasil:?}");
    let stdout = String::from_utf8(hasil.stdout).unwrap();
    assert_eq!(
        stdout,
        format!(
            "daw-render {} (mesin {})\n",
            env!("CARGO_PKG_VERSION"),
            daw_core::engine_version()
        )
    );
}

#[test]
fn argumen_tak_dikenal_keluar_dengan_kode_2() {
    let hasil = daw_render(&["--terbang"]);
    assert_eq!(hasil.status.code(), Some(2), "{hasil:?}");
    assert!(hasil.stdout.is_empty());
    assert!(!hasil.stderr.is_empty());
}

#[test]
fn kick_menulis_wav_yang_bisa_dibaca_lagi() {
    let berkas = std::env::temp_dir().join("mxm-daw-tes-kick.wav");
    let hasil = daw_render(&["kick", berkas.to_str().unwrap(), "0.5"]);
    assert!(hasil.status.success(), "{hasil:?}");

    let mut wav = hound::WavReader::open(&berkas).unwrap();
    let spek = wav.spec();
    assert_eq!(
        (spek.channels, spek.sample_rate, spek.bits_per_sample),
        (2, 48_000, 32)
    );
    let sampel: Vec<f32> = wav.samples::<f32>().map(Result::unwrap).collect();
    assert_eq!(sampel.len(), 2 * 48_000 * 2);
    let puncak = sampel.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
    assert!(
        puncak > 0.3 && puncak < 0.5,
        "puncak velocity 0,5: {puncak}"
    );
    std::fs::remove_file(berkas).unwrap();
}
