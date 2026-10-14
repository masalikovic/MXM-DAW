# 0003. Oboe (C++) sebagai adapter I/O audio, digabung dengan Rust dalam satu `.so`

- Status: Diterima
- Tanggal: 2026-10-14

## Konteks

Mesin Rust (ADR-0001) butuh cara membuka stream audio rendah-latensi di
Android. Android menyediakan AAudio (API 26+) dan OpenSL ES (lama). Banyak
perangkat punya bug atau perilaku khusus pada keduanya. Oboe adalah pustaka C++
dari Google (Apache-2.0) yang memilih API terbaik per perangkat dan memuat
penanganan bug yang dikumpulkan bertahun-tahun.

Selain itu, ada pertanyaan bentuk build: berapa banyak pustaka native yang
dimuat aplikasi?

## Pilihan yang dipertimbangkan

1. **AAudio langsung dari Rust** (crate `ndk` atau `cpal`): satu bahasa saja,
   tetapi penanganan kasus perangkat yang aneh jauh lebih sedikit.
2. **Binding Rust ke Oboe** (crate pihak ketiga): tetap Oboe, tetapi
   menambah satu lapisan binding yang perawatannya di luar kendali kita.
3. **Adapter C++ kecil memakai Oboe**, memanggil fungsi render Rust lewat
   antarmuka C. Rust dibangun sebagai `staticlib`, lalu CMake menggabungkan
   keduanya menjadi **satu** `libmxm_engine.so`.
4. Seperti pilihan 3, tetapi Rust dan C++ menjadi **dua** `.so` terpisah.

## Keputusan

Pilihan 3. Adapter C++ hanya membuka stream, meneruskan callback ke Rust, dan
melaporkan latensi serta underrun. Tidak ada DSP di C++. Pustaka yang
dihasilkan bernama `libmxm_engine.so`, satu per ABI. flutter_rust_bridge
(ADR-0004) dan Kotlin memuat pustaka yang sama.

Dibanding pilihan 4, satu `.so` berarti hanya satu salinan state mesin di
memori, tanpa urutan pemuatan pustaka yang harus dijaga, dan pemanggilan
C++ → Rust tetap di dalam satu pustaka.

## Konsekuensi

- (+) Mendapat ketahanan Oboe di banyak perangkat Android.
- (+) Bagian C++ kecil, sehingga hampir semua logika tetap dites di Rust.
- (−) Integrasi build bawaan flutter_rust_bridge (yang biasanya membangun
  `cdylib` Rust sendiri) tidak dipakai apa adanya. Kita butuh skrip build
  sendiri: cargo-ndk → `staticlib` → CMake → `.so`.
- (−) Bentuk build ini dibuktikan sejak M0 (dengan berkas C++ kecil tanpa
  Oboe), lalu Oboe sendiri baru masuk di M1.
- Bisa ditinjau ulang lewat ADR baru bila jalur Rust murni terbukti cukup.
