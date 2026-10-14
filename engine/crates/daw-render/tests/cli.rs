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
