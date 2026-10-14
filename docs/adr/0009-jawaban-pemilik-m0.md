# 0009. Batasan produk dari pemilik (jawaban §17)

- Status: Diterima
- Tanggal: 2026-10-14

## Konteks

Rencana §17 berisi delapan pertanyaan yang hanya bisa dijawab pemilik.
Pemilik menjawab semuanya di awal M0. Jawabannya pendek dan saling terkait,
jadi dicatat bersama dalam satu ADR. Bila salah satunya berubah nanti, yang
berubah ditulis sebagai ADR baru yang menggantikan butir itu saja.

## Keputusan

| # | Pertanyaan | Jawaban pemilik | Akibat langsung |
|---|---|---|---|
| 1 | Nama & application ID | **MXM-DAW**, `io.github.masalikovic.mxmdaw`; pratinjau `io.github.masalikovic.mxmdaw.preview` | ID tidak boleh diubah lagi (AGENTS.md). Pratinjau dan rilis bisa terpasang berdampingan |
| 2 | Perangkat acuan | **Redmi 13C**, MIUI Global 14.0.5 (14.0.5.0.TGPIDXM), **Android 13** (API 33, build TP1A.220624.014), patch keamanan 2024-01-01 | Semua angka §11 diukur di HP ini |
| 3 | Orientasi | **Pocket (potret) dulu**, Studio (lanskap) di M9 | Sampai M9 aplikasi dikunci potret, supaya tata letak yang belum siap untuk lanskap tidak pernah tampil |
| 4 | Putar di latar | **Tidak perlu dulu** | Tanpa *foreground service* dan tanpa izin terkaitnya. Stream berhenti saat aplikasi masuk latar dan tidak memutar (cocok dengan target baterai §11) |
| 5 | Kontroler MIDI USB | **Belum punya** | Bukti M8 tidak bisa memakai kontroler fisik. Bentuk buktinya ditentukan menjelang M8 dan ditanyakan ke pemilik |
| 6 | Kit pabrik | **Disintesis seluruhnya** | Kit dibuat oleh kode (`daw-render`) saat build, tanpa berkas audio biner di repo, CC0 (ADR-0010) |
| 7 | Kunci tanda tangan | **Dibuat pemilik dengan `keytool`, disimpan sebagai GitHub secret** | Agen hanya menulis panduan dan workflow yang membaca secret. Agen tidak pernah membuat, meminta, atau melihat kunci maupun kata sandinya |
| 8 | ABI | **arm64-v8a saja** | APK pratinjau dan rilis hanya berisi arm64-v8a. CI boleh membangun x86_64 khusus untuk tes emulator (rencana §13.3), tetapi hasilnya tidak dirilis |

## Konsekuensi

- (+) Ruang lingkup M0–M8 lebih kecil: satu orientasi, satu ABI rilis, tanpa
  layanan latar.
- (+) APK lebih kecil karena hanya satu ABI.
- (−) Redmi 13C adalah HP kelas terjangkau. Target latensi §11 (≤ 20 ms) belum
  tentu tercapai karena bergantung pada jalur audio di firmware vendor. Angka
  dicatat apa adanya sejak M1.
- (−) HP Android 32-bit (armeabi-v7a) tidak bisa memasang aplikasi.
- Yang perlu dipantau: MIUI biasanya meminta izin tambahan untuk memasang APK
  dari luar Play Store. Panduannya ditulis saat APK pratinjau pertama terbit
  (langkah M0 terakhir).
