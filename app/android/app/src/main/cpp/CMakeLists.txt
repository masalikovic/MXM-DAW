# libmxm_engine.so: satu pustaka native per ABI (ADR-0003).
#
#   libdaw_ffi.a (Rust, dibangun cargo-ndk)  ─┐
#                                             ├─▶ libmxm_engine.so ─▶ APK
#   mxm_engine_adapter.cpp (C++)             ─┘
#
# Rust dibangun lebih dulu oleh tools/build-android.sh. Adapter C++ membuka
# stream audio Oboe dan memanggil fungsi render Rust (mxm_engine_render).
cmake_minimum_required(VERSION 3.22)
project(mxm_engine LANGUAGES CXX)

# ABI Android → target Rust. Hanya arm64-v8a yang dirilis (ADR-0009 butir 8).
if(ANDROID_ABI STREQUAL "arm64-v8a")
  set(RUST_TARGET "aarch64-linux-android")
else()
  message(FATAL_ERROR "ABI ${ANDROID_ABI} belum didukung (ADR-0009 butir 8).")
endif()

# Rust selalu dibangun dengan profil release, juga untuk APK debug:
# mesin audio yang lambat tidak berguna untuk dicoba di HP.
set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../../../..")
set(RUST_LIB "${REPO_ROOT}/engine/target/${RUST_TARGET}/release/libdaw_ffi.a")
if(NOT EXISTS "${RUST_LIB}")
  message(FATAL_ERROR
    "Pustaka Rust tidak ditemukan: ${RUST_LIB}\n"
    "Bangun dulu dengan tools/build-android.sh (memakai cargo-ndk).")
endif()

add_library(daw_ffi STATIC IMPORTED)
set_target_properties(daw_ffi PROPERTIES IMPORTED_LOCATION "${RUST_LIB}")

# Oboe (Apache-2.0) dari paket Maven com.google.oboe:oboe lewat Prefab
# (app/android/app/build.gradle.kts).
find_package(oboe REQUIRED CONFIG)

add_library(mxm_engine SHARED mxm_engine_adapter.cpp)

# --whole-archive: tanpa ini linker membuang semua fungsi Rust yang tidak
# dipanggil dari C++, termasuk fungsi flutter_rust_bridge yang dicari Dart
# lewat namanya saat aplikasi berjalan.
target_link_libraries(mxm_engine
  PRIVATE
    "-Wl,--whole-archive" daw_ffi "-Wl,--no-whole-archive"
    oboe::oboe
    # Dibutuhkan pustaka standar Rust dan android_logger.
    log dl m
)
