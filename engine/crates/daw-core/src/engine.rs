//! Mesin real-time: menerima event dari UI lewat antrean bebas-lock dan
//! mengisi buffer audio (ADR-0013, ADR-0014).
//!
//! Pembagian peran:
//! - [`EventSender`] dipegang thread UI (lewat FFI). Ia juga memegang salinan
//!   parameter semua pad, jadi UI bisa membacanya tanpa bertanya ke thread
//!   audio (ADR-0002: model ada di Rust).
//! - [`Engine`] dipegang thread audio. [`Engine::render`] tidak mengalokasi,
//!   tidak mengunci, tidak melakukan I/O, dan **tidak membebaskan** memori
//!   (rencana §8.4); tes `tests/tanpa_alokasi.rs` memaksakan aturan ini.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::bank::PadBank;
use crate::pad::{PAD_COUNT, PadParams};
use crate::sampel::Sampel;
use crate::suara::Suara;

/// Polifoni (rencana §9).
pub const MAX_VOICES: usize = 32;
/// Kapasitas antrean UI → audio. Jauh di atas jumlah ketukan per blok audio.
pub const EVENT_CAPACITY: usize = 256;
/// Penguatan akhir supaya beberapa suara bertumpuk tidak langsung clipping.
const HEADROOM: f32 = 0.8;
/// Mesin memproses paling banyak sekian frame sekaligus (buffer campur tetap).
const BLOK: usize = 256;

/// Event dari UI ke thread audio.
#[derive(Clone, Debug)]
pub enum Event {
    /// Pad ditekan dengan kekuatan 0..=1.
    Hit {
        /// Nomor pad 0..16.
        pad: u8,
        /// Kekuatan ketukan (velocity).
        velocity: f32,
    },
    /// Pad dilepas (penting untuk mode gate dan loop).
    Release {
        /// Nomor pad.
        pad: u8,
    },
    /// Parameter pad berubah.
    SetPad {
        /// Nomor pad.
        pad: u8,
        /// Parameter baru (sudah disanitasi oleh pengirim).
        params: PadParams,
    },
    /// Sampel pad diganti. Sampel lama dikembalikan ke UI untuk dibebaskan.
    SetSample {
        /// Nomor pad.
        pad: u8,
        /// Sampel baru.
        sample: Arc<Sampel>,
    },
}

/// Angka yang ditulis thread audio dan dibaca UI tanpa lock (atomik).
#[derive(Debug, Default)]
pub struct EngineStats {
    voices_active: AtomicU32,
    hits_played: AtomicU32,
    hits_dropped: AtomicU32,
    pads_sounding: AtomicU32,
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

    /// Bit ke-`n` menyala bila pad `n` sedang berbunyi (untuk LED pad).
    #[must_use]
    pub fn pads_sounding(&self) -> u32 {
        self.pads_sounding.load(Ordering::Relaxed)
    }
}

/// Sisi UI: mengirim event ke thread audio dan memegang model pad.
pub struct EventSender {
    produsen: rtrb::Producer<Event>,
    sampah: rtrb::Consumer<Arc<Sampel>>,
    stats: Arc<EngineStats>,
    bank: PadBank,
}

impl EventSender {
    fn kirim(&mut self, event: Event) -> bool {
        self.empty_trash();
        self.produsen.push(event).is_ok()
    }

    /// Kirim satu ketukan. `false` bila pad tidak ada atau antrean penuh.
    pub fn hit(&mut self, pad: usize, velocity: f32) -> bool {
        let Ok(pad) = u8::try_from(pad) else {
            return false;
        };
        if usize::from(pad) >= PAD_COUNT {
            return false;
        }
        let velocity = if velocity.is_nan() {
            0.0
        } else {
            velocity.clamp(0.0, 1.0)
        };
        let ok = self.kirim(Event::Hit { pad, velocity });
        if !ok {
            self.stats.hits_dropped.fetch_add(1, Ordering::Relaxed);
        }
        ok
    }

    /// Pad dilepas.
    pub fn release(&mut self, pad: usize) -> bool {
        match u8::try_from(pad) {
            Ok(p) if pad < PAD_COUNT => self.kirim(Event::Release { pad: p }),
            _ => false,
        }
    }

    /// Ganti parameter pad. Nilai disanitasi dulu; model hanya berubah bila
    /// event berhasil masuk antrean.
    pub fn set_pad(&mut self, pad: usize, params: &PadParams) -> bool {
        let Some(p) = u8::try_from(pad).ok().filter(|_| pad < PAD_COUNT) else {
            return false;
        };
        let params = params.sanitized();
        let ok = self.kirim(Event::SetPad { pad: p, params });
        if ok {
            self.bank.set_params(pad, &params);
        }
        ok
    }

    /// Ganti sampel pad (mis. hasil impor). `start` pad menjadi 0 dan `end`
    /// panjang sampel.
    pub fn set_sample(&mut self, pad: usize, name: &str, sample: Sampel) -> bool {
        let Some(p) = u8::try_from(pad).ok().filter(|_| pad < PAD_COUNT) else {
            return false;
        };
        let sample = Arc::new(sample);
        if !self.kirim(Event::SetSample {
            pad: p,
            sample: Arc::clone(&sample),
        }) {
            return false;
        }
        let params = self.bank.set_sample(pad, name, sample);
        params.is_some_and(|params| self.kirim(Event::SetPad { pad: p, params }))
    }

    /// Parameter pad saat ini.
    #[must_use]
    pub fn pad(&self, pad: usize) -> Option<&PadParams> {
        self.bank.params(pad)
    }

    /// Sampel pad saat ini.
    #[must_use]
    pub fn sample(&self, pad: usize) -> Option<&Arc<Sampel>> {
        self.bank.sample(pad)
    }

    /// Nama suara pad.
    #[must_use]
    pub fn name(&self, pad: usize) -> Option<&str> {
        self.bank.name(pad)
    }

    /// Model 16 pad.
    #[must_use]
    pub fn bank(&self) -> &PadBank {
        &self.bank
    }

    /// Ambil model untuk disimpan saat mesin dibebaskan. Sampel lama yang
    /// masih di antrean sampah ikut dibebaskan di sini (thread UI).
    #[must_use]
    pub fn into_bank(mut self) -> PadBank {
        self.empty_trash();
        self.bank
    }

    /// Bebaskan sampel lama yang dikembalikan thread audio. Dipanggil
    /// otomatis setiap kali mengirim event.
    pub fn empty_trash(&mut self) -> usize {
        let mut n = 0;
        while self.sampah.pop().is_ok() {
            n += 1;
        }
        n
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
    sampah: rtrb::Producer<Arc<Sampel>>,
    suara: Box<[Suara; MAX_VOICES]>,
    pads: [PadParams; PAD_COUNT],
    sampel: [Option<Arc<Sampel>>; PAD_COUNT],
    sr: f32,
    campur: Box<[f32; BLOK * 2]>,
    stats: Arc<EngineStats>,
}

impl Engine {
    /// Mesin baru untuk `sample_rate` dengan kit pabrik terpasang, beserta
    /// pengirim event untuk UI.
    #[must_use]
    pub fn new(sample_rate: u32) -> (Self, EventSender) {
        Self::with_bank(sample_rate, PadBank::factory())
    }

    /// Mesin baru yang memainkan isi `bank` (mis. bank yang disimpan saat
    /// aplikasi ke latar).
    ///
    /// Semua alokasi terjadi di sini, bukan di thread audio.
    #[must_use]
    pub fn with_bank(sample_rate: u32, bank: PadBank) -> (Self, EventSender) {
        let (produsen, konsumen) = rtrb::RingBuffer::new(EVENT_CAPACITY);
        // Setiap SetSample mengembalikan paling banyak satu sampel lama.
        let (sampah_tulis, sampah_baca) = rtrb::RingBuffer::new(EVENT_CAPACITY);
        let stats = Arc::new(EngineStats::default());
        let pads = core::array::from_fn(|i| bank.params(i).copied().unwrap_or_default());
        let sampel = core::array::from_fn(|i| bank.sample(i).cloned());
        #[allow(clippy::cast_precision_loss)]
        let sr = sample_rate.max(1) as f32;
        (
            Self {
                konsumen,
                sampah: sampah_tulis,
                suara: Box::new([Suara::default(); MAX_VOICES]),
                pads,
                sampel,
                sr,
                campur: Box::new([0.0; BLOK * 2]),
                stats: Arc::clone(&stats),
            },
            EventSender {
                produsen,
                sampah: sampah_baca,
                stats,
                bank,
            },
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
            self.terapkan(event);
        }
        for i in 0..MAX_VOICES {
            let s = &mut self.suara[i];
            if s.aktif()
                && let Some(sampel) = &self.sampel[s.pad()]
            {
                s.siapkan(&self.pads[s.pad()], sampel, self.sr);
            }
        }

        for potongan in out.chunks_mut(BLOK * channels) {
            let frames = potongan.len() / channels;
            let campur = &mut self.campur[..frames * 2];
            campur.fill(0.0);
            for s in self.suara.iter_mut() {
                if !s.terdengar() {
                    continue;
                }
                match &self.sampel[s.pad()] {
                    Some(sampel) => s.tambah_ke(sampel, campur),
                    None => s.potong(),
                }
            }
            for (frame, lr) in potongan
                .chunks_exact_mut(channels)
                .zip(campur.chunks_exact(2))
            {
                let l = (lr[0] * HEADROOM).clamp(-1.0, 1.0);
                let r = (lr[1] * HEADROOM).clamp(-1.0, 1.0);
                match frame {
                    [m] => *m = (l + r) * 0.5,
                    [a, b, sisa @ ..] => {
                        *a = l;
                        *b = r;
                        sisa.fill(0.0);
                    }
                    [] => {}
                }
            }
        }

        let mut aktif = 0;
        let mut bit = 0_u32;
        for s in self.suara.iter() {
            if s.aktif() {
                aktif += 1;
                bit |= 1 << s.pad();
            }
        }
        self.stats.voices_active.store(aktif, Ordering::Relaxed);
        self.stats.pads_sounding.store(bit, Ordering::Relaxed);
    }

    fn terapkan(&mut self, event: Event) {
        match event {
            Event::Hit { pad, velocity } => {
                let pad = usize::from(pad);
                let Some(sampel) = &self.sampel[pad] else {
                    return;
                };
                let params = self.pads[pad];
                if params.choke != 0 {
                    for s in self.suara.iter_mut() {
                        if s.aktif() && s.pad() != pad && self.pads[s.pad()].choke == params.choke {
                            s.cekik();
                        }
                    }
                }
                let i = self.pilih_suara();
                self.suara[i].picu(pad, velocity, &params, sampel, self.sr);
                self.stats.hits_played.fetch_add(1, Ordering::Relaxed);
            }
            Event::Release { pad } => {
                for s in self.suara.iter_mut() {
                    if s.aktif() && s.pad() == usize::from(pad) {
                        s.lepas();
                    }
                }
            }
            Event::SetPad { pad, params } => self.pads[usize::from(pad)] = params,
            Event::SetSample { pad, sample } => {
                let pad = usize::from(pad);
                for s in self.suara.iter_mut() {
                    if s.pad() == pad {
                        s.potong();
                    }
                }
                if let Some(lama) = self.sampel[pad].replace(sample) {
                    // Antrean sampah seukuran antrean event, jadi tidak pernah
                    // penuh selama UI terus mengirim. Bila tetap penuh, lebih
                    // baik bocor daripada membebaskan memori di thread audio.
                    if let Err(rtrb::PushError::Full(lama)) = self.sampah.push(lama) {
                        core::mem::forget(lama);
                    }
                }
            }
        }
    }

    /// Indeks suara yang diam, atau yang tertua bila semuanya berbunyi.
    fn pilih_suara(&self) -> usize {
        if let Some(i) = self.suara.iter().position(|s| !s.aktif()) {
            return i;
        }
        let mut tertua = 0;
        for (i, s) in self.suara.iter().enumerate() {
            if s.umur() > self.suara[tertua].umur() {
                tertua = i;
            }
        }
        tertua
    }
}

#[cfg(test)]
mod tests;
