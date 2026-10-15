# 0016. Berkas proyek dan simpan otomatis

- Status: Diterima
- Tanggal: 2026-10-15

## Konteks

Bukti M4: proyek harus tetap utuh setelah aplikasi ditutup paksa lalu dibuka
lagi. Android boleh mematikan aplikasi di latar kapan saja tanpa memberi
tahu. ADR-0007 sudah memilih JSON berversi + sampel WAV. Yang belum
diputuskan: di mana berkas itu tinggal, kapan ditulis, dan bagaimana supaya
tidak pernah setengah tertulis atau tertimpa oleh versi yang salah.

## Pilihan yang dipertimbangkan

1. **Simpan saat aplikasi ke latar saja**: sederhana, tetapi aplikasi yang
   mogok atau dimatikan paksa sebelum sempat ke latar kehilangan semuanya.
2. **Simpan setiap perubahan langsung**: aman, tetapi memutar kenop bisa
   menghasilkan puluhan penulisan per detik.
3. **Revisi model + simpan berkala (500 ms) + simpan saat ke latar**: dipilih.

## Keputusan

- **Tempat**: folder data pribadi aplikasi (`filesDir/proyek`), tanpa izin
  penyimpanan. Isinya ikut terhapus saat aplikasi di-uninstall.
- **Isi**: `project.json` + `samples/s<id>.wav` (int16, sama persis dengan
  format di memori). Kit pabrik hanya disimpan sebagai rujukan
  `factory:NAMA`. Sampel yang tidak dirujuk lagi dihapus setelah JSON baru
  tersimpan.
- **Format** ada di `daw-io::proyek`, sebagai struktur serde **terpisah** dari
  tipe `daw-core`. Setiap kolom punya nilai bawaan, dan kolom asing
  diabaikan. Berkas `tests/fixtures/proyek-skema-1.json` di repo harus selalu
  terbaca.
- **Penulisan atomik**: `*.tmp` → `fsync` → ganti nama.
- **Revisi**: `EventSender::changes()` naik setiap model berubah, termasuk
  rekaman sequencer. FFI menambahkan perubahan tanpa mesin, sehingga revisi
  tidak pernah mundur. Dart (`SimpanOtomatis`) membandingkan revisi setiap
  500 ms, menyimpan paling banyak satu kali bersamaan, dan menyimpan
  seketika saat aplikasi ke latar.
- **Buka** dilakukan sebelum mesin audio mulai.
  - Proyek **rusak** dipindah ke `project.rusak.json`, tidak ditimpa.
  - Proyek dari **versi lebih baru** ditolak, dan simpan **diblokir**
    supaya berkas itu tidak tertimpa kit pabrik.

## Konsekuensi

- Perubahan terakhir paling banyak ±0,5 detik bisa hilang bila aplikasi
  mati tepat saat itu.
- Satu proyek saja untuk sekarang. Banyak proyek, ekspor `.mxmdaw`, dan
  berbagi datang di M7.
- Undo/redo (M4 langkah 4) bekerja pada model yang sama, jadi otomatis ikut
  tersimpan.
