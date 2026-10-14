# 0013. Bentuk mesin real-time: antrean bebas-lock, render tanpa alokasi

- Status: Diterima
- Tanggal: 2026-10-14

## Konteks

Di M1, pad di layar harus membunyikan suara lewat thread audio. Thread UI dan
thread audio berjalan bersamaan. Thread audio punya tenggat ±2–5 ms per blok
dan tidak boleh menunggu lock, mengalokasi memori, atau melakukan I/O
(rencana §8.4).

## Pilihan yang dipertimbangkan

1. **Mutex di sekitar mesin**: sederhana, tetapi thread audio bisa menunggu
   UI (*priority inversion*), dan setiap tunggu berisiko underrun.
2. **Variabel atomik per parameter**: cukup untuk satu nilai, tetapi tidak
   untuk urutan event (dua ketukan cepat bisa hilang salah satunya).
3. **Antrean SPSC bebas-lock (`rtrb`)**: UI menulis event, dan thread audio
   membaca semua event di awal setiap blok. Statistik dikirim balik lewat
   atomik.

## Keputusan

Pilihan 3, di `daw-core::engine`:

- `Engine::new(sample_rate)` mengalokasi semuanya (antrean 256 event, 8 suara)
  dan mengembalikan `(Engine, EventSender)`.
- `EventSender::hit(velocity)` tidak pernah menunggu. Bila antrean penuh,
  event dibuang dan dihitung (`hits_dropped`).
- `Engine::render(out, channels)` mengisi buffer interleaved: event diambil di
  awal blok, lalu suara dijumlahkan dan dibatasi ke [−1, 1].
- Tes `tests/tanpa_alokasi.rs` menjalankan `render` di dalam
  `assert_no_alloc`. Satu alokasi saja menghentikan proses tes.
- Event dimainkan di awal blok (selisih ≤ 1 blok, ±2–5 ms). Penempatan
  tepat-sampel datang bersama sequencer di M3.
- Suara M1 adalah kick sintetis (`daw-core::kick`): sinus 150 → 48 Hz, volume
  meluruh τ = 0,18 s, dan rampa awal 1 ms supaya tanpa klik.

## Konsekuensi

- (+) Thread audio tidak pernah menunggu UI.
- (+) Seluruh mesin bisa dites di laptop/CI tanpa HP, termasuk aturan tanpa
  alokasi, dan `daw-render kick` menghasilkan WAV yang sama persis.
- (−) Event dari UI terlambat paling lama satu blok. Hal ini terukur di HP
  sebagai bagian dari latensi sentuh → bunyi (rencana §11).
- (−) Mesin dibebaskan memorinya di luar thread audio. Urutan berhenti/mulai
  stream harus dijaga oleh adapter (ADR-0003, M1 langkah 2).
