# 0017. Undo/redo dan salin/tempel

- Status: Diterima
- Tanggal: 2026-10-15

## Konteks

Rencana §4.3 meminta undo/redo 100 langkah dan salin/tempel pad, grup, dan
pola. Perubahan datang dari banyak tempat (kenop, grid, rekaman, impor,
mute), dan thread audio memegang salinan model sendiri (ADR-0015).

## Pilihan yang dipertimbangkan

1. **Perintah terbalik** (setiap aksi tahu cara membatalkan dirinya): hemat
   memori, tetapi setiap aksi baru harus menulis kebalikannya, dan satu
   kebalikan yang salah merusak riwayat diam-diam.
2. **Salinan model per langkah**: dipilih. Sampel dipegang `Arc`, jadi
   salinan hanya berisi parameter, nama, dan not (beberapa KB). 100 langkah
   tetap kecil.

## Keputusan

- Riwayat (`daw_core::riwayat::History`) tinggal di sisi UI Rust (daw-ffi),
  bukan di Dart, supaya semua perubahan, termasuk yang kelak datang dari
  layar lain, lewat satu pintu.
- **Satu gerakan = satu langkah.** Perubahan yang berjarak < 0,7 detik
  (memutar kenop, menggeser swing) digabung. Aksi tunggal (impor, tempel,
  mute, mulai rekam) selalu membuka langkah baru dan tidak menelan gerakan
  sesudahnya. Satu sesi rekaman = satu langkah, dicatat saat REC ditekan.
- **Undo = menerapkan model lama.** `EventSender::apply_model` hanya
  mengirim bagian yang berbeda (sampel dibandingkan dengan `Arc::ptr_eq`),
  lewat antrean yang sama, jadi thread audio tetap tanpa alokasi.
- Membuka proyek mengosongkan riwayat. Riwayat tidak disimpan ke disk;
  hasil undo tersimpan seperti perubahan biasa (ADR-0016).
- **Salin/tempel**: suara pad dan grup disalin Rust saat ditempel (sampel
  dipakai bersama, tidak digandakan). Pola disalin sebagai isi di papan klip
  Dart, supaya bisa ditempel ke grup lain. Papan klip hanya di memori.

## Konsekuensi

- Tempel suara/grup mengambil keadaan sumber **saat ditempel**, bukan saat
  disalin. Bila sumber diubah di antaranya, yang ditempel versi terbaru.
- Sampel yang hanya dipegang riwayat tetap di RAM sampai langkahnya keluar
  dari 100 langkah. Batas RAM sampel (ADR-0014) belum menghitungnya.
- Penggabungan 0,7 detik berdasarkan waktu, bukan akhir sentuhan: dua
  kenop berbeda yang diputar sangat cepat berurutan menjadi satu langkah.
