//! Berkas proyek (ADR-0007, ADR-0016): `project.json` + `samples/s<id>.wav`
//! di folder data pribadi aplikasi.
//!
//! - Struktur JSON di sini **terpisah** dari tipe `daw-core`, supaya mesin
//!   boleh berubah tanpa merusak berkas lama. Setiap kolom punya nilai bawaan,
//!   jadi berkas lama yang belum punya kolom baru tetap terbaca (migrasi
//!   selalu menambah, rencana §10).
//! - Penulisan atomik: tulis ke `*.tmp`, `fsync`, lalu ganti nama. Aplikasi
//!   yang dimatikan paksa di tengah penulisan meninggalkan berkas lama utuh.
//! - Kit pabrik tidak disalin ke disk; cukup rujukan `factory:NAMA`.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use daw_core::bank::{PadBank, SampleSource};
use daw_core::kit::FACTORY_NAMES;
use daw_core::pad::{Envelope, Filter, FilterMode, GROUP_COUNT, PAD_COUNT, PadParams, PlayMode};
use daw_core::sampel::Sampel;
use daw_core::seq::{Note, Pattern, SeqSettings, Sequence};
use serde::{Deserialize, Serialize};

/// Versi skema yang ditulis dan versi tertinggi yang bisa dibaca.
pub const SCHEMA: u32 = 1;
/// Nama berkas utama.
pub const PROJECT_FILE: &str = "project.json";
const SAMPLES_DIR: &str = "samples";

/// Hasil membuka proyek.
#[derive(Debug)]
pub struct Loaded {
    /// Model pad.
    pub bank: PadBank,
    /// Model sequencer.
    pub seq: Sequence,
    /// Hal yang tidak bisa dipulihkan sepenuhnya (mis. sampel hilang).
    pub warnings: Vec<String>,
}

/// Kenapa proyek tidak bisa dibuka.
#[derive(Debug, PartialEq, Eq)]
pub enum LoadError {
    /// Dibuat oleh versi aplikasi yang lebih baru. Berkas **jangan** ditimpa.
    TooNew(u32),
    /// Rusak; sudah dipindah ke `nama` supaya tidak tertimpa simpan otomatis.
    Corrupt {
        /// Pesan galat untuk pengguna.
        message: String,
        /// Nama berkas cadangan.
        moved_to: String,
    },
    /// Gagal membaca dari disk.
    Io(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooNew(v) => write!(
                f,
                "proyek dibuat aplikasi versi lebih baru (skema {v}); perbarui aplikasi"
            ),
            Self::Corrupt { message, moved_to } => {
                write!(f, "proyek rusak ({message}); disimpan sebagai {moved_to}")
            }
            Self::Io(e) => write!(f, "proyek tidak bisa dibaca: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Struktur JSON (rencana §10). Nama kolom bahasa Inggris, sama dengan rencana.

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
struct ProyekJson {
    schema: u32,
    name: String,
    bpm: f32,
    swing: f32,
    metronome: bool,
    count_in_bars: u8,
    groups: Vec<GrupJson>,
}

impl Default for ProyekJson {
    fn default() -> Self {
        let s = SeqSettings::default();
        Self {
            schema: SCHEMA,
            name: "Proyek".to_owned(),
            bpm: s.bpm,
            swing: s.swing,
            metronome: s.metronome,
            count_in_bars: s.count_in_bars,
            groups: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
#[serde(default)]
struct GrupJson {
    id: String,
    mute: bool,
    pads: Vec<PadJson>,
    patterns: Vec<PolaJson>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
struct PadJson {
    index: u8,
    name: String,
    /// `factory:NAMA`, `samples/s<id>.wav`, atau `null`.
    sample: Option<String>,
    start: u32,
    end: u32,
    pitch: f32,
    level: f32,
    pan: f32,
    envelope: AmplopJson,
    filter: FilterJson,
    drive: f32,
    play_mode: String,
    choke: u8,
}

impl Default for PadJson {
    fn default() -> Self {
        Self::dari(0, "", None, &PadParams::default())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
struct AmplopJson {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

impl Default for AmplopJson {
    fn default() -> Self {
        let e = Envelope::default();
        Self {
            attack: e.attack,
            decay: e.decay,
            sustain: e.sustain,
            release: e.release,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
struct FilterJson {
    mode: String,
    cutoff: f32,
    resonance: f32,
}

impl Default for FilterJson {
    fn default() -> Self {
        Self {
            mode: "off".to_owned(),
            cutoff: 1.0,
            resonance: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
struct PolaJson {
    index: u8,
    bars: u8,
    quantize: KuantisasiJson,
    events: Vec<EventJson>,
}

impl Default for PolaJson {
    fn default() -> Self {
        let p = Pattern::default();
        Self {
            index: 1,
            bars: p.bars,
            quantize: KuantisasiJson {
                grid: p.quantize_grid,
                strength: p.quantize_strength,
            },
            events: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
#[serde(default)]
struct KuantisasiJson {
    grid: u32,
    strength: f32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
#[serde(default)]
struct EventJson {
    tick: u32,
    pad: u8,
    velocity: f32,
    length: u32,
    /// Transpose mode KEYS (semitone). Ditambahkan di M4 dengan bawaan 0,
    /// jadi berkas lama tanpa kolom ini tetap terbaca.
    pitch: i8,
}

impl PadJson {
    fn dari(index: u8, name: &str, sample: Option<String>, p: &PadParams) -> Self {
        Self {
            index,
            name: name.to_owned(),
            sample,
            start: p.start,
            end: p.end,
            pitch: p.pitch,
            level: p.level,
            pan: p.pan,
            envelope: AmplopJson {
                attack: p.envelope.attack,
                decay: p.envelope.decay,
                sustain: p.envelope.sustain,
                release: p.envelope.release,
            },
            filter: FilterJson {
                mode: match p.filter.mode {
                    FilterMode::Off => "off",
                    FilterMode::LowPass => "lp",
                    FilterMode::HighPass => "hp",
                    FilterMode::BandPass => "bp",
                }
                .to_owned(),
                cutoff: p.filter.cutoff,
                resonance: p.filter.resonance,
            },
            drive: p.drive,
            play_mode: match p.play_mode {
                PlayMode::OneShot => "one_shot",
                PlayMode::Gate => "gate",
                PlayMode::Loop => "loop",
            }
            .to_owned(),
            choke: p.choke,
        }
    }

    fn params(&self) -> PadParams {
        PadParams {
            level: self.level,
            pan: self.pan,
            pitch: self.pitch,
            start: self.start,
            end: self.end,
            envelope: Envelope {
                attack: self.envelope.attack,
                decay: self.envelope.decay,
                sustain: self.envelope.sustain,
                release: self.envelope.release,
            },
            filter: Filter {
                mode: match self.filter.mode.as_str() {
                    "lp" => FilterMode::LowPass,
                    "hp" => FilterMode::HighPass,
                    "bp" => FilterMode::BandPass,
                    _ => FilterMode::Off,
                },
                cutoff: self.filter.cutoff,
                resonance: self.filter.resonance,
            },
            drive: self.drive,
            play_mode: match self.play_mode.as_str() {
                "gate" => PlayMode::Gate,
                "loop" => PlayMode::Loop,
                _ => PlayMode::OneShot,
            },
            choke: self.choke,
        }
        .sanitized()
    }
}

const HURUF: [&str; GROUP_COUNT] = ["A", "B", "C", "D"];

/// Hanya kolom `schema`, dibaca sebelum isi lainnya.
#[derive(Deserialize)]
struct HanyaSkema {
    schema: Option<u32>,
}

fn berkas_sampel(id: u64) -> String {
    format!("{SAMPLES_DIR}/s{id}.wav")
}

// ---------------------------------------------------------------------------
// Simpan

/// Simpan model ke `dir` (dibuat bila belum ada). Hanya sampel impor yang
/// belum ada di disk yang ditulis; sampel yang tidak lagi dipakai dihapus
/// setelah `project.json` baru tersimpan.
///
/// # Errors
/// Pesan galat bila disk penuh atau tidak bisa ditulis.
pub fn save(dir: &Path, bank: &PadBank, seq: &Sequence) -> Result<(), String> {
    let galat = |apa: &str, e: std::io::Error| format!("gagal {apa}: {e}");
    fs::create_dir_all(dir.join(SAMPLES_DIR)).map_err(|e| galat("membuat folder proyek", e))?;

    let mut dipakai = BTreeSet::new();
    let mut grup = Vec::with_capacity(GROUP_COUNT);
    for (g, huruf) in HURUF.iter().enumerate() {
        let mut pads = Vec::with_capacity(PAD_COUNT);
        for lokal in 0..PAD_COUNT {
            let pad = g * PAD_COUNT + lokal;
            let sampel = match bank.source(pad) {
                Some(SampleSource::Factory(i)) => FACTORY_NAMES
                    .get(usize::from(i))
                    .map(|n| format!("factory:{n}")),
                Some(SampleSource::Imported(id)) => {
                    let nama = berkas_sampel(id);
                    if let Some(s) = bank.sample(pad) {
                        tulis_wav_bila_belum_ada(&dir.join(&nama), s)?;
                    }
                    dipakai.insert(nama.clone());
                    Some(nama)
                }
                Some(SampleSource::Empty) | None => None,
            };
            #[allow(clippy::cast_possible_truncation)]
            pads.push(PadJson::dari(
                lokal as u8,
                bank.name(pad).unwrap_or_default(),
                sampel,
                &bank.params(pad).copied().unwrap_or_default(),
            ));
        }
        let p = &seq.patterns[g];
        grup.push(GrupJson {
            id: (*huruf).to_owned(),
            mute: seq.mutes[g],
            pads,
            patterns: vec![PolaJson {
                index: 1,
                bars: p.bars,
                quantize: KuantisasiJson {
                    grid: p.quantize_grid,
                    strength: p.quantize_strength,
                },
                events: p
                    .notes
                    .iter()
                    .map(|n| EventJson {
                        tick: n.tick,
                        pad: n.pad,
                        velocity: n.velocity,
                        length: n.length,
                        pitch: n.pitch,
                    })
                    .collect(),
            }],
        });
    }
    let json = ProyekJson {
        schema: SCHEMA,
        name: "Proyek".to_owned(),
        bpm: seq.settings.bpm,
        swing: seq.settings.swing,
        metronome: seq.settings.metronome,
        count_in_bars: seq.settings.count_in_bars,
        groups: grup,
    };
    let isi = serde_json::to_vec_pretty(&json).map_err(|e| format!("gagal menyusun JSON: {e}"))?;
    tulis_atomik(&dir.join(PROJECT_FILE), &isi)?;
    hapus_sampel_tak_terpakai(dir, &dipakai);
    Ok(())
}

/// Tulis lewat berkas sementara: `isi` → `jalur.tmp` → fsync → ganti nama.
fn tulis_atomik(jalur: &Path, isi: &[u8]) -> Result<(), String> {
    let sementara = jalur.with_extension("tmp");
    let galat = |e: std::io::Error| format!("gagal menulis {}: {e}", jalur.display());
    let mut f = fs::File::create(&sementara).map_err(galat)?;
    f.write_all(isi).map_err(galat)?;
    f.sync_all().map_err(galat)?;
    drop(f);
    fs::rename(&sementara, jalur).map_err(galat)
}

fn tulis_wav_bila_belum_ada(jalur: &Path, sampel: &Sampel) -> Result<(), String> {
    if jalur.exists() {
        return Ok(());
    }
    let spek = hound::WavSpec {
        #[allow(clippy::cast_possible_truncation)]
        channels: sampel.channels() as u16,
        sample_rate: sampel.sample_rate(),
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let galat = |e: hound::Error| format!("gagal menyusun WAV: {e}");
        let mut w = hound::WavWriter::new(&mut buf, spek).map_err(galat)?;
        for &v in sampel.as_i16() {
            w.write_sample(v).map_err(galat)?;
        }
        w.finalize().map_err(galat)?;
    }
    tulis_atomik(jalur, buf.get_ref())
}

fn hapus_sampel_tak_terpakai(dir: &Path, dipakai: &BTreeSet<String>) {
    let Ok(isi) = fs::read_dir(dir.join(SAMPLES_DIR)) else {
        return;
    };
    for e in isi.flatten() {
        let nama = format!("{SAMPLES_DIR}/{}", e.file_name().to_string_lossy());
        if !dipakai.contains(&nama) {
            // Gagal menghapus bukan masalah: hanya memakan sedikit ruang.
            let _ = fs::remove_file(e.path());
        }
    }
}

// ---------------------------------------------------------------------------
// Buka

/// Buka proyek dari `dir`. `Ok(None)` bila belum ada proyek (pertama kali).
///
/// # Errors
/// [`LoadError`]: proyek terlalu baru (berkas dibiarkan), rusak (berkas
/// dipindah ke samping), atau tidak bisa dibaca.
pub fn load(dir: &Path) -> Result<Option<Loaded>, LoadError> {
    let jalur = dir.join(PROJECT_FILE);
    let isi = match fs::read(&jalur) {
        Ok(isi) => isi,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(LoadError::Io(e.to_string())),
    };
    // Baca versi skema lebih dulu, terpisah dari isinya: proyek dari versi
    // baru harus ditolak, bukan dibaca salah.
    let skema = match serde_json::from_slice::<HanyaSkema>(&isi) {
        Ok(h) => h.schema.unwrap_or(0),
        Err(e) => return Err(pindahkan_rusak(&jalur, &e.to_string())),
    };
    if skema > SCHEMA {
        return Err(LoadError::TooNew(skema));
    }
    let json: ProyekJson = match serde_json::from_slice(&isi) {
        Ok(j) => j,
        Err(e) => return Err(pindahkan_rusak(&jalur, &e.to_string())),
    };
    Ok(Some(dari_json(dir, &json)))
}

fn pindahkan_rusak(jalur: &Path, pesan: &str) -> LoadError {
    let mut cadangan: PathBuf = jalur.with_extension("rusak.json");
    let mut n = 1;
    while cadangan.exists() {
        n += 1;
        cadangan = jalur.with_extension(format!("rusak-{n}.json"));
    }
    let moved_to = cadangan
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    if let Err(e) = fs::rename(jalur, &cadangan) {
        return LoadError::Io(format!("proyek rusak dan tidak bisa dipindah: {e}"));
    }
    LoadError::Corrupt {
        message: pesan.to_owned(),
        moved_to,
    }
}

fn dari_json(dir: &Path, json: &ProyekJson) -> Loaded {
    // Sumber suara pabrik terpisah dari bank yang sedang diisi: bila pad A1
    // sudah diganti sampel impor, `factory:KICK` di grup B tetap kick asli.
    let pabrik = PadBank::factory();
    let mut bank = pabrik.clone();
    let mut seq = Sequence {
        settings: SeqSettings {
            bpm: json.bpm,
            swing: json.swing,
            metronome: json.metronome,
            count_in_bars: json.count_in_bars,
        }
        .sanitized(),
        ..Sequence::default()
    };
    let mut warnings = Vec::new();
    for grup in &json.groups {
        let Some(g) = HURUF.iter().position(|h| *h == grup.id) else {
            warnings.push(format!("grup '{}' tidak dikenal, dilewati", grup.id));
            continue;
        };
        seq.mutes[g] = grup.mute;
        for pad in &grup.pads {
            let lokal = usize::from(pad.index);
            if lokal >= PAD_COUNT {
                continue;
            }
            let global = g * PAD_COUNT + lokal;
            let (sampel, asal) = muat_sampel(dir, pad.sample.as_deref(), &pabrik, &mut warnings);
            bank.restore_pad(global, &pad.name, &pad.params(), sampel, asal);
        }
        if let Some(p) = grup.patterns.first() {
            seq.patterns[g] = Pattern {
                bars: p.bars.clamp(1, daw_core::seq::MAX_BARS),
                quantize_grid: p.quantize.grid,
                quantize_strength: p.quantize.strength.clamp(0.0, 1.0),
                notes: p
                    .events
                    .iter()
                    .filter(|e| usize::from(e.pad) < PAD_COUNT)
                    .map(|e| Note {
                        tick: e.tick,
                        pad: e.pad,
                        velocity: e.velocity.clamp(0.0, 1.0),
                        length: e.length.max(1),
                        pitch: e.pitch,
                    })
                    .collect(),
            };
        }
    }
    Loaded {
        bank,
        seq,
        warnings,
    }
}

/// Sampel dan asalnya dari rujukan di JSON.
fn muat_sampel(
    dir: &Path,
    rujukan: Option<&str>,
    pabrik: &PadBank,
    warnings: &mut Vec<String>,
) -> (Option<Arc<Sampel>>, SampleSource) {
    let Some(r) = rujukan else {
        return (None, SampleSource::Empty);
    };
    if let Some(nama) = r.strip_prefix("factory:") {
        // Grup A di bank pabrik berisi kit asli; sampelnya dipakai bersama.
        if let Some(i) = FACTORY_NAMES.iter().position(|n| *n == nama) {
            #[allow(clippy::cast_possible_truncation)]
            return (pabrik.sample(i).cloned(), SampleSource::Factory(i as u8));
        }
        warnings.push(format!("suara pabrik '{nama}' tidak dikenal"));
        return (None, SampleSource::Empty);
    }
    let id = r
        .strip_prefix(&format!("{SAMPLES_DIR}/s"))
        .and_then(|s| s.strip_suffix(".wav"))
        .and_then(|s| s.parse::<u64>().ok());
    let Some(id) = id else {
        warnings.push(format!("rujukan sampel '{r}' tidak dikenal"));
        return (None, SampleSource::Empty);
    };
    match fs::read(dir.join(r))
        .map_err(|e| e.to_string())
        .and_then(|b| crate::decode_wav(&b))
    {
        Ok(s) => (Some(Arc::new(s)), SampleSource::Imported(id)),
        Err(e) => {
            warnings.push(format!("sampel {r} hilang atau rusak: {e}"));
            (None, SampleSource::Empty)
        }
    }
}

#[cfg(test)]
mod tests;
