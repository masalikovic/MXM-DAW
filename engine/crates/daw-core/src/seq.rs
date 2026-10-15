//! Sequencer: pola, kuantisasi, swing, dan kompilasi (rencana §4.3, §9, §10).
//!
//! Pola disimpan **mentah** (`tick` seperti saat direkam). Kuantisasi dan swing
//! diterapkan saat dikompilasi, sehingga kekuatannya bisa diubah kapan saja
//! tanpa merusak rekaman asli. Hasil kompilasi ([`CompiledPattern`]) dikirim ke
//! thread audio sebagai `Arc` (ADR-0015).

// Tick (bilangan bulat) dan posisi pecahan (f64) memang dikonversi bolak-balik.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use crate::pad::{GROUP_COUNT, PAD_COUNT};

/// Tick per ketukan (not 1/4).
pub const PPQN: u32 = 960;
/// Tick per bar (4/4).
pub const TICKS_PER_BAR: u32 = 4 * PPQN;
/// Tick per langkah grid (not 1/16).
pub const STEP_TICKS: u32 = PPQN / 4;
/// Panjang pola maksimal (bar).
pub const MAX_BARS: u8 = 16;
/// Tempo yang diizinkan (BPM).
pub const BPM_MIN: f32 = 40.0;
/// Tempo tertinggi (BPM).
pub const BPM_MAX: f32 = 240.0;

/// Satu not dalam pola. `tick` mentah, seperti saat direkam.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    /// Posisi dari awal pola (tick, 960 per ketukan).
    pub tick: u32,
    /// Nomor pad.
    pub pad: u8,
    /// Kekuatan 0..1.
    pub velocity: f32,
    /// Panjang (tick); penting untuk pad gate dan loop.
    pub length: u32,
}

/// Pola satu grup.
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    /// Panjang dalam bar (1..=16).
    pub bars: u8,
    /// Not, tidak harus urut.
    pub notes: Vec<Note>,
    /// Grid kuantisasi dalam tick (mis. 240 = 1/16); 0 = mati.
    pub quantize_grid: u32,
    /// Kekuatan kuantisasi 0..1.
    pub quantize_strength: f32,
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            bars: 1,
            notes: Vec::new(),
            quantize_grid: STEP_TICKS,
            quantize_strength: 0.0,
        }
    }
}

impl Pattern {
    /// Panjang pola dalam tick.
    #[must_use]
    pub fn length_ticks(&self) -> u32 {
        u32::from(self.bars.clamp(1, MAX_BARS)) * TICKS_PER_BAR
    }
}

/// Pengaturan transport yang disimpan bersama proyek.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeqSettings {
    /// Tempo 40..240 BPM.
    pub bpm: f32,
    /// Swing 0,5 (lurus) … 0,75.
    pub swing: f32,
    /// Metronom berbunyi saat merekam.
    pub metronome: bool,
    /// Count-in sebelum merekam: 0, 1, atau 2 bar.
    pub count_in_bars: u8,
}

impl Default for SeqSettings {
    fn default() -> Self {
        Self {
            bpm: 120.0,
            swing: 0.5,
            metronome: true,
            count_in_bars: 1,
        }
    }
}

impl SeqSettings {
    /// Salinan dengan nilai di dalam jangkauan sah.
    #[must_use]
    pub fn sanitized(&self) -> Self {
        let bpm = if self.bpm.is_finite() {
            (self.bpm * 10.0).round() / 10.0
        } else {
            120.0
        };
        Self {
            bpm: bpm.clamp(BPM_MIN, BPM_MAX),
            swing: if self.swing.is_finite() {
                self.swing.clamp(0.5, 0.75)
            } else {
                0.5
            },
            metronome: self.metronome,
            count_in_bars: self.count_in_bars.min(2),
        }
    }
}

/// Model sequencer yang disimpan bersama proyek: pola tiap grup dan
/// pengaturannya.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sequence {
    /// Satu pola per grup A–D; not memakai nomor pad lokal 0..16.
    pub patterns: [Pattern; GROUP_COUNT],
    /// Grup yang di-mute: polanya tidak dibunyikan.
    pub mutes: [bool; GROUP_COUNT],
    /// Tempo, swing, metronom, count-in.
    pub settings: SeqSettings,
}

/// Aksi terjadwal di pola terkompilasi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// Pad dimainkan.
    On {
        /// Nomor pad.
        pad: u8,
        /// Kekuatan.
        velocity: f32,
    },
    /// Pad dilepas (akhir not).
    Off {
        /// Nomor pad.
        pad: u8,
    },
}

/// Aksi dengan posisi akhir (tick pecahan, sesudah kuantisasi dan swing).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scheduled {
    /// Posisi dari awal pola, 0 ≤ tick < panjang pola.
    pub tick: f64,
    /// Yang dilakukan.
    pub action: Action,
}

/// Pola siap main: aksi terurut menurut tick. Dibuat di thread UI.
#[derive(Clone, Debug, PartialEq)]
pub struct CompiledPattern {
    /// Panjang pola (tick).
    pub length_ticks: u32,
    /// Aksi terurut; `Off` sebelum `On` pada tick yang sama.
    pub actions: Box<[Scheduled]>,
}

impl CompiledPattern {
    /// Pola kosong sepanjang satu bar.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            length_ticks: TICKS_PER_BAR,
            actions: Box::new([]),
        }
    }
}

/// Posisi `tick` setelah kuantisasi: digeser menuju titik grid terdekat
/// sebesar `strength` (0 = asli, 1 = tepat di grid).
#[must_use]
pub fn quantize(tick: f64, grid: u32, strength: f32) -> f64 {
    if grid == 0 || strength <= 0.0 {
        return tick;
    }
    let g = f64::from(grid);
    let terdekat = (tick / g).round() * g;
    tick + (terdekat - tick) * f64::from(strength.min(1.0))
}

/// Swing (rencana §9): not 1/16 kedua di setiap pasangan (langkah 2, 4, 6, …
/// dihitung dari 1) digeser `(swing − 50 %) × 2 × durasi_langkah`. Hanya not
/// yang tepat berada di grid 1/16 yang digeser.
#[must_use]
pub fn swing(tick: f64, swing: f32) -> f64 {
    let langkah = f64::from(STEP_TICKS);
    let indeks = (tick / langkah).round();
    if (tick - indeks * langkah).abs() > 1e-6 || indeks as i64 % 2 == 0 {
        return tick;
    }
    // Swing disimpan f32; dibulatkan ke 0,1 % supaya sisa f32 tidak menggeser
    // posisi (lihat `bpm_tepat`).
    let swing = (f64::from(swing) * 1000.0).round() / 1000.0;
    tick + (swing - 0.5) * 2.0 * langkah
}

/// Kompilasi pola grup `group`: kuantisasi, swing, lalu urutkan. Nomor pad di
/// hasilnya global (grup × 16 + pad). Not untuk pad yang tidak ada dibuang.
#[must_use]
pub fn compile(pattern: &Pattern, group: usize, swing_amount: f32) -> CompiledPattern {
    #[allow(clippy::cast_possible_truncation)]
    let dasar_pad = (group.min(GROUP_COUNT - 1) * PAD_COUNT) as u8;
    let panjang = pattern.length_ticks();
    let l = f64::from(panjang);
    let bungkus = |t: f64| t.rem_euclid(l);
    let mut aksi: Vec<Scheduled> = Vec::with_capacity(pattern.notes.len() * 2);
    for n in &pattern.notes {
        if usize::from(n.pad) >= PAD_COUNT {
            continue;
        }
        let q = quantize(
            f64::from(n.tick),
            pattern.quantize_grid,
            pattern.quantize_strength,
        );
        let awal = bungkus(swing(q, swing_amount));
        let akhir = bungkus(awal + f64::from(n.length.max(1)));
        aksi.push(Scheduled {
            tick: awal,
            action: Action::On {
                pad: dasar_pad + n.pad,
                velocity: n.velocity.clamp(0.0, 1.0),
            },
        });
        aksi.push(Scheduled {
            tick: akhir,
            action: Action::Off {
                pad: dasar_pad + n.pad,
            },
        });
    }
    aksi.sort_by(|a, b| {
        a.tick.total_cmp(&b.tick).then_with(|| {
            matches!(a.action, Action::On { .. }).cmp(&matches!(b.action, Action::On { .. }))
        })
    });
    CompiledPattern {
        length_ticks: panjang,
        actions: aksi.into_boxed_slice(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Action, CompiledPattern, Note, Pattern, STEP_TICKS, SeqSettings, TICKS_PER_BAR, compile,
        quantize, swing,
    };

    fn not(tick: u32, pad: u8) -> Note {
        Note {
            tick,
            pad,
            velocity: 1.0,
            length: 120,
        }
    }

    #[test]
    fn kuantisasi_tidak_merusak_dan_bisa_diatur_kekuatannya() {
        assert!((quantize(250.0, 240, 0.0) - 250.0).abs() < 1e-9);
        assert!((quantize(250.0, 240, 1.0) - 240.0).abs() < 1e-9);
        assert!((quantize(250.0, 240, 0.5) - 245.0).abs() < 1e-9);
        assert!((quantize(250.0, 0, 1.0) - 250.0).abs() < 1e-9);
    }

    #[test]
    fn swing_menggeser_langkah_genap_saja() {
        // Langkah 1 (tick 0) tetap, langkah 2 (tick 240) bergeser.
        assert!((swing(0.0, 0.75) - 0.0).abs() < 1e-9);
        assert!((swing(240.0, 0.75) - 360.0).abs() < 1e-9);
        assert!((swing(480.0, 0.75) - 480.0).abs() < 1e-9);
        assert!((swing(240.0, 0.5) - 240.0).abs() < 1e-9);
        // Di luar grid 1/16: tidak digeser.
        assert!((swing(250.0, 0.75) - 250.0).abs() < 1e-9);
    }

    #[test]
    fn kompilasi_urut_dengan_off_dan_bungkus() {
        let pola = Pattern {
            bars: 1,
            notes: vec![
                not(960, 1),
                not(0, 0),
                Note {
                    length: 480,
                    ..not(TICKS_PER_BAR - 240, 2)
                },
                not(0, 99), // pad tidak ada: dibuang
            ],
            ..Pattern::default()
        };
        let k = compile(&pola, 0, 0.5);
        assert_eq!(k.length_ticks, TICKS_PER_BAR);
        assert_eq!(k.actions.len(), 6);
        assert!(k.actions.windows(2).all(|w| w[0].tick <= w[1].tick));
        // Not terakhir melewati ujung pola: off-nya dibungkus ke tick 240.
        let off2 = k
            .actions
            .iter()
            .find(|a| a.action == Action::Off { pad: 2 })
            .unwrap();
        assert!((off2.tick - 240.0).abs() < 1e-9);
    }

    #[test]
    fn kuantisasi_dan_swing_saat_kompilasi() {
        let pola = Pattern {
            bars: 1,
            notes: vec![not(250, 0)],
            quantize_grid: STEP_TICKS,
            quantize_strength: 1.0,
        };
        let k = compile(&pola, 0, 0.6);
        // 250 → 240 (langkah 2) → +48 karena swing 60 %.
        assert!(
            (k.actions[0].tick - 288.0).abs() < 1e-9,
            "{:?}",
            k.actions[0]
        );
        assert!(pola.notes[0].tick == 250, "rekaman asli tidak berubah");
    }

    #[test]
    fn pengaturan_dibatasi() {
        let s = SeqSettings {
            bpm: 999.95,
            swing: 0.1,
            metronome: false,
            count_in_bars: 9,
        }
        .sanitized();
        assert_eq!((s.bpm, s.swing, s.count_in_bars), (240.0, 0.5, 2));
        let s = SeqSettings {
            bpm: 97.26,
            ..SeqSettings::default()
        }
        .sanitized();
        assert!((s.bpm - 97.3).abs() < 1e-4, "satu desimal: {}", s.bpm);
    }

    #[test]
    fn nomor_pad_global_per_grup() {
        let pola = Pattern {
            notes: vec![not(0, 3)],
            ..Pattern::default()
        };
        assert_eq!(
            compile(&pola, 2, 0.5).actions[0].action,
            Action::On {
                pad: 35,
                velocity: 1.0
            }
        );
    }

    #[test]
    fn pola_kosong() {
        assert!(CompiledPattern::empty().actions.is_empty());
        let p = Pattern {
            bars: 40,
            ..Pattern::default()
        };
        assert_eq!(p.length_ticks(), 16 * TICKS_PER_BAR);
    }
}
