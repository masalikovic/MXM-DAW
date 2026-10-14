# 0014. Sampler: model pad di Rust, sampel dibebaskan di thread UI

- Status: Diterima
- Tanggal: 2026-10-14

## Konteks

M2 mengganti kick tunggal dengan sampler 16 pad. Sampel bisa berukuran
megabyte dan diganti saat aplikasi berjalan (impor WAV). Thread audio tidak
boleh mengalokasi **dan tidak boleh membebaskan** memori, karena keduanya bisa
mengunci di dalam pengalokasi (rencana §8.4). Selain itu, UI perlu membaca
parameter pad (untuk kenop dan layar) tanpa bertanya ke thread audio.

## Pilihan yang dipertimbangkan

1. **Suara memegang `Arc<Sampel>`**: sederhana, tetapi suara terakhir yang
   selesai bisa menjadi pemilik terakhir, sehingga memori dibebaskan di
   thread audio.
2. **Kumpulan sampel tetap dengan indeks**: tanpa `Arc`, tetapi butuh
   pengelolaan slot dan ukuran maksimum sejak awal.
3. **Slot `Arc<Sampel>` per pad + antrean sampah**: suara tidak memegang
   sampel, tetapi membaca slot pad setiap blok. Saat sampel diganti, suara
   pad itu dipotong (dipudarkan 2 ms), lalu `Arc` lama dikirim kembali ke
   UI lewat antrean SPSC kedua dan dibebaskan di sana.

## Keputusan

Pilihan 3, di `daw-core::engine`:

- `EventSender` (thread UI) memegang **model**: parameter 16 pad, sampel, dan
  nama. `Engine` (thread audio) memegang salinan yang diperbarui lewat event
  `SetPad` dan `SetSample`. Nilai disanitasi di sisi UI sebelum dikirim.
- Sampel disimpan sebagai int16 (rencana §9). Kit pabrik juga `Sampel` biasa,
  disintesis saat mesin dibuat (`daw-core::kit`), jadi kit dan sampel impor
  dimainkan oleh jalur yang sama.
- 32 suara. Suara yang dicuri tidak diputus, melainkan sisa bunyinya
  dipudarkan dalam ±2 ms ("ekor"). Ini menutup utang klik dari M1.
- Choke group 1–8: memicu pad memotong pad lain di grup yang sama dengan
  release 5 ms.
- Parameter dibaca ulang setiap blok, jadi kenop yang diputar langsung
  terdengar pada suara yang sedang berbunyi.
- Tes `tests/tanpa_alokasi.rs` sekarang juga mengganti sampel di tengah
  render. Mutasi "bebaskan sampel lama di thread audio" membuat tes itu
  abort.

## Konsekuensi

- Mengganti sampel pad memotong suara pad itu yang sedang berbunyi. Hal ini
  bisa diterima, karena mengganti sampel bukan aksi saat tampil.
- Bila antrean sampah penuh (tidak terjadi selama UI mengirim event), sampel
  lama sengaja dibocorkan, bukan dibebaskan di thread audio.
- Format proyek (M4) cukup menyimpan `PadParams`, karena nama dan satuannya
  sudah mengikuti rencana §10.
