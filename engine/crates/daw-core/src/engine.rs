//! Mesin real-time M1: menerima event dari UI lewat antrean bebas-lock dan
//! mengisi buffer audio (ADR-0013).
//!
//! Pembagian peran:
//! - [`EventSender`] dipegang thread UI (lewat FFI). Mengirim event tidak
//!   pernah menunggu; bila antrean penuh, event dibuang dan dihitung.
//! - [`Engine`] dipegang thread audio. [`Engine::render`] tidak mengalokasi,
//!   tidak mengunci, dan tidak melakukan I/O (rencana §8.4); tes
//!   `tests/tanpa_alokasi.rs` memaksakan aturan ini.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::kick::{KickParams, KickVoice};

/// Jumlah suara yang bisa berbunyi bersamaan (M1: kick saja).
pub const MAX_VOICES: usize = 8;
/// Kapasitas antrean UI → audio. Jauh di atas jumlah ketukan per blok audio.
pub const EVENT_CAPACITY: usize = 256;
/// Penguatan akhir supaya beberapa kick bertumpuk tidak langsung clipping.
const HEADROOM: f32 = 0.8;

/// Event dari UI ke thread audio.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Pad ditekan dengan kekuatan 0..=1.
    Hit {
        /// Kekuatan ketukan (velocity).
        velocity: f32,
    },
}

/// Angka yang ditulis thread audio dan dibaca UI tanpa lock (atomik).
#[derive(Debug, Default)]
pub struct EngineStats {
    voices_active: AtomicU32,
    hits_played: AtomicU32,
    hits_dropped: AtomicU32,
}

impl EngineStats {
    /// Suara yang berbunyi di akhir blok terakhir.
    #[must_use]
    pub fn voices_active(&self) -> u32 {
        self.voices_active.load(Ordering::Relaxed)
    }

    /// Ketukan yang sudah dimainkan thread audio.
    #[must_use]
    pub fn hits_played(&self) -> u32 {
        self.hits_played.load(Ordering::Relaxed)
    }

    /// Ketukan yang dibuang karena antrean penuh (seharusnya selalu 0).
    #[must_use]
    pub fn hits_dropped(&self) -> u32 {
        self.hits_dropped.load(Ordering::Relaxed)
    }
}

/// Sisi UI: mengirim event ke thread audio.
pub struct EventSender {
    produsen: rtrb::Producer<Event>,
    stats: Arc<EngineStats>,
}

impl EventSender {
    /// Kirim satu ketukan. `false` bila antrean penuh (event dibuang).
    pub fn hit(&mut self, velocity: f32) -> bool {
        let ok = self.produsen.push(Event::Hit { velocity }).is_ok();
        if !ok {
            self.stats.hits_dropped.fetch_add(1, Ordering::Relaxed);
        }
        ok
    }

    /// Statistik bersama dengan thread audio.
    #[must_use]
    pub fn stats(&self) -> &Arc<EngineStats> {
        &self.stats
    }
}

/// Sisi thread audio: mengisi buffer.
pub struct Engine {
    konsumen: rtrb::Consumer<Event>,
    suara: [KickVoice; MAX_VOICES],
    params: KickParams,
    stats: Arc<EngineStats>,
}

impl Engine {
    /// Mesin baru untuk `sample_rate`, beserta pengirim event untuk UI.
    ///
    /// Semua alokasi terjadi di sini, bukan di thread audio.
    #[must_use]
    pub fn new(sample_rate: u32) -> (Self, EventSender) {
        let (produsen, konsumen) = rtrb::RingBuffer::new(EVENT_CAPACITY);
        let stats = Arc::new(EngineStats::default());
        (
            Self {
                konsumen,
                suara: [KickVoice::default(); MAX_VOICES],
                params: KickParams::new(sample_rate),
                stats: Arc::clone(&stats),
            },
            EventSender { produsen, stats },
        )
    }

    /// Statistik bersama dengan UI.
    #[must_use]
    pub fn stats(&self) -> &Arc<EngineStats> {
        &self.stats
    }

    /// Isi `out` (sampel interleaved, `channels` kanal) dengan audio.
    ///
    /// Event yang tiba sebelum blok ini dimulai dibunyikan di awal blok.
    /// Penempatan tepat-sampel datang bersama sequencer di M3.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        let channels = channels.max(1);
        while let Ok(event) = self.konsumen.pop() {
            match event {
                Event::Hit { velocity } => {
                    self.pilih_suara().trigger(velocity);
                    self.stats.hits_played.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        for frame in out.chunks_exact_mut(channels) {
            let mut campuran = 0.0;
            for suara in &mut self.suara {
                campuran += suara.next_sample(&self.params);
            }
            let sampel = (campuran * HEADROOM).clamp(-1.0, 1.0);
            frame.fill(sampel);
        }

        let aktif = self.suara.iter().filter(|s| s.is_active()).count();
        #[allow(clippy::cast_possible_truncation)]
        self.stats
            .voices_active
            .store(aktif as u32, Ordering::Relaxed);
    }

    /// Suara yang diam, atau yang tertua bila semuanya berbunyi.
    fn pilih_suara(&mut self) -> &mut KickVoice {
        let indeks = self
            .suara
            .iter()
            .position(|s| !s.is_active())
            .unwrap_or_else(|| {
                let mut tertua = 0;
                for (i, s) in self.suara.iter().enumerate() {
                    if s.age() > self.suara[tertua].age() {
                        tertua = i;
                    }
                }
                tertua
            });
        &mut self.suara[indeks]
    }
}

#[cfg(test)]
mod tests {
    use super::{EVENT_CAPACITY, Engine, MAX_VOICES};

    const SR: u32 = 48_000;

    fn render_detik(engine: &mut Engine, detik: f32, channels: usize) -> Vec<f32> {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frames = (detik * 48_000.0) as usize;
        let mut buf = vec![0.0; frames * channels];
        for blok in buf.chunks_mut(192 * channels) {
            engine.render(blok, channels);
        }
        buf
    }

    fn puncak(x: &[f32]) -> f32 {
        x.iter().fold(0.0_f32, |m, v| m.max(v.abs()))
    }

    #[allow(clippy::cast_precision_loss)]
    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    #[test]
    fn tanpa_ketukan_hening() {
        let (mut mesin, _kirim) = Engine::new(SR);
        assert!(render_detik(&mut mesin, 0.5, 2).iter().all(|&v| v == 0.0));
    }

    #[test]
    fn ketukan_berbunyi_tanpa_klik_di_awal() {
        let (mut mesin, mut kirim) = Engine::new(SR);
        assert!(kirim.hit(1.0));
        let x = render_detik(&mut mesin, 0.05, 1);
        // Tanpa rampa 1 ms, sampel ke-10 sudah ±0,14; dengan rampa ±0,03.
        // (Sampel pertama selalu 0 karena sin(0), jadi tidak cukup diperiksa.)
        let awal = puncak(&x[..10]);
        assert!(awal < 0.05, "10 sampel pertama memuncak di {awal}");
        assert!(puncak(&x) > 0.5, "puncak {}", puncak(&x));
        assert!(puncak(&x) <= 1.0);
    }

    #[test]
    fn volume_meluruh() {
        let (mut mesin, mut kirim) = Engine::new(SR);
        kirim.hit(1.0);
        let x = render_detik(&mut mesin, 1.2, 1);
        let awal = rms(&x[..4_800]);
        let akhir = rms(&x[48_000..52_800]);
        assert!(akhir < awal / 100.0, "awal {awal}, akhir {akhir}");
    }

    #[test]
    fn nada_turun() {
        // Jarak antar zero-crossing (setengah periode) di awal lebih pendek
        // daripada di akhir, karena nada turun dari 150 Hz ke 48 Hz.
        let (mut mesin, mut kirim) = Engine::new(SR);
        kirim.hit(1.0);
        let x = render_detik(&mut mesin, 0.3, 1);
        let silang: Vec<usize> = (1..x.len())
            .filter(|&i| x[i - 1] <= 0.0 && x[i] > 0.0)
            .collect();
        let periode_awal = silang[1] - silang[0];
        let periode_akhir = silang[silang.len() - 1] - silang[silang.len() - 2];
        assert!(
            periode_akhir > periode_awal + 100,
            "{periode_awal} → {periode_akhir}"
        );
    }

    #[test]
    fn velocity_mengatur_kekerasan() {
        let puncak_dengan = |v: f32| {
            let (mut mesin, mut kirim) = Engine::new(SR);
            kirim.hit(v);
            puncak(&render_detik(&mut mesin, 0.05, 1))
        };
        let rasio = puncak_dengan(0.5) / puncak_dengan(1.0);
        assert!((rasio - 0.5).abs() < 0.01, "rasio {rasio}");
    }

    #[test]
    fn stereo_kedua_kanal_sama() {
        let (mut mesin, mut kirim) = Engine::new(SR);
        kirim.hit(1.0);
        let x = render_detik(&mut mesin, 0.05, 2);
        for frame in x.chunks_exact(2) {
            assert_eq!(frame[0].to_bits(), frame[1].to_bits());
        }
    }

    #[test]
    fn deterministik() {
        let jalankan = || {
            let (mut mesin, mut kirim) = Engine::new(SR);
            kirim.hit(0.8);
            render_detik(&mut mesin, 0.2, 2)
        };
        let (a, b) = (jalankan(), jalankan());
        assert!(a.iter().zip(&b).all(|(x, y)| x.to_bits() == y.to_bits()));
    }

    #[test]
    fn ketukan_beruntun_tidak_melebihi_batas_suara() {
        let (mut mesin, mut kirim) = Engine::new(SR);
        let mut blok = vec![0.0; 192 * 2];
        for _ in 0..40 {
            kirim.hit(1.0);
            mesin.render(&mut blok, 2);
            assert!(blok.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
        }
        assert_eq!(mesin.stats().hits_played(), 40);
        assert!(mesin.stats().voices_active() as usize <= MAX_VOICES);
    }

    #[test]
    fn antrean_penuh_dibuang_dan_dihitung() {
        let (_mesin, mut kirim) = Engine::new(SR);
        for _ in 0..EVENT_CAPACITY {
            assert!(kirim.hit(1.0));
        }
        assert!(!kirim.hit(1.0));
        assert_eq!(kirim.stats().hits_dropped(), 1);
    }
}
