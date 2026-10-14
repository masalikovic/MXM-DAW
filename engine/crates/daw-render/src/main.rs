//! `daw-render`: CLI mesin MXM-DAW.
//!
//! Nantinya CLI ini merender proyek dan kit pabrik menjadi WAV tanpa perangkat
//! audio, untuk tes golden dan benchmark di CI (rencana §9, §12). Saat ini:
//! `--version`, `--help`, `kit <folder>` (M2: dengarkan kit pabrik), dan
//! `demo <berkas.wav>` (M3: beat demo 4 bar dari sequencer, juga tes golden).

use std::process::ExitCode;

const BANTUAN: &str = "\
Pemakaian: daw-render [PILIHAN]

Pilihan:
  --version   Tampilkan versi CLI dan versi mesin
  --help      Tampilkan bantuan ini

Perintah:
  kit <folder> [velocity]
              Render 16 suara kit pabrik ke folder (01-kick.wav, …; 48 kHz,
              stereo, 32-bit float) dengan mesin yang sama persis dengan
              aplikasi
  demo <berkas.wav>
              Render beat demo 4 bar (96 BPM, swing 56 %) lewat sequencer";

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
        [perintah, folder] if perintah == "kit" => render_kit(folder, 1.0),
        [perintah, berkas] if perintah == "demo" => {
            let audio = render_demo();
            tulis_wav(std::path::Path::new(berkas), &audio, 48_000)?;
            Ok(format!("{berkas}: demo 4 bar, {} frame", audio.len() / 2))
        }
        [perintah, folder, velocity] if perintah == "kit" => {
            let v: f32 = velocity
                .parse()
                .map_err(|_| format!("velocity bukan angka: {velocity}"))?;
            if !(0.0..=1.0).contains(&v) {
                return Err(format!("velocity harus 0..1: {velocity}"));
            }
            render_kit(folder, v)
        }
        _ => Err(format!(
            "argumen tidak dikenal: {}\n\n{BANTUAN}",
            args.join(" ")
        )),
    }
}

/// Render setiap pad kit pabrik ke WAV tersendiri, lalu laporkan puncaknya.
fn render_kit(folder: &str, velocity: f32) -> Result<String, String> {
    const SR: u32 = 48_000;
    std::fs::create_dir_all(folder).map_err(|e| format!("gagal membuat {folder}: {e}"))?;
    let mut laporan = Vec::new();
    for pad in 0..daw_core::pad::PAD_COUNT {
        let (mut mesin, mut kirim) = daw_core::engine::Engine::new(SR);
        let nama = kirim.name(pad).unwrap_or("PAD").to_lowercase();
        // Panjang sampel + 0,1 s untuk ekor release/filter.
        let frames = kirim.sample(pad).map_or(0, |s| s.frames()) + SR as usize / 10;
        kirim.hit(pad, velocity);
        let mut sampel = vec![0.0_f32; frames * 2];
        for blok in sampel.chunks_mut(192 * 2) {
            mesin.render(blok, 2);
        }
        let berkas = std::path::Path::new(folder).join(format!("{:02}-{nama}.wav", pad + 1));
        tulis_wav(&berkas, &sampel, SR)?;
        let puncak = sampel.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
        laporan.push(format!(
            "{}: puncak {:.1} dBFS",
            berkas.display(),
            20.0 * puncak.log10()
        ));
    }
    Ok(laporan.join("\n"))
}

/// Beat demo 4 bar: kick, snare, hi-hat, clap, dan bas, diputar sequencer.
/// Dipakai `daw-render demo` dan tes golden (`tests/golden.rs`).
#[must_use]
fn render_demo() -> Vec<f32> {
    use daw_core::seq::{Note, Pattern, STEP_TICKS, SeqSettings, TICKS_PER_BAR};
    let (mut mesin, mut kirim) = daw_core::engine::Engine::new(48_000);
    let mut notes = Vec::new();
    let mut not = |langkah: u32, pad: u8, velocity: f32, panjang: u32| {
        notes.push(Note {
            tick: langkah * STEP_TICKS,
            pad,
            velocity,
            length: panjang * STEP_TICKS,
        });
    };
    for bar in 0..4 {
        let b = bar * 16;
        not(b, 0, 1.0, 1);
        not(b + 10, 0, 0.8, 1);
        not(b + 4, 1, 0.9, 1);
        not(b + 12, 1, 0.9, 1);
        for i in (0..16).step_by(2) {
            not(b + i, 3, if i % 4 == 0 { 0.8 } else { 0.55 }, 1);
        }
        not(b + 14, 4, 0.6, 2);
        not(b, 13, 0.9, 3);
        not(b + 7, 13, 0.7, 2);
    }
    not(60, 2, 1.0, 1);
    kirim.set_settings(&SeqSettings {
        bpm: 96.0,
        swing: 0.56,
        metronome: false,
        count_in_bars: 0,
    });
    kirim.set_pattern(Pattern {
        bars: 4,
        notes,
        ..Pattern::default()
    });
    kirim.transport(true, false);
    // 4 bar pada 96 BPM = 10 detik, ditambah 1 detik ekor.
    let ticks = f64::from(4 * TICKS_PER_BAR);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let frames = (ticks * 60.0 * 48_000.0 / (96.0 * 960.0)) as usize + 48_000;
    let mut audio = vec![0.0_f32; frames * 2];
    for blok in audio.chunks_mut(192 * 2) {
        mesin.render(blok, 2);
    }
    audio
}

/// Tulis audio stereo float 32-bit ke WAV.
fn tulis_wav(berkas: &std::path::Path, sampel: &[f32], sr: u32) -> Result<(), String> {
    let spek = hound::WavSpec {
        channels: 2,
        sample_rate: sr,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let galat = |e: hound::Error| format!("gagal menulis {}: {e}", berkas.display());
    let mut penulis = hound::WavWriter::create(berkas, spek).map_err(galat)?;
    for &x in sampel {
        penulis.write_sample(x).map_err(galat)?;
    }
    penulis.finalize().map_err(galat)
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
    fn kit_velocity_salah_ditolak() {
        assert!(run(&args(&["kit", "x", "2"])).is_err());
        assert!(run(&args(&["kit", "x", "keras"])).is_err());
    }

    #[test]
    fn argumen_tak_dikenal_ditolak() {
        let galat = run(&args(&["--terbang"])).unwrap_err();
        assert!(galat.contains("--terbang"), "{galat}");
    }
}
