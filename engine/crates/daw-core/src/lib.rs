//! Inti mesin MXM-DAW.
//!
//! Crate ini nantinya berisi model proyek, sequencer, voices, DSP, dan FX
//! (rencana §8.3). Aturannya: **tanpa I/O**. Berkas, perangkat audio, dan
//! Flutter adalah urusan crate lain, sehingga semua yang ada di sini bisa dites
//! di mesin mana pun tanpa HP (ADR-0001, ADR-0002).
//!
//! M0: versi mesin. M1: mesin real-time ([`engine`]). M2: sampler 16 pad
//! ([`pad`], [`sampel`], model di [`bank`], DSP di [`dsp`]) dan kit pabrik
//! sintetis ([`kit`]). M3: sequencer ([`seq`]) yang dijadwalkan tepat-sampel
//! di [`engine`].

pub mod bank;
pub mod dsp;
pub mod engine;
pub mod kit;
pub mod pad;
pub mod sampel;
pub mod seq;
mod suara;

/// Versi mesin, diambil dari `version` di `engine/Cargo.toml` saat kompilasi.
///
/// Nilainya ditanam ke biner oleh compiler, jadi tidak ada I/O saat
/// dipanggil dan aman dipanggil dari thread mana pun.
#[must_use]
pub fn engine_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::engine_version;

    #[test]
    fn versi_mesin_sama_dengan_versi_paket() {
        assert_eq!(engine_version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn versi_mesin_berbentuk_semver() {
        let bagian: Vec<&str> = engine_version().split('.').collect();
        assert_eq!(bagian.len(), 3, "harus MAJOR.MINOR.PATCH: {bagian:?}");
        for angka in bagian {
            assert!(
                angka.parse::<u32>().is_ok(),
                "bagian versi bukan angka: {angka:?}"
            );
        }
    }
}
