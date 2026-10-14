// Tes ujung ke ujung Dart → flutter_rust_bridge → Rust, di host (Linux).
//
// Butuh pustaka Rust yang sudah dibangun:
//   cd engine && cargo build -p daw-ffi
// CI menjalankan perintah itu sebelum `flutter test`.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/engine/engine.dart';

/// Hasil `cargo build -p daw-ffi`, relatif terhadap folder app/.
const pustakaHost = '../engine/target/debug/libdaw_ffi.so';

/// Versi di `[workspace.package]` engine/Cargo.toml: sumber kebenaran versi.
String versiDiCargoToml() {
  final isi = File('../engine/Cargo.toml').readAsStringSync();
  final cocok = RegExp(r'\[workspace\.package\][^\[]*?version = "([^"]+)"')
      .firstMatch(isi);
  return cocok!.group(1)!;
}

void main() {
  // Urutan penting: tes ini harus jalan sebelum mesin berhasil dimuat,
  // karena RustLib hanya bisa diinisialisasi sekali per proses.
  test('pustaka yang tidak ada → EngineUnavailable, bukan crash', () async {
    final status = await startEngine(library: 'libtidakada.so');

    expect(status, isA<EngineUnavailable>());
  });

  test('versi dari Rust sama dengan versi di engine/Cargo.toml', () async {
    expect(
      File(pustakaHost).existsSync(),
      isTrue,
      reason: 'Bangun dulu: cd engine && cargo build -p daw-ffi',
    );

    final status = await startEngine(library: pustakaHost);

    expect(
      status,
      isA<EngineReady>().having(
        (s) => s.version,
        'version',
        versiDiCargoToml(),
      ),
      reason: status is EngineUnavailable ? status.reason : null,
    );
  });
}
