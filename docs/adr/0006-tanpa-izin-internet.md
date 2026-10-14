# 0006. Build rilis tanpa izin `INTERNET`, diperiksa CI

- Status: Diterima
- Tanggal: 2026-10-14

## Konteks

MXM-DAW menjanjikan: tanpa akun, tanpa analitik, proyek dan sampel tidak
pernah keluar dari perangkat (rencana §2, §15). Janji di dokumen mudah
dilanggar tanpa sengaja, misalnya oleh dependensi yang diam-diam menambahkan
izin ke manifest.

Di Android, aplikasi tanpa izin `android.permission.INTERNET` tidak bisa
membuka soket jaringan sama sekali. Larangannya ditegakkan oleh sistem
operasi.

Catatan teknis: build *debug* Flutter memerlukan `INTERNET` untuk hot reload
dan debugger. Flutter menaruh izin itu di manifest khusus debug/profile, bukan
di manifest utama.

## Pilihan yang dipertimbangkan

1. **Kebijakan tertulis saja**: mudah, tetapi tidak ada yang memeriksa.
2. **Analitik atau crash reporting "anonim"**: membantu menemukan bug, tetapi
   melanggar janji privasi dan butuh internet. Ditolak.
3. **Tanpa `INTERNET` di build rilis dan pratinjau, diperiksa CI** pada
   manifest akhir di dalam APK (sesudah semua manifest dependensi digabung).

## Keputusan

Pilihan 3. CI membaca daftar izin dari APK rilis/pratinjau yang sudah jadi,
lalu gagal bila ada `INTERNET` atau izin lain di luar daftar yang disetujui.
Daftar izin yang disetujui disimpan di repo dan diubah hanya lewat PR. Bila
sebuah dependensi menambahkan izin terlarang, izin itu dibuang di manifest
kita (`tools:node="remove"`) atau dependensinya ditolak.

## Konsekuensi

- (+) Privasi dijamin oleh sistem operasi, bukan sekadar janji.
- (+) Penambahan izin selalu terlihat di PR.
- (−) Tidak ada laporan crash otomatis. Bug ditemukan lewat tes, panel
  kesehatan mesin, dan laporan pemilik.
- (−) Fitur yang butuh jaringan (mis. sinkronisasi tempo lewat jaringan)
  tidak mungkin tanpa ADR baru.
