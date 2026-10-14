# 0001. Mesin audio ditulis dalam Rust, bukan Dart

- Status: Diterima
- Tanggal: 2026-10-14

## Konteks

Sistem audio Android meminta buffer baru secara berkala lewat *callback*. Pada
48 kHz dengan *burst* 96–256 frame, callback datang setiap ±2–5 ms. Bila
callback terlambat sekali saja, terdengar klik atau putus (*underrun*).
Sequencer juga harus menaruh setiap not tepat di frame yang benar
(rencana §9).

Flutter menjalankan kode Dart di isolate yang juga menggambar UI. Dart memakai
*garbage collector*, jadi bisa berhenti sejenak kapan saja. Itu tidak masalah
untuk animasi, tetapi fatal untuk tenggat 2–5 ms yang tidak boleh terlewat.

## Pilihan yang dipertimbangkan

1. **Paket pemutar Flutter** (mis. pemutar sampel berbasis platform channel):
   cepat untuk mulai, tetapi tidak bisa menjadwalkan bunyi tepat ke sampel,
   dan setiap sentuhan melewati beberapa lapisan pesan.
2. **Mesin dalam C++** bersama Oboe: real-time dan matang, tetapi tanpa
   jaminan keamanan memori, dan alat tesnya lebih repot di CI.
3. **Mesin dalam Rust**, dipanggil dari callback native: tanpa GC, aman memori,
   `cargo test` berjalan di mesin mana pun tanpa HP, dan sama dengan bahasa
   inti mxm-gameboy.
4. **Kotlin/Java**: tetap memakai GC, jadi masalahnya sama dengan Dart.

## Keputusan

Pilihan 3. Sequencer, voice, DSP, FX, dan mixer ditulis dalam Rust. Callback
audio hanya memanggil fungsi render Rust. Dart tidak pernah berada di jalur
audio.

## Konsekuensi

- (+) Mesin bisa dites dan dirender offline (golden, benchmark) tanpa
  perangkat audio, di CI Linux biasa.
- (+) Aturan thread audio (tanpa alokasi, lock, I/O) bisa dipaksakan oleh tes
  (`assert_no_alloc`, rencana §12).
- (−) Ada tiga bahasa di satu aplikasi (Dart, Rust, C++), jadi build lebih
  rumit. Diatasi dengan satu skrip build dan CI sejak M0 (rencana §16).
- (−) Rust harus dikompilasi silang untuk Android (cargo-ndk).
