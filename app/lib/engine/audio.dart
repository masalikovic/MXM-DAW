import 'dart:typed_data';

import 'bridge/api.dart' as rust;

export 'bridge/api.dart'
    show
        AudioHealth,
        FilterKind,
        NoteData,
        PadSettings,
        PlayKind,
        SeqConfig,
        TransportState;

/// Kendali stream audio dan pad, dari sudut pandang UI.
///
/// Antarmuka ini memisahkan layar dari Rust, supaya tes widget bisa memakai
/// mesin palsu tanpa perangkat audio (ADR-0005).
abstract interface class MesinAudio {
  /// Buka stream audio. Melempar galat bila gagal.
  Future<void> mulai();

  /// Tutup stream audio (mis. saat aplikasi ke latar, ADR-0009 butir 4).
  Future<void> berhenti();

  /// Pad `pad` (0..15) ditekan. `false` bila mesin belum berjalan.
  bool ketuk(int pad, double velocity);

  /// Pad `pad` dilepas (mode gate dan loop berhenti).
  bool lepas(int pad);

  /// Bit ke-`n` menyala bila pad `n` sedang berbunyi (untuk LED).
  int padBerbunyi();

  /// Nama suara ke-16 pad.
  List<String> namaPad();

  /// Angka kesehatan mesin saat ini.
  rust.AudioHealth kesehatan();

  /// Parameter pad, atau `null` bila nomor pad salah.
  rust.PadSettings? pengaturan(int pad);

  /// Simpan parameter pad; mengembalikan nilai yang benar-benar tersimpan
  /// (sudah dibatasi Rust), atau `null` bila gagal.
  rust.PadSettings? aturPengaturan(int pad, rust.PadSettings nilai);

  /// (panjang dalam frame, sample rate) sampel pad.
  (int, int) infoSampel(int pad);

  /// Ringkasan gelombang: `titik` pasangan (min, max) berurutan.
  Float32List puncak(int pad, int titik);

  /// Impor WAV ke pad. Melempar pesan galat (teks) bila WAV tidak bisa dibaca.
  Future<rust.PadSettings> imporWav(int pad, String namaBerkas, Uint8List isi);

  /// Mulai bermain dari awal pola; `rekam` = rekam ketukan (dengan count-in).
  bool mainkan({required bool rekam});

  /// Berhenti bermain.
  bool hentikan();

  /// Rekam hidup/mati sambil bermain (overdub).
  bool aturRekam({required bool nyala});

  /// Posisi transport; sekaligus memasukkan ketukan yang direkam ke pola.
  rust.TransportState statusTransport();

  /// Tempo, swing, metronom, count-in, panjang pola, kuantisasi.
  rust.SeqConfig pengaturanSeq();

  /// Simpan pengaturan; mengembalikan nilai yang benar-benar tersimpan.
  rust.SeqConfig aturPengaturanSeq(rust.SeqConfig nilai);

  /// Not pola (tick mentah).
  List<rust.NoteData> notPola();

  /// Ganti semua not pola.
  bool aturNotPola(List<rust.NoteData> not);
}

/// Mesin sungguhan: Rust lewat flutter_rust_bridge, lalu Oboe (ADR-0003).
class MesinAudioRust implements MesinAudio {
  const MesinAudioRust();

  @override
  Future<void> mulai() => rust.audioStart();

  @override
  Future<void> berhenti() => rust.audioStop();

  @override
  bool ketuk(int pad, double velocity) =>
      rust.padHit(pad: pad, velocity: velocity);

  @override
  bool lepas(int pad) => rust.padRelease(pad: pad);

  @override
  int padBerbunyi() => rust.padsSounding();

  @override
  List<String> namaPad() => rust.padNames();

  @override
  rust.AudioHealth kesehatan() => rust.audioHealth();

  @override
  rust.PadSettings? pengaturan(int pad) => rust.padSettings(pad: pad);

  @override
  rust.PadSettings? aturPengaturan(int pad, rust.PadSettings nilai) =>
      rust.setPadSettings(pad: pad, settings: nilai);

  @override
  (int, int) infoSampel(int pad) => rust.padSampleInfo(pad: pad);

  @override
  Float32List puncak(int pad, int titik) =>
      rust.padPeaks(pad: pad, points: titik);

  @override
  Future<rust.PadSettings> imporWav(
    int pad,
    String namaBerkas,
    Uint8List isi,
  ) => rust.padImportWav(pad: pad, fileName: namaBerkas, bytes: isi);

  @override
  bool mainkan({required bool rekam}) => rust.transportPlay(record: rekam);

  @override
  bool hentikan() => rust.transportStop();

  @override
  bool aturRekam({required bool nyala}) => rust.transportRecord(enabled: nyala);

  @override
  rust.TransportState statusTransport() => rust.transportState();

  @override
  rust.SeqConfig pengaturanSeq() => rust.seqConfig();

  @override
  rust.SeqConfig aturPengaturanSeq(rust.SeqConfig nilai) =>
      rust.setSeqConfig(config: nilai);

  @override
  List<rust.NoteData> notPola() => rust.patternNotes();

  @override
  bool aturNotPola(List<rust.NoteData> not) => rust.setPatternNotes(notes: not);
}

/// Salinan [rust.SeqConfig] dengan sebagian nilai diganti.
extension SalinSeq on rust.SeqConfig {
  rust.SeqConfig salin({
    double? bpm,
    double? swing,
    bool? metronome,
    int? countInBars,
    int? bars,
    int? quantizeGrid,
    double? quantizeStrength,
  }) => rust.SeqConfig(
    bpm: bpm ?? this.bpm,
    swing: swing ?? this.swing,
    metronome: metronome ?? this.metronome,
    countInBars: countInBars ?? this.countInBars,
    bars: bars ?? this.bars,
    quantizeGrid: quantizeGrid ?? this.quantizeGrid,
    quantizeStrength: quantizeStrength ?? this.quantizeStrength,
  );
}

/// Salinan [rust.PadSettings] dengan sebagian nilai diganti.
extension SalinPengaturan on rust.PadSettings {
  rust.PadSettings salin({
    double? level,
    double? pan,
    double? pitch,
    int? start,
    int? end,
    double? attack,
    double? decay,
    double? sustain,
    double? release,
    rust.FilterKind? filter,
    double? cutoff,
    double? resonance,
    double? drive,
    rust.PlayKind? play,
    int? choke,
  }) => rust.PadSettings(
    level: level ?? this.level,
    pan: pan ?? this.pan,
    pitch: pitch ?? this.pitch,
    start: start ?? this.start,
    end: end ?? this.end,
    attack: attack ?? this.attack,
    decay: decay ?? this.decay,
    sustain: sustain ?? this.sustain,
    release: release ?? this.release,
    filter: filter ?? this.filter,
    cutoff: cutoff ?? this.cutoff,
    resonance: resonance ?? this.resonance,
    drive: drive ?? this.drive,
    play: play ?? this.play,
    choke: choke ?? this.choke,
  );
}
