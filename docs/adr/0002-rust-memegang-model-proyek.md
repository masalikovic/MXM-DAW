# 0002. Rust memegang model proyek; Dart hanya tampilan dan intent

- Status: Diterima
- Tanggal: 2026-10-14

## Konteks

Sebuah proyek (BPM, grup, pad, pola, adegan, lagu) dibaca oleh banyak bagian:
mesin audio saat memutar, ekspor WAV/MIDI, simpan otomatis, undo/redo, tes
golden di CI, dan tampilan di layar. Bila ada dua salinan model yang sama-sama
bisa diubah, keduanya cepat atau lambat akan berbeda isi.

## Pilihan yang dipertimbangkan

1. **Model di Dart, salinan di Rust**: nyaman untuk UI, tetapi ada dua sumber
   kebenaran yang harus disinkronkan di setiap perubahan, dan CLI Rust tidak
   bisa membuka proyek tanpa Dart.
2. **Model di Rust** (`daw-core`): UI mengirim *intent* ("ubah pitch pad A3
   menjadi +2"), Rust mengubah `Project`, mencatat undo, lalu mengompilasinya
   menjadi `RtState` untuk thread audio.

## Keputusan

Pilihan 2. `Project`, undo/redo, serde, dan migrasi skema ada di Rust. Dart
menyimpan state tampilan saja (halaman aktif, pad terpilih, dll.) dan membaca
data proyek lewat snapshot atau permintaan berversi (rencana §8.1).

## Konsekuensi

- (+) Satu sumber kebenaran. CLI `daw-render` bisa merender berkas proyek yang
  sama persis dengan yang dimainkan di HP.
- (+) Undo/redo dan migrasi dites di Rust tanpa Flutter.
- (−) Setiap perubahan dari UI melewati FFI. Data halaman diminta ulang hanya
  bila nomor versinya berubah, supaya Dart tidak menyalin seluruh proyek.
- (−) Butuh disiplin: logika model tidak boleh "bocor" ke Dart.
