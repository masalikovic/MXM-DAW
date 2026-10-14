# 0007. Proyek disimpan sebagai JSON berversi + sampel WAV

- Status: Diterima
- Tanggal: 2026-10-14

## Konteks

Proyek musik adalah hasil kerja berjam-jam. Kehilangannya adalah kegagalan
terburuk aplikasi ini (rencana §16). Format penyimpanan juga akan berubah
seiring tahap M3–M9 menambah fitur. mxm-gameboy mengajarkan bahwa data lama
harus bisa dibaca versi baru, dan data dari versi yang lebih baru harus
ditolak dengan jelas, bukan dibaca salah.

## Pilihan yang dipertimbangkan

1. **Database biner** (mis. SQLite): kueri cepat, tetapi tidak bisa dibaca
   manusia, dan sampel besar tetap butuh berkas terpisah.
2. **Format biner sendiri**: ringkas, tetapi sulit diperiksa saat ada bug dan
   sulit dimigrasi.
3. **JSON berversi** (`project.json`, kolom `schema`) **+ sampel WAV** dalam
   folder `samples/`. Ekspor/impor sebagai `.mxmdaw` (zip berisi keduanya).

## Keputusan

Pilihan 3, dengan aturan:

- `schema` naik setiap format berubah. Migrasi hanya **menambah**, tidak
  pernah menghapus data pengguna.
- Proyek dengan `schema` lebih baru dari yang dikenal aplikasi ditolak dengan
  pesan jelas.
- Simpan otomatis ditunda 500 ms sesudah perubahan terakhir, ditulis ke berkas
  sementara, lalu diganti nama secara atomik.
- Posisi not disimpan seperti saat direkam. Kuantisasi disimpan sebagai
  pengaturan pola, lalu diterapkan saat kompilasi (tidak merusak).

## Konsekuensi

- (+) Bisa dibuka dan diperiksa dengan editor teks; mudah dibandingkan di tes.
- (+) Aplikasi yang dimatikan paksa tidak merusak proyek.
- (+) `.mxmdaw` bisa dipindah ke HP lain atau dicadangkan.
- (−) JSON lebih besar dari biner. Untuk ukuran proyek kita (ratusan not per
  pola) hal ini tidak terasa, karena sebagian besar ukuran ada di sampel.
- (−) Setiap perubahan format butuh migrasi dan tes migrasi.
