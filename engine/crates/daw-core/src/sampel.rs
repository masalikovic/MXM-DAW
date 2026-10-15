//! Data audio satu sampel di memori (rencana §9: int16 di RAM, dikonversi saat
//! diputar, supaya 128 MB cukup untuk ±11 menit stereo).

/// Sampel mono atau stereo, 16-bit, tidak berubah setelah dibuat.
///
/// Dibagikan ke thread audio lewat `Arc`; pembebasannya selalu di thread UI
/// (lihat ADR-0014).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sampel {
    data: Box<[i16]>,
    kanal: usize,
    sample_rate: u32,
}

const SKALA: f32 = 1.0 / 32_768.0;

impl Sampel {
    /// Dari float interleaved (`kanal` 1 atau 2). Nilai di luar [−1, 1] dipotong.
    ///
    /// # Panics
    /// Bila `kanal` bukan 1 atau 2 — kesalahan pemanggil, bukan data pengguna.
    #[must_use]
    pub fn from_f32(sampel: &[f32], kanal: usize, sample_rate: u32) -> Self {
        assert!(kanal == 1 || kanal == 2, "kanal harus 1 atau 2: {kanal}");
        #[allow(clippy::cast_possible_truncation)]
        let data = sampel
            .iter()
            .map(|&x| (x.clamp(-1.0, 1.0) * 32_767.0).round() as i16)
            .collect();
        Self {
            data,
            kanal,
            sample_rate,
        }
    }

    /// Dari int16 interleaved apa adanya (mis. WAV 16-bit dari berkas proyek),
    /// supaya simpan → buka menghasilkan bit yang sama.
    ///
    /// # Panics
    /// Bila `kanal` bukan 1 atau 2.
    #[must_use]
    pub fn from_i16(data: Vec<i16>, kanal: usize, sample_rate: u32) -> Self {
        assert!(kanal == 1 || kanal == 2, "kanal harus 1 atau 2: {kanal}");
        Self {
            data: data.into_boxed_slice(),
            kanal,
            sample_rate,
        }
    }

    /// Data int16 interleaved, untuk ditulis ke berkas.
    #[must_use]
    pub fn as_i16(&self) -> &[i16] {
        &self.data
    }

    /// Jumlah frame.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.data.len() / self.kanal
    }

    /// 1 (mono) atau 2 (stereo).
    #[must_use]
    pub fn channels(&self) -> usize {
        self.kanal
    }

    /// Sample rate asli sampel.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Frame ke-`i` sebagai (kiri, kanan); di luar batas dijepit ke tepi.
    /// Sampel mono mengembalikan nilai yang sama di kedua kanal.
    #[inline]
    #[must_use]
    pub fn frame(&self, i: usize) -> (f32, f32) {
        let n = self.frames();
        if n == 0 {
            return (0.0, 0.0);
        }
        let i = i.min(n - 1) * self.kanal;
        let l = f32::from(self.data[i]) * SKALA;
        if self.kanal == 2 {
            (l, f32::from(self.data[i + 1]) * SKALA)
        } else {
            (l, l)
        }
    }

    /// Ringkasan bentuk gelombang: `titik` pasangan (min, max) mono, untuk
    /// digambar di layar. Dipanggil di thread UI.
    #[must_use]
    pub fn peaks(&self, titik: usize) -> Vec<(f32, f32)> {
        let n = self.frames();
        if n == 0 || titik == 0 {
            return Vec::new();
        }
        (0..titik)
            .map(|t| {
                let awal = t * n / titik;
                let akhir = ((t + 1) * n / titik).max(awal + 1).min(n);
                (awal..akhir).fold((0.0_f32, 0.0_f32), |(lo, hi), i| {
                    let (l, r) = self.frame(i);
                    let m = (l + r) * 0.5;
                    (lo.min(m), hi.max(m))
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::Sampel;

    #[test]
    fn mono_dan_stereo() {
        let m = Sampel::from_f32(&[0.5, -0.5, 2.0], 1, 48_000);
        assert_eq!((m.frames(), m.channels()), (3, 1));
        let (l, r) = m.frame(0);
        assert!((l - 0.5).abs() < 1e-3 && (l - r).abs() < f32::EPSILON);
        assert!((m.frame(2).0 - 1.0).abs() < 1e-3, "dipotong ke 1");
        assert_eq!(m.frame(99), m.frame(2), "di luar batas dijepit");

        let s = Sampel::from_f32(&[0.25, -0.25, 0.5, -0.5], 2, 44_100);
        assert_eq!((s.frames(), s.sample_rate()), (2, 44_100));
        let (l, r) = s.frame(1);
        assert!((l - 0.5).abs() < 1e-3 && (r + 0.5).abs() < 1e-3);
    }

    #[test]
    fn puncak_untuk_gambar() {
        let s = Sampel::from_f32(&[0.0, 1.0, -1.0, 0.0], 1, 48_000);
        let p = s.peaks(2);
        assert_eq!(p.len(), 2);
        assert!(p[0].1 > 0.99 && p[1].0 < -0.99);
        assert!(Sampel::from_f32(&[], 1, 48_000).peaks(4).is_empty());
    }
}
