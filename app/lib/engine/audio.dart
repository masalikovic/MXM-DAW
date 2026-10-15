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

  /// Pad global `pad` (0..63) ditekan, dengan transpose `nada` (mode KEYS).
  /// `false` bila mesin belum berjalan.
  bool ketuk(int pad, double velocity, {int nada = 0});

  /// Pad `pad` dilepas (mode gate dan loop berhenti).
  bool lepas(int pad, {int nada = 0});

  /// Note repeat: selama aktif, mesin membunyikan pad ulang setiap [rate] tick
  /// (240 = 1/16). [rate] 0 = berhenti.
  bool ulang(int pad, double velocity, {required int rate, int nada = 0});

  /// Bit ke-`n` menyala bila pad `n` sedang berbunyi (untuk LED).
  int padBerbunyi(int grup);

  /// Nama suara ke-64 pad (nomor global = grup × 16 + pad).
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
  rust.TransportState statusTransport(int grup);

  /// Tempo, swing, metronom, count-in, panjang pola, kuantisasi.
  rust.SeqConfig pengaturanSeq(int grup);

  /// Simpan pengaturan; mengembalikan nilai yang benar-benar tersimpan.
  rust.SeqConfig aturPengaturanSeq(int grup, rust.SeqConfig nilai);

  /// Not pola (tick mentah).
  List<rust.NoteData> notPola(int grup);

  /// Ganti semua not pola.
  bool aturNotPola(int grup, List<rust.NoteData> not);

  /// Grup A–D yang di-mute.
  List<bool> muteGrup();

  /// Mute atau buka grup: polanya diam, pad-nya tetap bisa dimainkan.
  bool aturMute(int grup, {required bool mute});

  /// Batalkan langkah terakhir; `false` bila tidak ada.
  bool urungkan();

  /// Ulangi langkah yang dibatalkan; `false` bila tidak ada.
  bool ulangi();

  /// (jumlah undo, jumlah redo) yang tersedia.
  (int, int) jumlahRiwayat();

  /// Salin suara pad global [dari] ke pad [ke].
  bool salinPad(int dari, int ke);

  /// Salin grup [dari] ke grup [ke]: 16 suara beserta polanya.
  bool salinGrup(int dari, int ke);
}

/// Mesin sungguhan: Rust lewat flutter_rust_bridge, lalu Oboe (ADR-0003).
class MesinAudioRust implements MesinAudio {
  const MesinAudioRust();

  @override
  Future<void> mulai() => rust.audioStart();

  @override
  Future<void> berhenti() => rust.audioStop();

  @override
  bool ketuk(int pad, double velocity, {int nada = 0}) =>
      rust.padHit(pad: pad, velocity: velocity, semitones: nada);

  @override
  bool lepas(int pad, {int nada = 0}) =>
      rust.padRelease(pad: pad, semitones: nada);

  @override
  bool ulang(int pad, double velocity, {required int rate, int nada = 0}) =>
      rust.padRepeat(pad: pad, velocity: velocity, semitones: nada, rate: rate);

  @override
  int padBerbunyi(int grup) => rust.padsSounding(group: grup);

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
  rust.TransportState statusTransport(int grup) =>
      rust.transportState(group: grup);

  @override
  rust.SeqConfig pengaturanSeq(int grup) => rust.seqConfig(group: grup);

  @override
  rust.SeqConfig aturPengaturanSeq(int grup, rust.SeqConfig nilai) =>
      rust.setSeqConfig(group: grup, config: nilai);

  @override
  List<rust.NoteData> notPola(int grup) => rust.patternNotes(group: grup);

  @override
  bool aturNotPola(int grup, List<rust.NoteData> not) =>
      rust.setPatternNotes(group: grup, notes: not);

  @override
  List<bool> muteGrup() => rust.groupMutes();

  @override
  bool aturMute(int grup, {required bool mute}) =>
      rust.setGroupMute(group: grup, mute: mute);

  @override
  bool urungkan() => rust.undo();

  @override
  bool ulangi() => rust.redo();

  @override
  (int, int) jumlahRiwayat() => rust.historyCounts();

  @override
  bool salinPad(int dari, int ke) => rust.padCopy(from: dari, to: ke);

  @override
  bool salinGrup(int dari, int ke) => rust.groupCopy(from: dari, to: ke);
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
