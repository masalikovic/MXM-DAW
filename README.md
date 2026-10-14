# MXM-DAW

*Sampler-composer* genggam untuk Android: pad, sampler, sequencer, efek,
mixer, dan mode lagu dalam satu aplikasi yang terasa seperti alat musik
fisik. Tampilan dibangun dengan **Flutter**, mesin audio dengan **Rust**.
Tanpa akun, tanpa internet.

Proyek ini juga **proyek belajar arsitektur perangkat lunak**: setiap
keputusan dicatat, dan setiap tahap punya bukti selesai yang bisa diperiksa.

## Mulai membaca dari sini

1. [Rencana lengkap](docs/rencana.md): visi, fitur, desain, arsitektur,
   tahapan.
2. [Architecture Decision Records](docs/adr/README.md): keputusan dan
   alasannya.
3. [AGENTS.md](AGENTS.md): aturan untuk agen yang ikut mengerjakan repo ini.

## Status

| Tahap | Isi | Status |
|---|---|---|
| M0 | Fondasi: ADR, CI, token desain, aplikasi kosong yang memanggil Rust, APK pratinjau | 🟡 berjalan |
| M1 | Mesin audio: Oboe + render Rust, panel kesehatan mesin | ⚪ belum |
| M2 | Pad & sampler dasar | ⚪ belum |
| M3 | Sequencer | ⚪ belum |
| M4 | Grup, keys, proyek (MVP) | ⚪ belum |
| M5 | FX & mixer | ⚪ belum |
| M6 | Sampling | ⚪ belum |
| M7 | Lagu & ekspor | ⚪ belum |
| M8 | MIDI | ⚪ belum |
| M9 | Premium & 1.0 | ⚪ belum |

## Mesin (Rust)

Mesin audio ada di folder `engine/` (ADR-0001). Versi Rust dikunci di
`engine/rust-toolchain.toml`, dan rustup memasangnya otomatis.

```text
cd engine
cargo test                          # semua tes
cargo run -p daw-render -- --version
cargo clippy --all-targets -- -D warnings
cargo deny check                    # lisensi & keamanan dependensi (perlu cargo-deny)
```

## Aplikasi (Flutter)

Aplikasi ada di folder `app/`. Versi Flutter dikunci di `app/pubspec.yaml`
(`environment.flutter`, saat ini 3.47.7).

```text
cd engine && cargo build -p daw-ffi && cd ..   # pustaka Rust untuk tes Dart
cd app
flutter pub get --enforce-lockfile
flutter analyze --fatal-infos
flutter test
```

Jembatan Dart ↔ Rust dibuat oleh flutter_rust_bridge 2.13.0 (ADR-0004).
Setelah mengubah `engine/crates/daw-ffi/src/api.rs`, jalankan generator dari
folder `app/` lalu commit hasilnya; CI gagal bila hasilnya basi:

```text
cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked
cargo install cargo-expand --version 1.0.127 --locked
cd app && flutter_rust_bridge_codegen generate
```

## APK Android

Satu pustaka native per ABI, `libmxm_engine.so`, berisi Rust (staticlib dari
cargo-ndk) + adapter C++ (CMake), lihat ADR-0003. Hanya arm64-v8a
(ADR-0009). Butuh Android SDK + NDK (versi di `app/android/app/build.gradle.kts`)
dan `cargo install cargo-ndk --version 4.1.2 --locked`.

```text
tools/build-android.sh                     # Rust → CMake → app-release.apk
tools/cek-apk.sh app/build/app/outputs/flutter-apk/app-release.apk
tools/tes-cek-apk.sh                       # tes untuk pemeriksa APK itu sendiri
```

`cek-apk.sh` gagal bila APK meminta izin di luar `tools/izin-rilis.txt`
(termasuk `INTERNET`, ADR-0006), tidak berisi `libmxm_engine.so`, berisi ABI
lain, tidak mengekspor fungsi yang dicari Dart, atau lebih besar dari 25 MB.
CI menjalankan semuanya dan mengunggah APK sebagai artefak.

## Memasang di HP

Belum ada APK. APK pratinjau pertama terbit di akhir M0 lewat halaman
Releases, dan panduan pemasangannya ditulis di sini.

## Privasi

Build rilis tidak punya izin `INTERNET`, dan CI memeriksanya (ADR-0006).
Proyek dan sampel tidak pernah keluar dari perangkat kecuali Anda sendiri
yang mengekspornya.

## Lisensi

Kode berlisensi **MIT OR Apache-2.0** (lihat `LICENSE-MIT` dan
`LICENSE-APACHE`). Kit pabrik, ikon, dan angka segmen dibuat sendiri dan
dirilis sebagai CC0. Lihat [ADR-0010](docs/adr/0010-lisensi-dan-aset.md).
