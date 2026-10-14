# MXM-DAW

*Sampler-composer* genggam untuk Android: pad, sampler, sequencer, efek,
mixer, dan mode lagu dalam satu aplikasi yang terasa seperti alat musik
fisik. Tampilan dibangun dengan **Flutter**, mesin audio dengan **Rust**.
Tanpa akun, tanpa internet.

Proyek ini juga **proyek belajar arsitektur perangkat lunak**: setiap
keputusan dicatat, dan setiap tahap punya bukti selesai yang bisa diperiksa.

## Mulai membaca dari sini

1. [Rencana lengkap](docs/rencana.md): visi, fitur, desain, arsitektur,
   tahapan.
2. [Architecture Decision Records](docs/adr/README.md): keputusan dan
   alasannya.
3. [AGENTS.md](AGENTS.md): aturan untuk agen yang ikut mengerjakan repo ini.

## Status

| Tahap | Isi | Status |
|---|---|---|
| M0 | Fondasi: ADR, CI, token desain, aplikasi kosong yang memanggil Rust, APK pratinjau | 🟡 berjalan |
| M1 | Mesin audio: Oboe + render Rust, panel kesehatan mesin | ⚪ belum |
| M2 | Pad & sampler dasar | ⚪ belum |
| M3 | Sequencer | ⚪ belum |
| M4 | Grup, keys, proyek (MVP) | ⚪ belum |
| M5 | FX & mixer | ⚪ belum |
| M6 | Sampling | ⚪ belum |
| M7 | Lagu & ekspor | ⚪ belum |
| M8 | MIDI | ⚪ belum |
| M9 | Premium & 1.0 | ⚪ belum |

## Memasang di HP

Belum ada APK. APK pratinjau pertama terbit di akhir M0 lewat halaman
Releases, dan panduan pemasangannya ditulis di sini.

## Privasi

Build rilis tidak punya izin `INTERNET`, dan CI memeriksanya (ADR-0006).
Proyek dan sampel tidak pernah keluar dari perangkat kecuali Anda sendiri
yang mengekspornya.

## Lisensi

Kode berlisensi **MIT OR Apache-2.0** (lihat `LICENSE-MIT` dan
`LICENSE-APACHE`). Kit pabrik, ikon, dan angka segmen dibuat sendiri dan
dirilis sebagai CC0. Lihat [ADR-0010](docs/adr/0010-lisensi-dan-aset.md).
