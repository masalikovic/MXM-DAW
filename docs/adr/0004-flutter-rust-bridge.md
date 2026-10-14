# 0004. flutter_rust_bridge 2 sebagai jembatan Dart ↔ Rust

- Status: Diterima
- Tanggal: 2026-10-14

## Konteks

Dart perlu memanggil Rust dengan dua pola berbeda:

- **Sinkron dan sangat cepat:** pad ditekan → event masuk antrean mesin. Ini
  harus langsung, tanpa lompat ke isolate lain (rencana §7).
- **Aliran data:** snapshot 60 Hz (playhead, meter, LED) dari Rust ke Dart.

Menulis lem FFI dengan tangan untuk banyak fungsi rawan salah tipe dan salah
urus memori.

## Pilihan yang dipertimbangkan

1. **`dart:ffi` manual** (+ `ffigen` untuk header C): kendali penuh, tetapi
   semua konversi tipe, string, dan stream ditulis sendiri.
2. **flutter_rust_bridge 2**: binding Dart dibuat otomatis dari tanda tangan
   fungsi Rust; mendukung fungsi `sync` dan `Stream`.

## Keputusan

Pilihan 2. Fungsi pad dan parameter ditandai sinkron. Snapshot dikirim lewat
stream. Kode hasil generator di-commit, dan CI memeriksa bahwa kode itu tidak
basi (generator dijalankan ulang, lalu hasilnya harus sama persis).

## Konsekuensi

- (+) Jauh lebih sedikit kode lem yang ditulis tangan.
- (+) Tipe Rust dan Dart selalu cocok, karena dibuat dari sumber yang sama.
- (−) Versi crate Rust dan paket Dart flutter_rust_bridge harus selalu sama;
  keduanya dipin.
- (−) Bergantung pada alat pihak ketiga. Bila suatu saat tidak cocok, lapisan
  `daw-ffi` cukup tipis untuk diganti `dart:ffi` manual lewat ADR baru.
- Lihat ADR-0003: pustaka yang dimuat adalah `libmxm_engine.so`, bukan
  `cdylib` bawaan generator.
