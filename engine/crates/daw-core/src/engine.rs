//! Mesin real-time: menerima event dari UI lewat antrean bebas-lock dan
//! mengisi buffer audio (ADR-0013, ADR-0014, ADR-0015).
//!
//! Pembagian peran:
//! - [`EventSender`] dipegang thread UI (lewat FFI). Ia memegang **model**:
//!   parameter dan sampel 64 pad (grup A–D), serta pola dan pengaturan
//!   sequencer. UI
//!   membacanya tanpa bertanya ke thread audio (ADR-0002).
//! - [`Engine`] dipegang thread audio. [`Engine::render`] tidak mengalokasi,
//!   tidak mengunci, tidak melakukan I/O, dan **tidak membebaskan** memori
//!   (rencana §8.4); tes `tests/tanpa_alokasi.rs` memaksakan aturan ini.
//!
//! Ada tiga antrean SPSC: event (UI → audio), sampah (audio → UI, untuk
//! sampel/pola lama yang harus dibebaskan di UI), dan rekaman (audio → UI,
//! ketukan yang direkam beserta posisinya).

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, Ordering};

use crate::bank::PadBank;
use crate::pad::{GROUP_COUNT, PAD_COUNT, PadParams, TOTAL_PADS};
use crate::sampel::Sampel;
use crate::seq::{
    Action, CompiledPattern, Note, PPQN, Pattern, STEP_TICKS, SeqSettings, Sequence, TICKS_PER_BAR,
    compile,
};
use crate::suara::Suara;

mod transport;

use transport::{Klik, Transport};

/// Polifoni (rencana §9).
pub const MAX_VOICES: usize = 32;
/// Kapasitas antrean UI → audio. Jauh di atas jumlah ketukan per blok audio.
pub const EVENT_CAPACITY: usize = 256;
/// Penguatan akhir supaya beberapa suara bertumpuk tidak langsung clipping.
const HEADROOM: f32 = 0.8;
/// Mesin memproses paling banyak sekian frame sekaligus (buffer campur tetap).
const BLOK: usize = 256;
/// Aksi terjadwal paling banyak per blok. Kelebihannya dimainkan di akhir
/// blok (tidak hilang), tetapi itu hanya terjadi pada pola yang tidak wajar.
const MAKS_JADWAL: usize = 128;

/// Perintah transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportCommand {
    /// Mulai dari awal pola. Bila `record`, count-in dimainkan dulu.
    Play {
        /// Rekam ketukan pad.
        record: bool,
        /// Bar count-in (0..=2) sebelum rekaman dimulai.
        count_in_bars: u8,
    },
    /// Berhenti; semua pad gate/loop dilepas.
    Stop,
    /// Hidupkan/matikan rekaman tanpa menghentikan pemutaran (overdub).
    Record(bool),
}

/// Event dari UI ke thread audio.
#[derive(Clone, Debug)]
pub enum Event {
    /// Pad ditekan dengan kekuatan 0..=1.
    Hit {
        /// Nomor pad global 0..64.
        pad: u8,
        /// Kekuatan ketukan (velocity).
        velocity: f32,
        /// Transpose (semitone) untuk mode KEYS; 0 = nada asli.
        pitch: i8,
    },
    /// Pad dilepas (penting untuk mode gate dan loop). Hanya suara dengan
    /// transpose yang sama yang dilepas, supaya akor di mode KEYS tidak putus.
    Release {
        /// Nomor pad.
        pad: u8,
        /// Transpose.
        pitch: i8,
    },
    /// Note repeat (rencana §4.1): selama aktif, pad dibunyikan ulang tepat di
    /// setiap kelipatan `rate` tick. `rate` 0 = berhenti.
    Repeat {
        /// Nomor pad.
        pad: u8,
        /// Kekuatan setiap ulangan.
        velocity: f32,
        /// Transpose.
        pitch: i8,
        /// Jarak antar-ulangan (tick; 240 = 1/16), atau 0.
        rate: u32,
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
    /// Pola satu grup diganti. Pola lama dikembalikan ke UI untuk dibebaskan.
    SetPattern {
        /// Grup 0..4.
        group: u8,
        /// Pola terkompilasi (nomor pad global).
        pattern: Arc<CompiledPattern>,
    },
    /// Mute grup: polanya tidak dibunyikan (ketukan langsung tetap berbunyi).
    SetMute {
        /// Grup 0..4.
        group: u8,
        /// Di-mute.
        mute: bool,
    },
    /// Transport.
    Transport(TransportCommand),
    /// Tempo (BPM, sudah disanitasi).
    SetTempo(f32),
    /// Metronom hidup/mati.
    SetMetronome(bool),
}

/// Ketukan yang direkam thread audio, dengan posisi absolutnya.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Recorded {
    /// Pad ditekan pada `tick` (sejak awal pemutaran, ≥ 0).
    On {
        /// Posisi.
        tick: f64,
        /// Nomor pad.
        pad: u8,
        /// Kekuatan.
        velocity: f32,
        /// Transpose.
        pitch: i8,
    },
    /// Pad dilepas pada `tick`.
    Off {
        /// Posisi.
        tick: f64,
        /// Nomor pad.
        pad: u8,
        /// Transpose.
        pitch: i8,
    },
}

/// Memori yang harus dibebaskan di thread UI.
enum Sampah {
    // Isinya tidak pernah dibaca: cukup dibebaskan (drop) di thread UI.
    #[allow(dead_code)]
    Sampel(Arc<Sampel>),
    #[allow(dead_code)]
    Pola(Arc<CompiledPattern>),
}

/// Angka yang ditulis thread audio dan dibaca UI tanpa lock (atomik).
#[derive(Debug, Default)]
pub struct EngineStats {
    voices_active: AtomicU32,
    hits_played: AtomicU32,
    hits_dropped: AtomicU32,
    pads_sounding: AtomicU64,
    posisi_tick: AtomicI64,
    transport: AtomicU32,
}

const BIT_MAIN: u32 = 1;
const BIT_REKAM: u32 = 2;

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

    /// Bit ke-`n` menyala bila pad global `n` sedang berbunyi (untuk LED).
    #[must_use]
    pub fn pads_sounding(&self) -> u64 {
        self.pads_sounding.load(Ordering::Relaxed)
    }

    /// Posisi transport (tick sejak awal; negatif saat count-in).
    #[must_use]
    pub fn position_ticks(&self) -> i64 {
        self.posisi_tick.load(Ordering::Relaxed)
    }

    /// Transport sedang berjalan.
    #[must_use]
    pub fn playing(&self) -> bool {
        self.transport.load(Ordering::Relaxed) & BIT_MAIN != 0
    }

    /// Sedang merekam.
    #[must_use]
    pub fn recording(&self) -> bool {
        self.transport.load(Ordering::Relaxed) & BIT_REKAM != 0
    }
}

/// Sisi UI: mengirim event ke thread audio dan memegang model.
pub struct EventSender {
    produsen: rtrb::Producer<Event>,
    sampah: rtrb::Consumer<Sampah>,
    rekaman: rtrb::Consumer<Recorded>,
    stats: Arc<EngineStats>,
    bank: PadBank,
    seq: Sequence,
    /// Not yang sedang direkam dan menunggu "lepas": (pad global, transpose,
    /// indeks not di pola grupnya).
    terbuka: Vec<(usize, i8, usize)>,
    /// Jumlah perubahan model sejak dibuat (sinyal untuk simpan otomatis).
    perubahan: u64,
}

/// Nomor pad global yang sah sebagai `u8`.
fn nomor_pad(pad: usize) -> Option<u8> {
    u8::try_from(pad).ok().filter(|_| pad < TOTAL_PADS)
}

/// Velocity di dalam 0..=1; NaN menjadi 0.
fn velocity_sah(v: f32) -> f32 {
    if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) }
}

impl EventSender {
    fn kirim(&mut self, event: Event) -> bool {
        self.empty_trash();
        self.produsen.push(event).is_ok()
    }

    /// Kirim satu ketukan. `false` bila pad tidak ada atau antrean penuh.
    pub fn hit(&mut self, pad: usize, velocity: f32) -> bool {
        self.hit_note(pad, velocity, 0)
    }

    /// Ketukan dengan transpose (mode KEYS).
    pub fn hit_note(&mut self, pad: usize, velocity: f32, pitch: i8) -> bool {
        let Some(pad) = nomor_pad(pad) else {
            return false;
        };
        let ok = self.kirim(Event::Hit {
            pad,
            velocity: velocity_sah(velocity),
            pitch,
        });
        if !ok {
            self.stats.hits_dropped.fetch_add(1, Ordering::Relaxed);
        }
        ok
    }

    /// Pad dilepas.
    pub fn release(&mut self, pad: usize) -> bool {
        self.release_note(pad, 0)
    }

    /// Lepas satu nada pad (mode KEYS).
    pub fn release_note(&mut self, pad: usize, pitch: i8) -> bool {
        nomor_pad(pad).is_some_and(|pad| self.kirim(Event::Release { pad, pitch }))
    }

    /// Mulai (`rate` > 0, dalam tick) atau hentikan (`rate` = 0) note repeat
    /// untuk satu nada pad.
    pub fn repeat(&mut self, pad: usize, velocity: f32, pitch: i8, rate: u32) -> bool {
        nomor_pad(pad).is_some_and(|pad| {
            self.kirim(Event::Repeat {
                pad,
                velocity: velocity_sah(velocity),
                pitch,
                rate,
            })
        })
    }

    /// Ganti parameter pad. Nilai disanitasi dulu; model hanya berubah bila
    /// event berhasil masuk antrean.
    pub fn set_pad(&mut self, pad: usize, params: &PadParams) -> bool {
        let Some(p) = u8::try_from(pad).ok().filter(|_| pad < TOTAL_PADS) else {
            return false;
        };
        let params = params.sanitized();
        let ok = self.kirim(Event::SetPad { pad: p, params });
        if ok {
            self.bank.set_params(pad, &params);
            self.perubahan += 1;
        }
        ok
    }

    /// Ganti sampel pad (mis. hasil impor). `start` pad menjadi 0 dan `end`
    /// panjang sampel.
    pub fn set_sample(&mut self, pad: usize, name: &str, sample: Sampel) -> bool {
        let Some(p) = u8::try_from(pad).ok().filter(|_| pad < TOTAL_PADS) else {
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
        self.perubahan += 1;
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

    /// Model sequencer.
    #[must_use]
    pub fn sequence(&self) -> &Sequence {
        &self.seq
    }

    /// Ganti pola satu grup (mis. dari grid langkah). Dikompilasi di sini,
    /// lalu dikirim. `false` bila grup tidak ada.
    pub fn set_pattern(&mut self, group: usize, pattern: Pattern) -> bool {
        if group >= GROUP_COUNT {
            return false;
        }
        self.seq.patterns[group] = pattern;
        self.perubahan += 1;
        self.terbuka.retain(|t| t.0 / PAD_COUNT != group);
        self.kirim_pola(group)
    }

    /// Mute atau buka grup.
    pub fn set_mute(&mut self, group: usize, mute: bool) -> bool {
        let Some(g) = u8::try_from(group).ok().filter(|_| group < GROUP_COUNT) else {
            return false;
        };
        let ok = self.kirim(Event::SetMute { group: g, mute });
        if ok {
            self.seq.mutes[group] = mute;
            self.perubahan += 1;
        }
        ok
    }

    /// Ganti tempo, swing, metronom, atau count-in.
    pub fn set_settings(&mut self, settings: &SeqSettings) -> bool {
        let baru = settings.sanitized();
        let lama = self.seq.settings;
        self.seq.settings = baru;
        if baru != lama {
            self.perubahan += 1;
        }
        let mut ok = true;
        if baru.bpm.to_bits() != lama.bpm.to_bits() {
            ok &= self.kirim(Event::SetTempo(baru.bpm));
        }
        if baru.metronome != lama.metronome {
            ok &= self.kirim(Event::SetMetronome(baru.metronome));
        }
        if baru.swing.to_bits() != lama.swing.to_bits() {
            for g in 0..GROUP_COUNT {
                ok &= self.kirim_pola(g);
            }
        }
        ok
    }

    /// Perintah transport. Count-in diambil dari pengaturan.
    pub fn transport(&mut self, play: bool, record: bool) -> bool {
        let perintah = if play {
            TransportCommand::Play {
                record,
                count_in_bars: self.seq.settings.count_in_bars,
            }
        } else {
            TransportCommand::Stop
        };
        self.terbuka.clear();
        self.kirim(Event::Transport(perintah))
    }

    /// Hidupkan/matikan rekaman sambil tetap bermain (overdub).
    pub fn set_recording(&mut self, record: bool) -> bool {
        self.kirim(Event::Transport(TransportCommand::Record(record)))
    }

    /// Ambil ketukan yang direkam thread audio dan masukkan ke pola.
    ///
    /// `kompensasi_ms` adalah latensi keluaran: pemain mengetuk mengikuti
    /// bunyi yang **terdengar**, yang tertinggal sebanyak itu dari posisi
    /// mesin, jadi posisi rekaman dimundurkan sebesar itu. Mengembalikan
    /// jumlah not baru.
    pub fn poll_recording(&mut self, kompensasi_ms: f64) -> usize {
        let tick_per_ms = f64::from(self.seq.settings.bpm) * f64::from(PPQN) / 60_000.0;
        let geser = kompensasi_ms.max(0.0) * tick_per_ms;
        let mut baru = 0;
        let mut berubah = [false; GROUP_COUNT];
        while let Ok(r) = self.rekaman.pop() {
            let (tick, pad, pitch) = match r {
                Recorded::On {
                    tick, pad, pitch, ..
                }
                | Recorded::Off { tick, pad, pitch } => (tick, pad, pitch),
            };
            let global = usize::from(pad);
            let grup = global / PAD_COUNT;
            let pola = &mut self.seq.patterns[grup];
            let panjang_u = pola.length_ticks();
            let panjang = f64::from(panjang_u);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let ke_pola = ((tick - geser).rem_euclid(panjang).round() as u32) % panjang_u;
            match r {
                Recorded::On { velocity, .. } => {
                    #[allow(clippy::cast_possible_truncation)]
                    pola.notes.push(Note {
                        tick: ke_pola,
                        pad: (global % PAD_COUNT) as u8,
                        velocity,
                        length: STEP_TICKS,
                        pitch,
                    });
                    self.terbuka.retain(|t| (t.0, t.1) != (global, pitch));
                    self.terbuka.push((global, pitch, pola.notes.len() - 1));
                    baru += 1;
                    berubah[grup] = true;
                }
                Recorded::Off { .. } => {
                    let posisi = self
                        .terbuka
                        .iter()
                        .position(|t| (t.0, t.1) == (global, pitch));
                    if let Some(i) = posisi.map(|p| self.terbuka.remove(p).2)
                        && let Some(n) = pola.notes.get_mut(i)
                    {
                        n.length = ((ke_pola + panjang_u - n.tick) % panjang_u).max(1);
                        berubah[grup] = true;
                    }
                }
            }
        }
        for (g, ya) in berubah.into_iter().enumerate() {
            if ya {
                self.perubahan += 1;
                self.kirim_pola(g);
            }
        }
        baru
    }

    fn kirim_pola(&mut self, group: usize) -> bool {
        let pola = Arc::new(compile(
            &self.seq.patterns[group],
            group,
            self.seq.settings.swing,
        ));
        #[allow(clippy::cast_possible_truncation)]
        let group = group as u8;
        self.kirim(Event::SetPattern {
            group,
            pattern: pola,
        })
    }

    /// Ambil model untuk disimpan saat mesin dibebaskan. Memori lama yang
    /// masih di antrean sampah ikut dibebaskan di sini (thread UI).
    #[must_use]
    pub fn into_model(mut self) -> (PadBank, Sequence) {
        self.empty_trash();
        (self.bank, self.seq)
    }

    /// Ambil model pad saja (lihat [`EventSender::into_model`]).
    #[must_use]
    pub fn into_bank(self) -> PadBank {
        self.into_model().0
    }

    /// Bebaskan sampel/pola lama yang dikembalikan thread audio. Dipanggil
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

    /// Jumlah perubahan model sejak pengirim ini dibuat. Naik setiap kali
    /// pad, sampel, pola, mute, atau pengaturan berubah, termasuk rekaman.
    #[must_use]
    pub fn changes(&self) -> u64 {
        self.perubahan
    }
}

/// Satu aksi terjadwal di dalam blok.
#[derive(Clone, Copy, Debug)]
enum Jadwal {
    Pola(Action),
    Klik {
        awal_bar: bool,
    },
    Ulang {
        pad: u8,
        velocity: f32,
        pitch: i8,
        tick: f64,
    },
}

/// Slot note repeat yang sedang aktif.
#[derive(Clone, Copy, Debug, Default)]
struct Ulang {
    pad: u8,
    pitch: i8,
    velocity: f32,
    /// Jarak antar-ulangan (tick); 0 = slot kosong.
    rate: u32,
    /// Tick saat ulangan dimulai; ulangan pertama jatuh di grid sesudahnya
    /// (ketukan awalnya sudah dikirim UI).
    mulai: f64,
}

/// Note repeat yang bisa aktif bersamaan (mis. akor di mode KEYS).
const MAKS_ULANG: usize = 16;

/// Sisi thread audio: mengisi buffer.
pub struct Engine {
    konsumen: rtrb::Consumer<Event>,
    sampah: rtrb::Producer<Sampah>,
    rekaman: rtrb::Producer<Recorded>,
    suara: Box<[Suara; MAX_VOICES]>,
    pads: [PadParams; TOTAL_PADS],
    sampel: [Option<Arc<Sampel>>; TOTAL_PADS],
    pola: [Arc<CompiledPattern>; GROUP_COUNT],
    mute: [bool; GROUP_COUNT],
    transport: Transport,
    /// Jam yang selalu berjalan, untuk note repeat saat transport berhenti.
    jam: Transport,
    ulang: [Ulang; MAKS_ULANG],
    klik: Klik,
    jadwal: Box<[(u32, Jadwal); MAKS_JADWAL]>,
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

    /// Mesin baru yang memainkan isi `bank` dengan sequencer kosong.
    #[must_use]
    pub fn with_bank(sample_rate: u32, bank: PadBank) -> (Self, EventSender) {
        Self::with_model(sample_rate, bank, Sequence::default())
    }

    /// Mesin baru dari model yang disimpan (mis. saat aplikasi kembali dari
    /// latar). Semua alokasi terjadi di sini, bukan di thread audio.
    #[must_use]
    pub fn with_model(sample_rate: u32, bank: PadBank, seq: Sequence) -> (Self, EventSender) {
        let (produsen, konsumen) = rtrb::RingBuffer::new(EVENT_CAPACITY);
        // Setiap SetSample/SetPattern mengembalikan paling banyak satu benda.
        let (sampah_tulis, sampah_baca) = rtrb::RingBuffer::new(EVENT_CAPACITY);
        let (rekam_tulis, rekam_baca) = rtrb::RingBuffer::new(EVENT_CAPACITY);
        let stats = Arc::new(EngineStats::default());
        let pads = core::array::from_fn(|i| bank.params(i).copied().unwrap_or_default());
        let sampel = core::array::from_fn(|i| bank.sample(i).cloned());
        let settings = seq.settings.sanitized();
        let pola = core::array::from_fn(|g| Arc::new(compile(&seq.patterns[g], g, settings.swing)));
        #[allow(clippy::cast_precision_loss)]
        let sr = sample_rate.max(1) as f32;
        let mut transport = Transport::new(sr, settings.bpm);
        transport.metronom = settings.metronome;
        let mut jam = Transport::new(sr, settings.bpm);
        jam.mulai(0.0, false);
        (
            Self {
                konsumen,
                sampah: sampah_tulis,
                rekaman: rekam_tulis,
                suara: Box::new([Suara::default(); MAX_VOICES]),
                pads,
                sampel,
                pola,
                mute: seq.mutes,
                transport,
                jam,
                ulang: [Ulang::default(); MAKS_ULANG],
                klik: Klik::default(),
                jadwal: Box::new([(0, Jadwal::Klik { awal_bar: false }); MAKS_JADWAL]),
                sr,
                campur: Box::new([0.0; BLOK * 2]),
                stats: Arc::clone(&stats),
            },
            EventSender {
                produsen,
                sampah: sampah_baca,
                rekaman: rekam_baca,
                stats,
                bank,
                seq: Sequence { settings, ..seq },
                terbuka: Vec::new(),
                perubahan: 0,
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
    /// Ketukan dari UI dibunyikan di awal blok. Not dari pola dan klik
    /// metronom dibunyikan **tepat** pada sampelnya di dalam blok.
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

        let frames = out.len() / channels;
        let n_jadwal = self.susun_jadwal(frames);
        let mut awal = 0;
        for j in 0..=n_jadwal {
            let (offset, aksi) = if j < n_jadwal {
                let (o, a) = self.jadwal[j];
                (o as usize, Some(a))
            } else {
                (frames, None)
            };
            if offset > awal {
                self.campur_ke(&mut out[awal * channels..offset * channels], channels);
                awal = offset;
            }
            match aksi {
                Some(Jadwal::Pola(Action::On {
                    pad,
                    velocity,
                    pitch,
                })) => {
                    self.picu(usize::from(pad), velocity, pitch);
                }
                Some(Jadwal::Pola(Action::Off { pad, pitch })) => {
                    self.lepas(usize::from(pad), pitch);
                }
                Some(Jadwal::Ulang {
                    pad,
                    velocity,
                    pitch,
                    tick,
                }) => {
                    self.picu(usize::from(pad), velocity, pitch);
                    self.rekam_pada(
                        tick,
                        Recorded::On {
                            tick,
                            pad,
                            velocity,
                            pitch,
                        },
                    );
                }
                Some(Jadwal::Klik { awal_bar }) => self.klik.picu(awal_bar, self.sr),
                None => {}
            }
        }
        self.transport.maju(frames as u64);
        self.jam.maju(frames as u64);
        self.tulis_statistik();
    }

    /// Isi `self.jadwal` dengan aksi pola dan klik yang jatuh di blok ini,
    /// terurut menurut offset frame. Mengembalikan jumlahnya.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn susun_jadwal(&mut self, frames: usize) -> usize {
        if frames == 0 {
            return 0;
        }
        let mut n = 0;
        if self.transport.main {
            n = self.jadwal_transport(frames);
        }
        if self.ulang.iter().any(|u| u.rate > 0) {
            n = self.jadwal_ulang(frames, n);
        }

        // Urutkan menurut offset; insertion sort stabil, n kecil.
        let jadwal = &mut self.jadwal[..n];
        for i in 1..n {
            let mut j = i;
            while j > 0 && jadwal[j - 1].0 > jadwal[j].0 {
                jadwal.swap(j - 1, j);
                j -= 1;
            }
        }
        n
    }

    /// Ulangan note repeat di blok ini. Saat transport berjalan, grid ulangan
    /// mengikuti posisi lagu (dan ikut direkam); saat berhenti, mengikuti jam
    /// bebas dengan tempo yang sama.
    #[allow(clippy::cast_possible_truncation)]
    fn jadwal_ulang(&mut self, frames: usize, mut jumlah: usize) -> usize {
        let sumber = if self.transport.main {
            &self.transport
        } else {
            &self.jam
        };
        let f0 = sumber.frame();
        let f1 = f0 + frames as u64;
        let lo = sumber.tick_di(f0) - sumber.tick_per_frame();
        let hi = sumber.tick_di(f1) + sumber.tick_per_frame();
        for u in &self.ulang {
            if u.rate == 0 {
                continue;
            }
            let jarak = f64::from(u.rate);
            let mut ke = (lo / jarak).ceil();
            while ke * jarak < hi {
                let tick = ke * jarak;
                let frame = sumber.frame_untuk(tick);
                if tick > u.mulai + 1e-6 && (f0..f1).contains(&frame) && jumlah < MAKS_JADWAL {
                    self.jadwal[jumlah] = (
                        (frame - f0) as u32,
                        Jadwal::Ulang {
                            pad: u.pad,
                            velocity: u.velocity,
                            pitch: u.pitch,
                            tick,
                        },
                    );
                    jumlah += 1;
                }
                ke += 1.0;
            }
        }
        jumlah
    }

    /// Klik metronom dan not pola di blok ini (transport berjalan).
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn jadwal_transport(&mut self, frames: usize) -> usize {
        let t = &self.transport;
        let f0 = t.frame();
        let f1 = f0 + frames as u64;
        // Kandidat dicari di rentang tick yang sedikit lebih lebar; blok mana
        // yang memainkannya diputuskan oleh **frame**-nya. Tanpa ini, event yang
        // jatuh tepat di frame pertama blok berikutnya ikut blok ini dan
        // berbunyi satu sampel terlalu cepat.
        let lo = t.tick_di(f0) - t.tick_per_frame();
        let hi = t.tick_di(f1) + t.tick_per_frame();
        let mut n = 0;
        let mut tambah = |tick: f64, aksi: Jadwal, jadwal: &mut [(u32, Jadwal)]| {
            let f = t.frame_untuk(tick);
            if (f0..f1).contains(&f) && n < MAKS_JADWAL {
                jadwal[n] = ((f - f0) as u32, aksi);
                n += 1;
            }
        };

        // Klik metronom: saat merekam, termasuk count-in (tick negatif).
        if t.metronom && t.rekam {
            let ketuk = f64::from(PPQN);
            let bar = f64::from(TICKS_PER_BAR);
            let mut k = (lo / ketuk).ceil();
            while k * ketuk < hi {
                let awal_bar = ((k * ketuk) / bar).fract().abs() < 1e-9;
                tambah(k * ketuk, Jadwal::Klik { awal_bar }, &mut self.jadwal[..]);
                k += 1.0;
            }
        }

        // Not pola tiap grup yang tidak di-mute; hanya setelah count-in
        // (tick ≥ 0). Setiap grup berputar dengan panjang polanya sendiri.
        let mulai = lo.max(0.0);
        for (pola, &mute) in self.pola.iter().zip(&self.mute) {
            let l = f64::from(pola.length_ticks.max(1));
            if mute || hi <= mulai || pola.actions.is_empty() {
                continue;
            }
            let mut putaran = (mulai / l).floor();
            while putaran * l < hi {
                let dasar = putaran * l;
                let dari = pola.actions.partition_point(|a| dasar + a.tick < mulai);
                for a in &pola.actions[dari..] {
                    let tick = dasar + a.tick;
                    if tick >= hi {
                        break;
                    }
                    tambah(tick, Jadwal::Pola(a.action), &mut self.jadwal[..]);
                }
                putaran += 1.0;
            }
        }

        n
    }

    /// Campur semua suara ke `out` (sepanjang `out.len() / channels` frame).
    fn campur_ke(&mut self, out: &mut [f32], channels: usize) {
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
            for lr in campur.chunks_exact_mut(2) {
                let k = self.klik.berikut();
                lr[0] += k;
                lr[1] += k;
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
    }

    fn tulis_statistik(&self) {
        let mut aktif = 0;
        let mut bit = 0_u64;
        for s in self.suara.iter() {
            if s.aktif() {
                aktif += 1;
                bit |= 1 << s.pad();
            }
        }
        self.stats.voices_active.store(aktif, Ordering::Relaxed);
        self.stats.pads_sounding.store(bit, Ordering::Relaxed);
        #[allow(clippy::cast_possible_truncation)]
        let posisi = self.transport.tick().floor() as i64;
        self.stats.posisi_tick.store(posisi, Ordering::Relaxed);
        let mut status = 0;
        if self.transport.main {
            status |= BIT_MAIN;
        }
        if self.transport.rekam {
            status |= BIT_REKAM;
        }
        self.stats.transport.store(status, Ordering::Relaxed);
    }

    /// Mainkan pad (dari ketukan UI atau dari pola).
    fn picu(&mut self, pad: usize, velocity: f32, pitch: i8) {
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
        self.suara[i].picu(pad, velocity, pitch, &params, sampel, self.sr);
        self.stats.hits_played.fetch_add(1, Ordering::Relaxed);
    }

    /// Lepas suara pad dengan transpose `pitch` (akor KEYS tidak ikut putus).
    fn lepas(&mut self, pad: usize, pitch: i8) {
        for s in self.suara.iter_mut() {
            if s.aktif() && s.pad() == pad && s.nada() == pitch {
                s.lepas();
            }
        }
    }

    /// Kirim rekaman bila sedang merekam dan count-in sudah lewat. Ketukan
    /// dari UI direkam di posisi awal blok.
    fn rekam(&mut self, r: impl FnOnce(f64) -> Recorded) {
        let tick = self.transport.tick();
        self.rekam_pada(tick, r(tick));
    }

    /// Rekam pada `tick` yang tepat (mis. ulangan note repeat).
    fn rekam_pada(&mut self, tick: f64, r: Recorded) {
        if self.transport.main && self.transport.rekam && tick >= 0.0 {
            // Antrean penuh: ketukan tetap berbunyi, hanya tidak direkam.
            let _ = self.rekaman.push(r);
        }
    }

    /// Tick sumber note repeat saat ini (transport bila berjalan, jam bila
    /// tidak).
    fn tick_ulang(&self) -> f64 {
        if self.transport.main {
            self.transport.tick()
        } else {
            self.jam.tick()
        }
    }

    fn buang(&mut self, sampah: Sampah) {
        // Antrean sampah seukuran antrean event, jadi tidak pernah penuh
        // selama UI terus mengirim. Bila tetap penuh, lebih baik bocor daripada
        // membebaskan memori di thread audio.
        if let Err(rtrb::PushError::Full(s)) = self.sampah.push(sampah) {
            core::mem::forget(s);
        }
    }

    fn terapkan(&mut self, event: Event) {
        match event {
            Event::Hit {
                pad,
                velocity,
                pitch,
            } => {
                self.picu(usize::from(pad), velocity, pitch);
                self.rekam(|tick| Recorded::On {
                    tick,
                    pad,
                    velocity,
                    pitch,
                });
            }
            Event::Release { pad, pitch } => {
                self.lepas(usize::from(pad), pitch);
                self.rekam(|tick| Recorded::Off { tick, pad, pitch });
            }
            Event::Repeat {
                pad,
                velocity,
                pitch,
                rate,
            } => {
                let sama = self
                    .ulang
                    .iter()
                    .position(|u| u.rate > 0 && u.pad == pad && u.pitch == pitch);
                if rate == 0 {
                    if let Some(i) = sama {
                        self.ulang[i].rate = 0;
                    }
                } else if let Some(i) = sama.or_else(|| self.ulang.iter().position(|u| u.rate == 0))
                {
                    self.ulang[i] = Ulang {
                        pad,
                        pitch,
                        velocity,
                        rate,
                        mulai: self.tick_ulang(),
                    };
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
                    self.buang(Sampah::Sampel(lama));
                }
            }
            Event::SetPattern { group, pattern } => {
                let lama = core::mem::replace(&mut self.pola[usize::from(group)], pattern);
                self.buang(Sampah::Pola(lama));
            }
            Event::Transport(TransportCommand::Play {
                record,
                count_in_bars,
            }) => {
                let count_in = if record {
                    -f64::from(u32::from(count_in_bars.min(2)) * TICKS_PER_BAR)
                } else {
                    0.0
                };
                self.transport.mulai(count_in, record);
                // Sumber ulangan berpindah ke transport: grid mulai dari awal.
                for u in &mut self.ulang {
                    u.mulai = f64::NEG_INFINITY;
                }
            }
            Event::Transport(TransportCommand::Stop) => {
                self.transport.berhenti();
                for s in self.suara.iter_mut() {
                    if s.aktif() {
                        s.lepas();
                    }
                }
                let sekarang = self.jam.tick();
                for u in &mut self.ulang {
                    u.mulai = sekarang;
                }
            }
            Event::Transport(TransportCommand::Record(rekam)) => {
                self.transport.rekam = rekam && self.transport.main;
            }
            Event::SetTempo(bpm) => {
                self.transport.atur_tempo(bpm);
                self.jam.atur_tempo(bpm);
            }
            Event::SetMetronome(nyala) => self.transport.metronom = nyala,
            Event::SetMute { group, mute } => self.mute[usize::from(group)] = mute,
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
#[cfg(test)]
mod tests_seq;
