# 0010. Lisensi kode, dependensi, dan aset

- Status: Diterima
- Tanggal: 2026-10-14

> Catatan: ini keputusan teknis proyek, bukan nasihat hukum.

## Konteks

Repo ini publik. Ada empat jenis materi yang perlu diatur (rencana §15):

1. Kode kita sendiri (Rust, Dart, C++, Kotlin, dokumen).
2. Dependensi pihak ketiga (crate Rust, paket Dart, Oboe).
3. Aset: kit pabrik, ikon, font, angka segmen.
4. Merek dan desain produk lain yang menjadi inspirasi genre.

## Pilihan yang dipertimbangkan

Untuk kode kita:
1. MIT saja. 2. Apache-2.0 saja. 3. **MIT OR Apache-2.0** (konvensi ekosistem
Rust, sama dengan mxm-gameboy).

Untuk dependensi:
1. Bebas, diperiksa manual. 2. **Daftar lisensi yang diizinkan, diperiksa CI.**

Untuk kit pabrik:
1. Sampel CC0 dari internet: cepat, tetapi asal-usul dan izinnya sulit
   dibuktikan. 2. **Dibuat sendiri oleh kode** (ADR-0009 butir 6).

## Keputusan

- **Kode kita:** MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`).
- **Dependensi:** hanya lisensi permisif (MIT, Apache-2.0, BSD, Zlib,
  Unlicense) atau MPL-2.0 yang dipakai tanpa modifikasi. Pustaka GPL (mis.
  Ableton Link, Rubber Band) ditolak kecuali ada ADR khusus. Diperiksa
  `cargo-deny` untuk Rust dan skrip kecil untuk paket Dart. Semua lisensi
  tampil di halaman Tentang.
- **Kit pabrik:** disintesis kode kita saat build, dirilis sebagai CC0. Tidak
  ada sampel, loop, atau preset milik pihak lain.
- **Ikon dan angka segmen:** dibuat sendiri dari kode/piksel, CC0.
- **Font:** Inter dan JetBrains Mono (SIL Open Font License).
- **Merek:** tidak memakai nama, logo, huruf, ikon layar, atau tata letak
  persis produk lain, termasuk produk teenage engineering yang menjadi
  referensi genre (rencana §5.1).

## Konsekuensi

- (+) Repo bebas dari materi yang status hukumnya meragukan.
- (+) Orang lain bebas memakai kode dan kit kita.
- (−) Kit pabrik terbatas pada bunyi yang bisa disintesis, sampai pemilik
  memutuskan merekam sampel sendiri (perlu ADR baru).
- (−) Setiap dependensi baru harus lolos pemeriksaan lisensi di CI.
