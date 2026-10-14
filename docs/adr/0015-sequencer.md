# 0015. Sequencer: pola mentah di UI, jadwal tepat-sampel di thread audio

- Status: Diterima
- Tanggal: 2026-10-14

## Konteks

M3 menambah sequencer 960 PPQN (rencana §4.3, §9). Ada empat syarat:

1. Not harus jatuh tepat di sampelnya dan tidak melenceng walau dimainkan
   ribuan bar.
2. Kuantisasi tidak boleh merusak rekaman asli.
3. Thread audio tetap tanpa alokasi, lock, atau pembebasan memori.
4. Pola harus bertahan saat aplikasi ke latar (ADR-0014).

## Pilihan yang dipertimbangkan

1. **Thread audio memegang pola dan mengubahnya saat merekam**: rekaman
   langsung masuk, tetapi menambah not berarti mengalokasi `Vec` di thread
   audio.
2. **Posisi dijumlahkan per blok (`tick += frame × tick_per_frame`)**:
   sederhana, tetapi galat pembulatan menumpuk.
3. **Model di UI, hasil kompilasi ke audio, posisi dari jangkar**: dipilih.

## Keputusan

- **Model** (`daw-core::seq::Sequence`: `Pattern` + `SeqSettings`) dipegang
  `EventSender` di thread UI. Not disimpan dengan tick **mentah**.
- **Kompilasi** (`seq::compile`) menerapkan kuantisasi (`grid`, `strength`)
  dan swing, lalu mengurutkan aksi On/Off. Hasilnya `Arc<CompiledPattern>`
  dikirim lewat antrean event. Pola lama kembali lewat antrean sampah,
  persis seperti sampel.
- **Posisi** = `jangkar_tick + frame × tick_per_frame`, dengan `frame` berupa
  bilangan bulat. Ganti tempo memindahkan jangkar, sehingga posisi tidak
  melompat. BPM dan swing dibulatkan ke presisi aslinya (0,1 BPM; 0,1 %),
  karena sisa f32 terbukti menggeser not satu sampel setelah 100 bar.
- **Jadwal per blok**: kandidat aksi dicari di rentang tick yang sedikit
  lebih lebar. Blok mana yang memainkannya ditentukan oleh **frame**-nya
  (`frame_untuk(tick)`), lalu render dipecah tepat di offset itu. Klik
  metronom dijadwalkan dengan cara yang sama.
- **Rekaman**: ketukan saat merekam dikirim balik lewat antrean ketiga
  (`Recorded`, tick absolut). UI memasukkannya ke pola, setelah dimundurkan
  sebesar latensi keluaran: pemain mengetuk mengikuti bunyi yang terdengar.
- **Count-in** dimulai dari tick negatif. Metronom berbunyi hanya saat
  merekam; not pola dan rekaman baru berlaku mulai tick 0.

## Konsekuensi

- Tes timing: 300 not dalam 100 bar pada 123,4 BPM jatuh tepat di sampel
  yang dihitung rumus. Ukuran blok 37 atau 256 menghasilkan bit yang sama.
- Tes golden (`daw-render/tests/golden.rs`) membandingkan RMS per 100 ms dari
  beat demo, bukan hash, supaya perbedaan satu bit `sin`/`exp` antar sistem
  tidak membuatnya gagal.
- Ketukan dari layar masih dimainkan di awal blok (selisih ≤ 1 blok, ±5 ms);
  hanya not dari pola yang tepat-sampel.
- Rekaman memakai posisi di awal blok saat ketukan diproses, jadi galatnya
  ≤ 1 blok ditambah variasi latensi sentuh. Kuantisasi menutup sisanya.
