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

  /// Pad ditekan. `false` bila mesin belum berjalan.
  bool ketuk(double velocity);

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
  bool ketuk(double velocity) => rust.padHit(velocity: velocity);

  @override
  rust.AudioHealth kesehatan() => rust.audioHealth();
}
