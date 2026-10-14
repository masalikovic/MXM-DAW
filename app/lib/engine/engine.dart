import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

import 'bridge/api.dart';
import 'bridge/frb_generated.dart';

/// Nama pustaka mesin di dalam APK: satu `.so` berisi Rust + C++ (ADR-0003).
const androidEngineLibrary = 'libmxm_engine.so';

/// Hasil memuat mesin Rust saat aplikasi mulai.
///
/// Kegagalan tidak disembunyikan dan tidak membuat aplikasi tertutup paksa:
/// layar menampilkan alasannya apa adanya (prinsip "jujur", rencana §2).
sealed class EngineStatus {
  const EngineStatus();
}

/// Mesin termuat; [version] berasal langsung dari Rust (`daw_core`).
final class EngineReady extends EngineStatus {
  const EngineReady(this.version);

  final String version;
}

/// Mesin gagal dimuat; [reason] adalah pesan galat aslinya.
final class EngineUnavailable extends EngineStatus {
  const EngineUnavailable(this.reason);

  final String reason;
}

/// Memuat pustaka mesin dari [library], lalu menanyakan versinya ke Rust.
///
/// Di HP, [library] adalah nama pustaka di APK. Di tes host (Linux),
/// [library] adalah path ke hasil `cargo build -p daw-ffi`.
Future<EngineStatus> startEngine({
  String library = androidEngineLibrary,
}) async {
  try {
    await RustLib.init(externalLibrary: ExternalLibrary.open(library));
    return EngineReady(engineVersion());
  } on Object catch (galat) {
    return EngineUnavailable('$galat');
  }
}
