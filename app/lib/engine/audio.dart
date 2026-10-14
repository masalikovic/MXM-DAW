import 'bridge/api.dart' as rust;

export 'bridge/api.dart' show AudioHealth;

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
}
