# MXM-DAW — aturan untuk agen

## Produk
MXM-DAW adalah sampler-composer Android: pad, sampler, sequencer, FX, mixer,
mode lagu. Bukan DAW timeline desktop, bukan marketplace, tanpa akun, tanpa
internet. Rencana: docs/rencana.md. Keputusan: docs/adr/.
Komunikasi dan dokumentasi dalam bahasa Indonesia.

## Identitas visual
Palet MXM: tinta #0E0E0E, cream #F5EBD7, oranye #FF8A3D, abu-abu tenang,
cokelat #7A3F17 sebagai aksen. Jangan meniru merek, nama, logo, tata letak
persis, atau ikon produk lain.

## Audio
- Thread audio: tanpa alokasi, lock, log, atau I/O.
- Model proyek milik Rust (daw-core); Dart hanya tampilan dan intent.
- Setiap perubahan DSP/sequencer butuh tes. Golden render diperbarui hanya
  setelah hasilnya didengar atau diperiksa.

## Aset
- Kit pabrik, ikon, dan angka segmen dibuat sendiri (CC0); font berlisensi OFL.
- Jangan menambahkan sampel, loop, atau preset milik pihak lain.

## Keamanan dan privasi
- Build rilis tanpa izin INTERNET (CI memeriksa manifest). Tanpa analitik.
  Sampel dan proyek tidak pernah diunggah.
- Izin mikrofon diminta hanya saat pengguna mulai merekam.
- Keystore, kata sandi, dan rahasia tidak pernah ada di commit, log, artefak,
  atau percakapan. Agen tidak membuat atau meminta kunci tanda tangan.

## Proses
- Satu branch per langkah dari main; PR dengan tabel bukti.
- Laporkan terpisah: kode ditulis, tes lulus, CI, emulator, HP pemilik.
- Jangan reset/clean working tree; jangan mengubah application ID atau kunci
  tanda tangan rilis.
- Pemilik yang merge. ADR yang sudah diterima tidak diedit.
