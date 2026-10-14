//! `daw-render`: CLI mesin MXM-DAW.
//!
//! Nantinya CLI ini merender proyek dan kit pabrik menjadi WAV tanpa perangkat
//! audio, untuk tes golden dan benchmark di CI (rencana §9, §12). Di M0 baru
//! ada `--version` dan `--help`.

use std::process::ExitCode;

const BANTUAN: &str = "\
Pemakaian: daw-render [PILIHAN]

Pilihan:
  --version   Tampilkan versi CLI dan versi mesin
  --help      Tampilkan bantuan ini";

/// Hasil satu perintah: teks untuk stdout, atau pesan galat untuk stderr.
///
/// Dipisah dari `main` supaya bisa dites tanpa menjalankan proses baru.
fn run(args: &[String]) -> Result<String, String> {
    match args {
        [] => Ok(BANTUAN.to_owned()),
        [satu] if satu == "--help" => Ok(BANTUAN.to_owned()),
        [satu] if satu == "--version" => Ok(format!(
            "daw-render {} (mesin {})",
            env!("CARGO_PKG_VERSION"),
            daw_core::engine_version()
        )),
        _ => Err(format!(
            "argumen tidak dikenal: {}\n\n{BANTUAN}",
            args.join(" ")
        )),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(keluaran) => {
            println!("{keluaran}");
            ExitCode::SUCCESS
        }
        Err(galat) => {
            eprintln!("daw-render: {galat}");
            // 2 = argumen salah, konvensi umum alat baris perintah.
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    fn args(daftar: &[&str]) -> Vec<String> {
        daftar.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn version_menyebut_versi_cli_dan_mesin() {
        let keluaran = run(&args(&["--version"])).unwrap();
        assert_eq!(
            keluaran,
            format!(
                "daw-render {} (mesin {})",
                env!("CARGO_PKG_VERSION"),
                daw_core::engine_version()
            )
        );
    }

    #[test]
    fn tanpa_argumen_menampilkan_bantuan() {
        let keluaran = run(&[]).unwrap();
        assert!(keluaran.contains("--version"), "{keluaran}");
    }

    #[test]
    fn help_sama_dengan_tanpa_argumen() {
        assert_eq!(run(&args(&["--help"])), run(&[]));
    }

    #[test]
    fn argumen_tak_dikenal_ditolak() {
        let galat = run(&args(&["--terbang"])).unwrap_err();
        assert!(galat.contains("--terbang"), "{galat}");
    }
}
