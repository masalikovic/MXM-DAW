# 0012. Saluran pratinjau/rilis dan kunci tanda tangan pratinjau

- Status: Diterima
- Tanggal: 2026-10-14

## Konteks

Pemilik memasang MXM-DAW dari halaman Releases tanpa toolchain (rencana
§13.4). Android hanya mau memasang APK baru di atas yang lama bila tanda
tangannya sama; bila berbeda, aplikasi harus di-uninstall dan semua proyek
ikut hilang (rencana §16). Jadi APK pratinjau harus selalu ditandatangani
kunci yang sama, sejak APK pertama.

Pemilik membuat kunci sendiri dengan `keytool` dan menyimpannya sebagai
GitHub secret (ADR-0009 butir 7). Agen tidak pernah membuat, meminta, atau
melihat kunci maupun kata sandinya.

## Pilihan yang dipertimbangkan

1. **Satu aplikasi, kunci debug CI**: kunci debug dibuat ulang di setiap
   runner, jadi setiap APK berbeda tanda tangan. Ditolak.
2. **Satu aplikasi, kunci pemilik untuk semua build termasuk PR**: kode yang
   belum ditinjau ikut ditandatangani kunci asli. Ditolak.
3. **Dua saluran (flavor Gradle) + kunci pemilik hanya untuk push ke `main`.**

## Keputusan

Pilihan 3:

- Flavor `pratinjau`: ID `io.github.masalikovic.mxmdaw.preview`, label
  "MXM-DAW Pratinjau". Flavor `rilis`: ID `io.github.masalikovic.mxmdaw`
  (untuk tag rilis kelak). Keduanya bisa terpasang berdampingan.
- Kunci dibaca Gradle dari variabel lingkungan yang hanya diisi CI saat push ke
  `main`, dari secret `MXMDAW_PRATINJAU_KEYSTORE_B64`,
  `MXMDAW_PRATINJAU_PASSWORD`, dan `MXMDAW_PRATINJAU_ALIAS`. PR dan build lokal
  memakai kunci debug.
- Sidik jari SHA-256 sertifikat kunci pratinjau (informasi publik) disimpan di
  `tools/sidik-kunci-pratinjau.txt`. `tools/cek-tanda-tangan.sh` menolak APK
  pratinjau yang ditandatangani kunci lain.
- Nomor build = nomor run CI, sehingga selalu naik.
- Setiap push ke `main` yang lulus semua job menerbitkan pre-release
  `preview-<build>` berisi APK + berkas SHA-256.
- Kunci rilis (flavor `rilis`) dibuat terpisah menjelang rilis 1.0, lewat ADR
  baru.

## Konsekuensi

- (+) APK pratinjau selalu bisa dipasang di atas versi sebelumnya.
- (+) Pergantian kunci yang tidak disengaja langsung membuat CI merah.
- (−) Bila pemilik kehilangan keystore atau kata sandinya, APK pratinjau baru
  tidak bisa dipasang di atas yang lama. Mitigasi: cadangan di dua tempat
  (docs/panduan-kunci-pratinjau.md) dan ekspor `.mxmdaw` (M7).
- (−) Sebelum secret diatur, `main` tidak menerbitkan APK (CI memberi
  peringatan, bukan diam).
