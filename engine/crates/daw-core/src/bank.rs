//! Model 16 pad: parameter, sampel, dan nama (ADR-0002, ADR-0014).
//!
//! `PadBank` hidup lebih lama daripada stream audio. Saat aplikasi ke latar,
//! stream dan mesin dibebaskan, tetapi bank disimpan; saat kembali, mesin baru
//! dibuat dari bank yang sama, jadi suntingan dan sampel impor tidak hilang.

use std::sync::Arc;

use crate::kit::factory_kit;
use crate::pad::{PAD_COUNT, PadParams};
use crate::sampel::Sampel;

/// Isi 16 pad satu grup.
#[derive(Debug, Clone)]
pub struct PadBank {
    params: [PadParams; PAD_COUNT],
    samples: [Option<Arc<Sampel>>; PAD_COUNT],
    names: [String; PAD_COUNT],
}

impl PadBank {
    /// Bank berisi kit pabrik. Mensintesis 16 suara (beberapa milidetik).
    #[must_use]
    pub fn factory() -> Self {
        let kit = factory_kit();
        Self {
            params: core::array::from_fn(|i| kit[i].params),
            samples: core::array::from_fn(|i| Some(Arc::new(kit[i].sample.clone()))),
            names: core::array::from_fn(|i| kit[i].name.to_owned()),
        }
    }

    /// Parameter pad.
    #[must_use]
    pub fn params(&self, pad: usize) -> Option<&PadParams> {
        self.params.get(pad)
    }

    /// Sampel pad (bisa kosong).
    #[must_use]
    pub fn sample(&self, pad: usize) -> Option<&Arc<Sampel>> {
        self.samples.get(pad).and_then(Option::as_ref)
    }

    /// Nama suara pad.
    #[must_use]
    pub fn name(&self, pad: usize) -> Option<&str> {
        self.names.get(pad).map(String::as_str)
    }

    /// Simpan parameter (disanitasi). Mengembalikan nilai yang tersimpan, atau
    /// `None` bila nomor pad salah.
    pub fn set_params(&mut self, pad: usize, params: &PadParams) -> Option<PadParams> {
        let tujuan = self.params.get_mut(pad)?;
        *tujuan = params.sanitized();
        Some(*tujuan)
    }

    /// Ganti sampel dan nama pad; `start` = 0 dan `end` = panjang sampel.
    /// Mengembalikan parameter baru, atau `None` bila nomor pad salah.
    pub fn set_sample(&mut self, pad: usize, name: &str, sample: Arc<Sampel>) -> Option<PadParams> {
        let params = PadParams {
            start: 0,
            end: u32::try_from(sample.frames()).unwrap_or(u32::MAX),
            ..*self.params.get(pad)?
        };
        self.samples[pad] = Some(sample);
        name.clone_into(&mut self.names[pad]);
        self.set_params(pad, &params)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::PadBank;
    use crate::pad::PadParams;
    use crate::sampel::Sampel;

    #[test]
    fn pabrik_berisi_kit() {
        let b = PadBank::factory();
        assert_eq!(b.name(0), Some("KICK"));
        assert!(b.sample(15).is_some());
        assert!(b.params(16).is_none());
    }

    #[test]
    fn simpan_parameter_disanitasi() {
        let mut b = PadBank::factory();
        let p = PadParams {
            level: 9.0,
            ..PadParams::default()
        };
        assert!((b.set_params(2, &p).unwrap().level - 1.0).abs() < f32::EPSILON);
        assert!(b.set_params(16, &p).is_none());
    }

    #[test]
    fn ganti_sampel_mengatur_start_end() {
        let mut b = PadBank::factory();
        let s = Arc::new(Sampel::from_f32(&[0.0; 100], 1, 44_100));
        let p = b.set_sample(1, "PUNYAKU", s).unwrap();
        assert_eq!((p.start, p.end), (0, 100));
        assert_eq!(b.name(1), Some("PUNYAKU"));
    }
}
