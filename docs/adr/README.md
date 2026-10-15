# Architecture Decision Records (ADR)

ADR adalah catatan pendek tentang **satu** keputusan arsitektur: konteksnya,
pilihan yang dipertimbangkan, apa yang diputuskan, dan konsekuensinya.

## Kenapa memakai ADR

Kode menjelaskan *apa* yang dilakukan sistem, tetapi jarang menjelaskan
*kenapa* bentuknya begitu dan alternatif apa yang ditolak. Alasan itu cepat
terlupa, baik oleh pemilik maupun oleh sesi agen berikutnya yang mulai tanpa
ingatan percakapan sebelumnya. Cara ini sama dengan mxm-gameboy (ADR-0001 di
repo itu).

## Aturan

- Satu berkas per keputusan: `NNNN-judul-pendek.md`, nomor naik terus.
- ADR masuk lewat PR. **ADR dianggap diterima saat PR-nya di-merge pemilik.**
- ADR yang sudah diterima **tidak diedit**. Bila keputusan berubah, tulis ADR
  baru dan isi bagian "Digantikan oleh" di daftar di bawah.
- Keputusan kecil (nama variabel, format kode) tidak perlu ADR.
- Catatan: ini keputusan teknis proyek, bukan nasihat hukum.

## Daftar

ADR 0001–0008 berasal dari tabel keputusan inti di
[rencana §8.2](../rencana.md#82-keputusan-inti-calon-adr-di-m0).

| No | Judul | Status |
|---|---|---|
| [0001](0001-mesin-audio-di-rust.md) | Mesin audio ditulis dalam Rust, bukan Dart | Diterima |
| [0002](0002-rust-memegang-model-proyek.md) | Rust memegang model proyek; Dart hanya tampilan dan intent | Diterima |
| [0003](0003-oboe-sebagai-adapter-io.md) | Oboe (C++) sebagai adapter I/O audio, digabung dengan Rust dalam satu `.so` | Diterima |
| [0004](0004-flutter-rust-bridge.md) | flutter_rust_bridge 2 sebagai jembatan Dart ↔ Rust | Diterima |
| [0005](0005-riverpod.md) | Riverpod dan state tak berubah untuk UI | Diterima |
| [0006](0006-tanpa-izin-internet.md) | Build rilis tanpa izin `INTERNET`, diperiksa CI | Diterima |
| [0007](0007-format-proyek.md) | Proyek disimpan sebagai JSON berversi + sampel WAV | Diterima |
| [0008](0008-minsdk-27.md) | minSdk 27 (Android 8.1) | Diterima |
| [0009](0009-jawaban-pemilik-m0.md) | Batasan produk dari pemilik (jawaban §17) | Diterima |
| [0010](0010-lisensi-dan-aset.md) | Lisensi kode, dependensi, dan aset | Diterima |
| [0011](0011-token-warna-dan-kontras.md) | Token warna, warna grup C/D, dan tes kontras | Diterima |
| [0012](0012-saluran-dan-kunci-pratinjau.md) | Saluran pratinjau/rilis dan kunci tanda tangan pratinjau | Diterima |
| [0013](0013-mesin-real-time.md) | Bentuk mesin real-time: antrean bebas-lock, render tanpa alokasi | Diterima |
| [0014](0014-sampler-dan-memori-sampel.md) | Sampler: model pad di Rust, sampel dibebaskan di thread UI | Diterima |
| [0015](0015-sequencer.md) | Sequencer: pola mentah di UI, jadwal tepat-sampel di thread audio | Diterima |
| [0016](0016-proyek-dan-simpan-otomatis.md) | Berkas proyek dan simpan otomatis | Diterima |

## Template

Salin blok ini ke berkas baru `NNNN-judul-pendek.md`:

```markdown
# NNNN. Judul keputusan

- Status: Diterima | Digantikan oleh NNNN
- Tanggal: YYYY-MM-DD

## Konteks
Masalah apa yang sedang dihadapi? Batasan apa yang berlaku?

## Pilihan yang dipertimbangkan
1. Pilihan A: plus/minus
2. Pilihan B: plus/minus

## Keputusan
Kita memilih ... karena ...

## Konsekuensi
Apa yang jadi lebih mudah, apa yang jadi lebih sulit, apa yang perlu dipantau.
```
