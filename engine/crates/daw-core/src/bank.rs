//! Model 64 pad (grup A–D × 16): parameter, sampel, dan nama (ADR-0002,
//! ADR-0014). Nomor pad global = grup × 16 + pad.
//!
//! `PadBank` hidup lebih lama daripada stream audio. Saat aplikasi ke latar,
//! stream dan mesin dibebaskan, tetapi bank disimpan; saat kembali, mesin baru
//! dibuat dari bank yang sama, jadi suntingan dan sampel impor tidak hilang.

use std::sync::Arc;

use crate::kit::factory_kit;
use crate::pad::{FilterMode, GROUP_COUNT, PAD_COUNT, PadParams, TOTAL_PADS};
use crate::sampel::Sampel;

/// Asal sampel sebuah pad; menentukan cara menyimpannya di berkas proyek.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleSource {
    /// Tanpa sampel.
    Empty,
    /// Suara kit pabrik ke-`n`; tidak perlu disimpan, cukup dibangkitkan ulang.
    Factory(u8),
    /// Sampel impor dengan nomor unik; disimpan sebagai `samples/s<id>.wav`.
    Imported(u64),
}

/// Isi 64 pad (4 grup).
#[derive(Debug, Clone)]
pub struct PadBank {
    params: [PadParams; TOTAL_PADS],
    samples: [Option<Arc<Sampel>>; TOTAL_PADS],
    names: [String; TOTAL_PADS],
    sources: [SampleSource; TOTAL_PADS],
    next_id: u64,
}

/// Variasi kit pabrik untuk grup B–D: sampel yang sama (tanpa memori
/// tambahan), warna bunyi berbeda, supaya setiap grup langsung bisa dimainkan.
fn variasi(grup: usize, p: PadParams) -> PadParams {
    let mut p = p;
    match grup {
        // B: lebih rendah dan redup.
        1 => {
            p.pitch = -3.0;
            p.filter.mode = FilterMode::LowPass;
            p.filter.cutoff = 0.62;
            p.drive = 0.25;
        }
        // C: lebih tinggi dan tipis.
        2 => {
            p.pitch = 5.0;
            p.filter.mode = FilterMode::HighPass;
            p.filter.cutoff = 0.2;
        }
        // D: satu kuint lebih rendah, kasar.
        3 => {
            p.pitch = -7.0;
            p.drive = 0.6;
        }
        _ => {}
    }
    p
}

impl PadBank {
    /// Bank berisi kit pabrik di grup A dan variasinya di grup B–D.
    /// Mensintesis 16 suara sekali (beberapa milidetik); keempat grup berbagi
    /// sampel yang sama.
    #[must_use]
    pub fn factory() -> Self {
        let kit = factory_kit();
        let sampel: Vec<Arc<Sampel>> = kit.iter().map(|s| Arc::new(s.sample.clone())).collect();
        debug_assert_eq!(GROUP_COUNT * kit.len(), TOTAL_PADS);
        Self {
            params: core::array::from_fn(|i| variasi(i / PAD_COUNT, kit[i % PAD_COUNT].params)),
            samples: core::array::from_fn(|i| Some(Arc::clone(&sampel[i % PAD_COUNT]))),
            names: core::array::from_fn(|i| kit[i % PAD_COUNT].name.to_owned()),
            #[allow(clippy::cast_possible_truncation)]
            sources: core::array::from_fn(|i| SampleSource::Factory((i % PAD_COUNT) as u8)),
            next_id: 1,
        }
    }

    /// Asal sampel pad.
    #[must_use]
    pub fn source(&self, pad: usize) -> Option<SampleSource> {
        self.sources.get(pad).copied()
    }

    /// Pasang isi pad apa adanya (dipakai pemuat berkas proyek). `false` bila
    /// nomor pad salah.
    pub fn restore_pad(
        &mut self,
        pad: usize,
        name: &str,
        params: &PadParams,
        sample: Option<Arc<Sampel>>,
        source: SampleSource,
    ) -> bool {
        if pad >= TOTAL_PADS {
            return false;
        }
        self.params[pad] = params.sanitized();
        name.clone_into(&mut self.names[pad]);
        self.sources[pad] = if sample.is_some() {
            source
        } else {
            SampleSource::Empty
        };
        self.samples[pad] = sample;
        if let SampleSource::Imported(id) = source {
            self.next_id = self.next_id.max(id + 1);
        }
        true
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

    /// Salin isi pad `dari` ke pad `ke`: parameter, sampel (dibagi lewat
    /// `Arc`), nama, dan asalnya. `false` bila nomor pad salah.
    pub fn copy_pad(&mut self, dari: usize, ke: usize) -> bool {
        if dari >= TOTAL_PADS || ke >= TOTAL_PADS {
            return false;
        }
        self.params[ke] = self.params[dari];
        let (sampel, nama) = (self.samples[dari].clone(), self.names[dari].clone());
        self.samples[ke] = sampel;
        self.names[ke] = nama;
        self.sources[ke] = self.sources[dari];
        true
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
        self.sources[pad] = SampleSource::Imported(self.next_id);
        self.next_id += 1;
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
        assert_eq!(b.name(16), Some("KICK"), "grup B");
        assert!((b.params(16).unwrap().pitch + 3.0).abs() < f32::EPSILON);
        assert!(
            Arc::ptr_eq(b.sample(0).unwrap(), b.sample(48).unwrap()),
            "sampel dibagi"
        );
        assert!(b.params(64).is_none());
    }

    #[test]
    fn simpan_parameter_disanitasi() {
        let mut b = PadBank::factory();
        let p = PadParams {
            level: 9.0,
            ..PadParams::default()
        };
        assert!((b.set_params(2, &p).unwrap().level - 1.0).abs() < f32::EPSILON);
        assert!(b.set_params(64, &p).is_none());
    }

    #[test]
    fn ganti_sampel_mengatur_start_end() {
        let mut b = PadBank::factory();
        let s = Arc::new(Sampel::from_f32(&[0.0; 100], 1, 44_100));
        let p = b.set_sample(1, "PUNYAKU", s).unwrap();
        assert_eq!((p.start, p.end), (0, 100));
        assert_eq!(b.name(1), Some("PUNYAKU"));
        assert_eq!(b.source(1), Some(super::SampleSource::Imported(1)));
        assert_eq!(b.source(2), Some(super::SampleSource::Factory(2)));
    }
}
