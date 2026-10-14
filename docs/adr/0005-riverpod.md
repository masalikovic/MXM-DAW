# 0005. Riverpod dan state tak berubah untuk UI

- Status: Diterima
- Tanggal: 2026-10-14

## Konteks

Layar MXM-DAW punya banyak bagian kecil yang berubah dengan ritme berbeda:
LED dan meter 60 kali per detik, halaman pola saat not berubah, pengaturan
jarang sekali. Bila satu perubahan membuat seluruh layar dibangun ulang,
frame UI melewati anggaran 8 ms (rencana §11).

Model proyek sendiri ada di Rust (ADR-0002). State di Dart hanya untuk
tampilan dan hasil baca dari mesin.

## Pilihan yang dipertimbangkan

1. **`setState` + `InheritedWidget` bawaan**: tanpa dependensi, tetapi sulit
   mengatur siapa yang dibangun ulang saat aplikasi membesar.
2. **Bloc**: terstruktur, tetapi butuh banyak kelas event/state untuk kasus
   yang sebagian besar hanya "baca snapshot, tampilkan".
3. **Riverpod** dengan objek state tak berubah: setiap widget berlangganan
   tepat ke bagian yang ia perlukan; provider mudah diganti palsu saat tes.

## Keputusan

Pilihan 3. State UI berupa objek tak berubah. Widget memakai `select` agar
hanya dibangun ulang saat bagian yang ia baca berubah. Meter, LED, dan
playhead juga dibungkus `RepaintBoundary` (rencana §5.4).

## Konsekuensi

- (+) Rebuild terkendali dan bisa diperiksa di Flutter DevTools.
- (+) Tes widget bisa memakai mesin palsu tanpa Rust.
- (−) Satu konsep tambahan yang perlu dipelajari (provider, `ref.watch`,
  `select`).
