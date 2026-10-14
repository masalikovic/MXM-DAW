#!/usr/bin/env bash
# Membangun APK MXM-DAW untuk arm64-v8a (ADR-0003, ADR-0009).
#
#   1. cargo-ndk: Rust (daw-ffi) → engine/target/aarch64-linux-android/release/libdaw_ffi.a
#   2. flutter build apk: Gradle → CMake (Rust + C++ → libmxm_engine.so) → APK
#
# Butuh: Rust (rustup), cargo-ndk, Flutter, Android SDK + NDK (ANDROID_NDK_HOME).
# Saluran dipilih lewat MXMDAW_SALURAN (bawaan: pratinjau; ADR-0012).
# Argumen tambahan diteruskan ke `flutter build apk`, mis. --build-number=7.
set -euo pipefail

akar="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "== 1/2 Rust untuk Android (cargo-ndk, arm64-v8a, API 27)"
cd "$akar/engine"
# --platform 27 = minSdk (ADR-0008).
cargo ndk --target arm64-v8a --platform 27 build --release --locked -p daw-ffi

echo "== 2/2 APK (Flutter + Gradle + CMake)"
cd "$akar/app"
flutter build apk --release --target-platform android-arm64 \
  --flavor "${MXMDAW_SALURAN:-pratinjau}" "$@"
