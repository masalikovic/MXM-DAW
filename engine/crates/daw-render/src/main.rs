//! `daw-render`: CLI mesin MXM-DAW.
//!
//! Nantinya CLI ini merender proyek dan kit pabrik menjadi WAV tanpa perangkat
//! audio, untuk tes golden dan benchmark di CI (rencana §9, §12). Saat ini:
//! `--version`, `--help`, dan `kick <berkas.wav>` (M1: dengarkan kick mesin).

use std::process::ExitCode;

const BANTUAN: &str = "\
Pemakaian: daw-render [PILIHAN]

Pilihan:
  --version   Tampilkan versi CLI dan versi mesin
  --help      Tampilkan bantuan ini

Perintah:
  kick <berkas.wav> [velocity]
              Render satu kick (2 detik, 48 kHz, stereo, 32-bit float)
              dengan mesin yang sama persis dengan aplikasi";

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
        [perintah, berkas] if perintah == "kick" => render_kick(berkas, 1.0),
        [perintah, berkas, velocity] if perintah == "kick" => {
            let v: f32 = velocity
                .parse()
                .map_err(|_| format!("velocity bukan angka: {velocity}"))?;
            if !(0.0..=1.0).contains(&v) {
                return Err(format!("velocity harus 0..1: {velocity}"));
            }
            render_kick(berkas, v)
        }
        _ => Err(format!(
            "argumen tidak dikenal: {}\n\n{BANTUAN}",
            args.join(" ")
        )),
    }
}

/// Render satu kick ke WAV, lalu laporkan puncaknya.
fn render_kick(berkas: &str, velocity: f32) -> Result<String, String> {
    const SR: u32 = 48_000;
    let (mut mesin, mut kirim) = daw_core::engine::Engine::new(SR);
    kirim.hit(velocity);
    let mut sampel = vec![0.0_f32; 2 * SR as usize * 2];
    for blok in sampel.chunks_mut(192 * 2) {
        mesin.render(blok, 2);
    }
    let spek = hound::WavSpec {
        channels: 2,
        sample_rate: SR,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let galat = |e: hound::Error| format!("gagal menulis {berkas}: {e}");
    let mut penulis = hound::WavWriter::create(berkas, spek).map_err(galat)?;
    for &x in &sampel {
        penulis.write_sample(x).map_err(galat)?;
    }
    penulis.finalize().map_err(galat)?;
    let puncak = sampel.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
    Ok(format!(
        "{berkas}: kick velocity {velocity}, 2 detik, puncak {:.1} dBFS",
        20.0 * puncak.log10()
    ))
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
    fn kick_velocity_salah_ditolak() {
        assert!(run(&args(&["kick", "x.wav", "2"])).is_err());
        assert!(run(&args(&["kick", "x.wav", "keras"])).is_err());
    }

    #[test]
    fn argumen_tak_dikenal_ditolak() {
        let galat = run(&args(&["--terbang"])).unwrap_err();
        assert!(galat.contains("--terbang"), "{galat}");
    }
}
