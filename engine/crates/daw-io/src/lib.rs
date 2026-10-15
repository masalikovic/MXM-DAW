//! Membaca sampel audio untuk MXM-DAW.
//!
//! Crate ini menerima **byte**, bukan jalur berkas. Di Android, berkas dipilih
//! lewat Storage Access Framework dan dibaca oleh Kotlin, jadi aplikasi tidak
//! butuh izin penyimpanan (rencana §4.2, ADR-0006). Akibatnya semua fungsi di
//! sini bisa dites di mana saja dengan byte buatan.
//!
//! M2: WAV (PCM 8/16/24/32-bit dan float 32-bit). AIFF, FLAC, MP3, dan OGG
//! menyusul. M4: berkas proyek ([`proyek`]).

use std::io::Cursor;

pub mod proyek;

use daw_core::sampel::Sampel;

/// Sampel terpanjang yang diterima (detik). 2 menit stereo 48 kHz ≈ 23 MB
/// sebagai int16, masih jauh di bawah anggaran memori sampel (rencana §9).
pub const MAX_SECONDS: u32 = 120;

/// Batas sample rate yang masuk akal.
const RATE_MIN: u32 = 8_000;
const RATE_MAX: u32 = 192_000;

/// Baca WAV dari byte. Lebih dari dua kanal: hanya dua kanal pertama dipakai.
///
/// # Errors
/// Pesan yang bisa ditampilkan ke pengguna (bahasa Indonesia) bila berkas bukan
/// WAV, formatnya tidak didukung, kosong, atau terlalu panjang.
pub fn decode_wav(bytes: &[u8]) -> Result<Sampel, String> {
    let mut wav = hound::WavReader::new(Cursor::new(bytes))
        .map_err(|e| format!("bukan berkas WAV yang bisa dibaca ({e})"))?;
    let spek = wav.spec();
    let kanal = usize::from(spek.channels);
    if kanal == 0 {
        return Err("WAV tanpa kanal audio".to_owned());
    }
    if !(RATE_MIN..=RATE_MAX).contains(&spek.sample_rate) {
        return Err(format!(
            "sample rate {} Hz tidak didukung (8–192 kHz)",
            spek.sample_rate
        ));
    }
    let frames = wav.duration();
    if frames == 0 {
        return Err("WAV kosong".to_owned());
    }
    if frames / spek.sample_rate > MAX_SECONDS {
        return Err(format!(
            "terlalu panjang: {} detik (maksimal {MAX_SECONDS})",
            frames / spek.sample_rate
        ));
    }

    // WAV 16-bit 1–2 kanal dibaca langsung sebagai int16 (format penyimpanan
    // mesin), jadi berkas proyek yang dibuka lagi sama persis bit demi bit.
    if spek.sample_format == hound::SampleFormat::Int && spek.bits_per_sample == 16 && kanal <= 2 {
        let data: Vec<i16> = wav
            .samples::<i16>()
            .collect::<Result<_, _>>()
            .map_err(|e| galat_baca(&e))?;
        return Ok(Sampel::from_i16(data, kanal, spek.sample_rate));
    }

    let mentah: Vec<f32> = match (spek.sample_format, spek.bits_per_sample) {
        (hound::SampleFormat::Float, 32) => wav
            .samples::<f32>()
            .collect::<Result<_, _>>()
            .map_err(|e| galat_baca(&e))?,
        (hound::SampleFormat::Int, bit @ 8..=32) => {
            #[allow(clippy::cast_precision_loss)]
            let skala = 1.0 / (1_u64 << (bit - 1)) as f32;
            #[allow(clippy::cast_precision_loss)]
            wav.samples::<i32>()
                .map(|s| s.map(|v| v as f32 * skala))
                .collect::<Result<_, _>>()
                .map_err(|e| galat_baca(&e))?
        }
        (format, bit) => {
            return Err(format!("format WAV {format:?} {bit}-bit tidak didukung"));
        }
    };

    let keluar = kanal.min(2);
    let data: Vec<f32> = if kanal <= 2 {
        mentah
    } else {
        mentah
            .chunks_exact(kanal)
            .flat_map(|f| [f[0], f[1]])
            .collect()
    };
    Ok(Sampel::from_f32(&data, keluar, spek.sample_rate))
}

fn galat_baca(e: &hound::Error) -> String {
    format!("WAV rusak atau terpotong ({e})")
}

/// Nama pad dari nama berkas: tanpa ekstensi, huruf besar, maksimal 8
/// karakter ASCII (untuk layar segmen).
#[must_use]
pub fn pad_name_from_file(nama_berkas: &str) -> String {
    let dasar = nama_berkas
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(nama_berkas);
    let dasar = dasar.rsplit_once('.').map_or(dasar, |(d, _)| d);
    let nama: String = dasar
        .chars()
        .map(|c| c.to_ascii_uppercase())
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .take(8)
        .collect();
    if nama.trim_matches('-').is_empty() {
        "SAMPEL".to_owned()
    } else {
        nama
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_SECONDS, decode_wav, pad_name_from_file};

    fn wav(
        spek: hound::WavSpec,
        tulis: impl FnOnce(&mut hound::WavWriter<&mut std::io::Cursor<Vec<u8>>>),
    ) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = hound::WavWriter::new(&mut buf, spek).unwrap();
            tulis(&mut w);
            w.finalize().unwrap();
        }
        buf.into_inner()
    }

    fn spek(channels: u16, bits: u16, format: hound::SampleFormat) -> hound::WavSpec {
        hound::WavSpec {
            channels,
            sample_rate: 44_100,
            bits_per_sample: bits,
            sample_format: format,
        }
    }

    #[test]
    fn pcm16_mono() {
        let b = wav(spek(1, 16, hound::SampleFormat::Int), |w| {
            for v in [0_i16, 16_384, -32_768, 32_767] {
                w.write_sample(v).unwrap();
            }
        });
        let s = decode_wav(&b).unwrap();
        assert_eq!((s.frames(), s.channels(), s.sample_rate()), (4, 1, 44_100));
        assert!((s.frame(1).0 - 0.5).abs() < 1e-3);
        assert!((s.frame(2).0 + 1.0).abs() < 1e-3);
    }

    #[test]
    fn pcm24_dan_float_stereo() {
        let b = wav(spek(2, 24, hound::SampleFormat::Int), |w| {
            w.write_sample(4_194_304_i32).unwrap(); // +0,5
            w.write_sample(-4_194_304_i32).unwrap(); // −0,5
        });
        let s = decode_wav(&b).unwrap();
        let (l, r) = s.frame(0);
        assert!((l - 0.5).abs() < 1e-3 && (r + 0.5).abs() < 1e-3);

        let b = wav(spek(2, 32, hound::SampleFormat::Float), |w| {
            w.write_sample(0.25_f32).unwrap();
            w.write_sample(-0.75_f32).unwrap();
        });
        let (l, r) = decode_wav(&b).unwrap().frame(0);
        assert!((l - 0.25).abs() < 1e-3 && (r + 0.75).abs() < 1e-3);
    }

    #[test]
    fn pcm8_dan_banyak_kanal() {
        let b = wav(spek(1, 8, hound::SampleFormat::Int), |w| {
            w.write_sample(64_i8).unwrap();
        });
        assert!((decode_wav(&b).unwrap().frame(0).0 - 0.5).abs() < 1e-2);

        let b = wav(spek(4, 16, hound::SampleFormat::Int), |w| {
            for v in [100_i16, 200, 300, 400] {
                w.write_sample(v).unwrap();
            }
        });
        let s = decode_wav(&b).unwrap();
        assert_eq!(s.channels(), 2, "hanya dua kanal pertama");
        assert!((s.frame(0).1 - 200.0 / 32_768.0).abs() < 1e-4);
    }

    #[test]
    fn ditolak_dengan_pesan_jelas() {
        assert!(
            decode_wav(b"bukan wav")
                .unwrap_err()
                .contains("bukan berkas WAV")
        );
        assert!(decode_wav(&[]).is_err());

        let kosong = wav(spek(1, 16, hound::SampleFormat::Int), |_| {});
        assert!(decode_wav(&kosong).unwrap_err().contains("kosong"));

        let mut terpotong = wav(spek(1, 16, hound::SampleFormat::Int), |w| {
            for _ in 0..100 {
                w.write_sample(1_i16).unwrap();
            }
        });
        terpotong.truncate(terpotong.len() - 51);
        assert!(decode_wav(&terpotong).is_err());

        let mut s = spek(1, 16, hound::SampleFormat::Int);
        s.sample_rate = 4_000;
        let lambat = wav(s, |w| w.write_sample(1_i16).unwrap());
        assert!(decode_wav(&lambat).unwrap_err().contains("4000 Hz"));
    }

    #[test]
    fn terlalu_panjang_ditolak() {
        let mut s = spek(1, 8, hound::SampleFormat::Int);
        s.sample_rate = 8_000;
        let detik = MAX_SECONDS + 1;
        let b = wav(s, |w| {
            for _ in 0..8_000 * detik {
                w.write_sample(0_i8).unwrap();
            }
        });
        assert!(decode_wav(&b).unwrap_err().contains("terlalu panjang"));
    }

    #[test]
    fn nama_pad_dari_berkas() {
        assert_eq!(pad_name_from_file("kick_tape.wav"), "KICK-TAP");
        assert_eq!(pad_name_from_file("/sdcard/Snare 2.WAV"), "SNARE-2");
        assert_eq!(pad_name_from_file("vox"), "VOX");
        assert_eq!(pad_name_from_file("ギター.wav"), "SAMPEL");
    }
}
