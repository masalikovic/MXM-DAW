# MXM-DAW — Rencana Lengkap

> **MXM-DAW** adalah *sampler-composer* genggam untuk Android: pad, sampler,
> sequencer, efek, mixer, dan mode lagu dalam satu aplikasi yang terasa seperti
> alat musik fisik. Tampilan dibangun dengan Flutter, mesin audio dengan Rust.
> Prosesnya sama seperti
> [mxm-gameboy](https://github.com/Masalikovic/mxm-gameboy): visi → ADR →
> tahap kecil dengan bukti → jurnal.

- Status dokumen: **rencana awal v1 (2026-10-14)**, ditinjau pemilik sebelum M0.
- Bahasa: Indonesia; istilah audio tetap dalam bahasa Inggris (lihat
  [Glosarium](#19-glosarium)).
- Repo: publik, `Masalikovic/mxm-daw`, dibuat oleh pemilik (lihat
  [§18](#18-memulai-di-sesi-baru)).

## Ringkasan satu menit

- **Apa:** groovebox saku: 4 grup × 16 pad, sampler dengan chop, sequencer
  dengan swing dan timing correct, FX, mixer, adegan → lagu, ekspor
  WAV/stem/MIDI.
- **Bagaimana:** Flutter hanya untuk tampilan. Mesin audio, sequencer, dan
  model proyek ditulis dalam Rust (deterministik, bisa dites tanpa HP), lalu
  diputar lewat Oboe/AAudio di thread audio native.
- **Kenapa premium:** latensi rendah yang diukur dan ditampilkan, bahasa visual
  perangkat keras bergaya MXM, tanpa akun, tanpa internet, dan proyek tidak
  pernah hilang.
- **MVP:** akhir M4, beat utuh sudah bisa dibuat di HP. M5–M9 menambah
  kedalaman sampai rilis 1.0.
- **Langkah pertama:** pemilik membuat repo, lalu membuka sesi baru dengan
  dokumen ini ([§18](#18-memulai-di-sesi-baru)).

---

## Daftar isi

1. [Visi](#1-visi)
2. [Prinsip produk](#2-prinsip-produk)
3. [Pengguna dan skenario](#3-pengguna-dan-skenario)
4. [Fitur lengkap](#4-fitur-lengkap)
5. [Desain: bahasa visual MXM-DAW](#5-desain-bahasa-visual-mxm-daw)
6. [Tata letak layar](#6-tata-letak-layar)
7. [Interaksi](#7-interaksi)
8. [Arsitektur](#8-arsitektur)
9. [Mesin audio](#9-mesin-audio)
10. [Model data dan berkas proyek](#10-model-data-dan-berkas-proyek)
11. [Anggaran performa](#11-anggaran-performa)
12. [Strategi tes](#12-strategi-tes)
13. [Kerangka kerja: proses pengembangan](#13-kerangka-kerja-proses-pengembangan)
14. [Tahapan M0–M9](#14-tahapan-m0m9)
15. [Legal, lisensi, privasi](#15-legal-lisensi-privasi)
16. [Risiko dan mitigasi](#16-risiko-dan-mitigasi)
17. [Keputusan yang menunggu pemilik](#17-keputusan-yang-menunggu-pemilik)
18. [Memulai di sesi baru](#18-memulai-di-sesi-baru)
19. [Glosarium](#19-glosarium)
20. [Lampiran: AGENTS.md awal](#20-lampiran-agentsmd-awal)

---

## 1. Visi

**Satu kalimat:** alat musik saku yang mengubah ide beat menjadi lagu dalam
hitungan menit: cepat disentuh, jujur bunyinya, dan bisa dipercaya.

| Biasanya | MXM-DAW |
|---|---|
| Timeline rumit ala desktop, dikecilkan ke HP | Alur *sampler-composer*: pad → pola → adegan → lagu |
| Menu bertumpuk | Satu permukaan utama; setiap tombol punya satu tugas, SHIFT untuk tugas kedua |
| Latensi "cukup" | Latensi diukur dan ditampilkan; jalur audio rendah-latensi sejak tahap pertama |
| Butuh akun atau internet | **Tanpa izin internet** di build rilis; semuanya di perangkat |
| Desain generik | Bahasa visual MXM (tinta, cream, oranye) yang terasa seperti perangkat keras |

**Bukan tujuan (setidaknya sampai 1.0):** editor audio multitrack ala desktop,
plugin pihak ketiga, kolaborasi daring, versi iOS, AI generatif.

---

## 2. Prinsip produk

1. **Bunyi dulu.** Tidak ada fitur yang boleh membuat audio tersendat. Thread
   audio didahulukan dari segalanya, termasuk animasi.
2. **Satu sentuhan, satu hasil.** Tekan pad → bunyi. REC + PLAY → merekam.
   Fitur lanjutan ada, tetapi tidak menghalangi jalur cepat.
3. **Fisik, tanpa berlebihan.** Kedalaman tombol, LED, dan layar segmen cukup
   untuk terasa seperti alat, tanpa tekstur kayu atau kulit tiruan.
4. **Jujur.** Latensi, beban CPU, dan buffer underrun ditampilkan, tidak
   disembunyikan. Bukti dilaporkan per jenis, apa adanya (§12).
5. **Milik pengguna.** Proyek dan sampel tidak pernah keluar dari perangkat,
   kecuali pengguna sendiri yang mengekspornya.
6. **Buatan sendiri.** Kit pabrik, ikon, dan angka segmen dibuat sendiri, sama
   seperti mxm-gameboy yang hanya mengirim homebrew buatan sendiri.
7. **Efisien.** Setiap tahap menghasilkan sesuatu yang bisa dimainkan di HP
   pemilik. Fitur premium menunggu fondasi yang sudah terbukti.

---

## 3. Pengguna dan skenario

**Pengguna utama:** pemilik (pembuat beat yang sedang belajar produksi musik),
lalu siapa pun yang ingin groovebox di saku.

| Skenario | Yang harus terasa |
|---|---|
| Ide beat di jalan (5 menit) | Buka → kit siap → tekan pad → rekam pola, tanpa membuka menu |
| Sampling suara sekitar | Tahan SAMPLE → rekam dari mikrofon → otomatis dipotong → langsung ada di pad |
| Menyusun lagu | Pola per grup → adegan → urutan lagu → ekspor WAV |
| Tampil langsung | Punch-in FX, mute grup, ganti adegan tepat di akhir bar |
| Mengolah di laptop | Ekspor stem per grup + MIDI, lalu buka di DAW desktop |

---

## 4. Fitur lengkap

Kolom **Tahap** merujuk ke §14. Fitur bertanda ★ adalah fitur premium sesudah
1.0 (ditulis "1.x"), tetapi arsitekturnya disiapkan sejak awal.

### 4.1 Pad dan suara

| Fitur | Detail | Tahap |
|---|---|---|
| 4 grup × 16 pad | Grup A–D, 64 suara per proyek | M2/M4 |
| Velocity | Dari posisi sentuh vertikal di pad, atau tetap (pilihan) | M2 |
| Mode main | One-shot, gate (bunyi selama ditekan), loop | M2 |
| Choke group | Mis. hi-hat terbuka dipotong hi-hat tertutup | M2 |
| Parameter suara | Level, pan, pitch (semitone + cent), start/end, ADSR, filter (LP/HP/BP + resonansi), drive | M2 |
| Mode KEYS | Satu suara dimainkan kromatik di 16 pad, dengan skala dan oktaf | M4 |
| Note repeat | Tahan pad + pilih rate (1/4 … 1/32, triplet) | M4 |
| ★ Synth | Oscillator wavetable/FM sederhana untuk bass dan lead | 1.x |
| ★ Layer | Dua sampel per pad (velocity layer) | 1.x |

### 4.2 Sampler

| Fitur | Detail | Tahap |
|---|---|---|
| Impor | WAV, AIFF, FLAC, MP3, OGG lewat Storage Access Framework, tanpa izin penyimpanan luas | M2 |
| Rekam | Mikrofon, input USB audio, atau *resample* keluaran master | M6 |
| Threshold | Mulai merekam saat sinyal melewati ambang | M6 |
| Kompensasi latensi | Kalibrasi round-trip (klik → rekam) supaya rekaman tepat di ketukan | M6 |
| Editor gelombang | Zoom, start/end/loop, snap ke zero-crossing, normalisasi, fade | M2/M6 |
| Chop | Bagi rata (2–16), deteksi transien, atau manual; hasilnya langsung ke pad | M6 |
| ★ Time-stretch | Menyesuaikan loop ke BPM tanpa mengubah nada | 1.x |

### 4.3 Sequencer

| Fitur | Detail | Tahap |
|---|---|---|
| Resolusi | 960 PPQN; posisi dihitung dari tick absolut dan dijadwalkan tepat ke sampel | M3 |
| Tempo | 40–240 BPM dengan satu desimal, tap tempo | M3 |
| Pola | 1–16 bar, 99 pola per grup | M3/M4 |
| Rekam langsung | Overdub, metronom, count-in 1–2 bar | M3 |
| Timing correct | Kuantisasi 0–100 %, tidak merusak: rekaman asli tetap disimpan, kekuatannya bisa diubah kapan saja | M3 |
| Swing | 50–75 %, global dan per grup | M3 |
| Step grid | 16 langkah per halaman, velocity, panjang not, geser mikro-timing | M3 |
| Salin/tempel/hapus | Pola, langkah, grup | M4 |
| Undo/redo | 100 langkah | M4 |
| ★ Parameter lock | Nilai parameter berbeda per langkah | 1.x |
| ★ Probabilitas & kondisi | Langkah berbunyi 50 %, tiap 2 putaran, dll. | 1.x |

### 4.4 Efek dan mixer

| Fitur | Detail | Tahap |
|---|---|---|
| Insert per grup | Filter, drive/saturasi, bitcrush | M5 |
| Send | Delay (sinkron tempo), reverb, chorus | M5 |
| Master | Kompresor + limiter, mencegah clipping | M5 |
| Punch-in FX | Stutter/repeat, tape stop, sapuan filter; dimainkan dari pad/fader | M5 |
| Mixer | 4 grup + master: level, pan, mute, solo, send, meter puncak/RMS | M5 |
| Sidechain | Kick menekan grup lain (ducking) | M5 |

### 4.5 Lagu dan ekspor

| Fitur | Detail | Tahap |
|---|---|---|
| Adegan (scene) | Satu pola per grup, 99 adegan | M7 |
| Mode lagu | Urutan adegan dengan jumlah ulang | M7 |
| Ekspor audio | WAV 16/24-bit atau 32-bit float; mix atau stem per grup | M7 |
| Ekspor MIDI | Pola dan lagu sebagai `.mid` | M7 |
| Proyek | Simpan otomatis; ekspor/impor `.mxmdaw` (zip berisi JSON + sampel) | M4/M7 |
| Bagikan | Lewat share sheet Android | M7 |

### 4.6 MIDI dan sinkronisasi

| Fitur | Detail | Tahap |
|---|---|---|
| USB MIDI masuk | Kontroler memainkan pad/keys | M8 |
| MIDI keluar & clock | MXM-DAW sebagai pengirim atau penerima clock | M8 |
| MIDI learn | Kenop/fader kontroler ke parameter mana pun | M8 |
| ★ Bluetooth MIDI | Setelah USB stabil | 1.x |

### 4.7 Kualitas hidup

Onboarding singkat; kit pabrik orisinal; tema terang dan gelap; bahasa
Indonesia + Inggris; aksesibilitas (TalkBack, target sentuh ≥ 48 dp, status
tidak hanya lewat warna); panel "kesehatan mesin" (latensi, CPU, underrun);
halaman lisensi.

---

## 5. Desain: bahasa visual MXM-DAW

### 5.1 Arah

Inspirasinya adalah **genre** sampler-composer genggam: permukaan seperti panel
perangkat keras, pad besar, layar segmen gelap dengan ikon menyala, kenop
berwarna. Identitasnya tetap **MXM**.

**Aturan keras:** jangan meniru merek, nama produk, logo, huruf Jepang, ikon
layar, atau tata letak persis produk mana pun, termasuk gambar referensi
K.O. II milik teenage engineering. Repo ini publik, jadi MXM-DAW harus berdiri
dengan bahasa visualnya sendiri. Contohnya, MXM-DAW memakai 16 pad 4×4 dengan
kolom grup di kiri, bukan tata letak 12 tombol produk referensi.

### 5.2 Token warna

Diturunkan dari palet MXM (tinta `#0E0E0E`, cream `#F5EBD7`, oranye `#FF8A3D`,
abu-abu tenang) dan cokelat LCD mxm-gameboy (`#7A3F17`).

| Token | Terang | Gelap | Dipakai untuk |
|---|---|---|---|
| `body` | `#ECE6DA` | `#161514` | Panel utama |
| `body.edge` | `#D9D2C4` | `#24221F` | Garis panel, celah antar-modul |
| `key.dark` | `#1A1918` | `#2A2826` | Pad angka, tombol fungsi |
| `key.light` | `#F5EBD7` | `#3A3733` | Tombol grup, tombol sekunder |
| `key.accent` | `#FF8A3D` | `#FF8A3D` | REC, kenop utama |
| `legend.onDark` | `#F5EBD7` | `#F5EBD7` | Tulisan di tombol gelap |
| `legend.onLight` | `#0E0E0E` | `#F5EBD7` | Tulisan di tombol terang |
| `legend.onAccent` | `#0E0E0E` | `#0E0E0E` | Tulisan di tombol oranye (cream di atas oranye terlalu pucat) |
| `display.bg` | `#0B0B0B` | `#050505` | Layar segmen |
| `display.on` | `#F5EBD7` | `#F5EBD7` | Segmen menyala |
| `display.accent` | `#FF8A3D` | `#FF8A3D` | Ikon aktif, angka penting |
| `display.off` | cream 7 % | cream 6 % | Segmen padam yang samar terlihat |
| `text.muted` | `#645F57` | `#8A857C` | Label kecil di panel |
| `group.A–D` | oranye · cream · abu-abu · cokelat | sama | LED dan garis atas pad (di atas tombol gelap), selalu bersama huruf A–D |
| `meter.clip` | oranye berkedip + teks "CLIP" | sama | Peringatan clipping |

Kontras teks memenuhi WCAG AA (≥ 4,5:1) dan diperiksa oleh tes (§12). Contoh:
`text.muted` di atas `body` = 5,1:1 (terang) dan 5,0:1 (gelap); tinta di atas
oranye = 8,2:1.

### 5.3 Tipografi

- **Inter** (OFL) untuk antarmuka; **JetBrains Mono** (OFL) untuk angka di
  panel.
- **Angka segmen di layar digambar sendiri** (7/14 segmen, `CustomPainter`):
  tanpa lisensi font, tajam di semua ukuran, dan segmen padam ikut tergambar
  samar.
- Skala: 11 / 13 / 15 / 20 / 28 / 44 (angka BPM besar).
- Label panel memakai istilah musik internasional (REC, PLAY, SHIFT);
  penjelasan dan menu dilokalkan.

### 5.4 Komponen

| Komponen | Spesifikasi |
|---|---|
| **Pad** | 52–72 dp mengikuti lebar layar (tidak pernah < 48 dp), sudut 6 dp; tekan = turun 2 dp + bayangan memendek (60 ms), lepas 140 ms; LED kecil di atas pad saat suaranya berbunyi |
| **Tombol fungsi** | Dua label: fungsi utama (atas) dan fungsi SHIFT (bawah, warna redup) |
| **Kenop** | Ø 56 dp; geser vertikal 200 dp = penuh; tahan SHIFT = halus (×0,1); ketuk ganda = nilai bawaan; cincin nilai di sekelilingnya |
| **Fader** | Vertikal, jalur 88 dp (Pocket) sampai 160 dp (Studio), tutup bertekstur; fungsi aktif tertulis di labelnya |
| **Layar segmen** | Angka besar (BAR.BEAT / BPM), baris ikon 16×16 piksel buatan sendiri, meter, nama suara |
| **LED** | 6 dp, menyala dengan pendar halus (80 ms), padam seketika |
| **Grid langkah** | Sel 32–44 dp, velocity sebagai tinggi isi, playhead tanpa easing supaya terasa mekanis |
| **Gelombang** | Dirender sekali per tingkat zoom ke `Picture`, lalu digeser tanpa menggambar ulang |
| **Meter, LED, playhead** | Masing-masing di `RepaintBoundary` sendiri, jadi 60 kali per detik hanya bagian kecil itu yang digambar ulang |

### 5.5 Gerak, haptik, dan suara antarmuka

- Animasi mengikuti refresh rate layar (90/120 Hz bila ada) dan menghormati
  pengaturan "kurangi gerakan".
- Haptik: ketukan ringan pada pad (bisa dimatikan), "klik" di titik henti
  kenop, denyut di ketukan metronom saat merekam (opsional).
- Antarmuka sendiri tidak berbunyi. Satu-satunya bunyi adalah musik pengguna.

---

## 6. Tata letak layar

### 6.1 Pocket (potret): permukaan utama untuk bermain

```
┌──────────────────────────────────────────┐
│ MXM-DAW               CPU 18%  LAT 12 ms │
├──────────────────────────────────────────┤
│ ┌──────────────────────────────────────┐ │
│ │ A:07  KICK-TAPE             > 012.3  │ │
│ │ BPM 096.0   SW 56%   FX ON           │ │
│ │ [REC][LOOP][KEYS]   L ####-  R ###-  │ │
│ └──────────────────────────────────────┘ │
│ [SOUND] [PATTERN] [SONG] [FX] [MIX]      │
│  (VOL)    ( X )    ( Y )    FADER [=]    │
├────┬─────────────────────────────┬───────┤
│ A  │  [ 1 ] [ 2 ] [ 3 ] [ 4 ]    │ SAMPLE│
│ B  │  [ 5 ] [ 6 ] [ 7 ] [ 8 ]    │ TIMING│
│ C  │  [ 9 ] [10 ] [11 ] [12 ]    │  KEYS │
│ D  │  [13 ] [14 ] [15 ] [16 ]    │ ERASE │
├────┴─────────────────────────────┴───────┤
│ [SHIFT] [UNDO]  [-] [+]  [o REC] [> PLAY]│
└──────────────────────────────────────────┘
```

Grup di kiri, 16 pad di tengah, fungsi di kanan, transport di bawah: susunan
umum groovebox, disusun ulang untuk ibu jari dan layar 6–7 inci. Di HP selebar
360 dp, pad berukuran ±53 dp; di HP 412 dp, ±66 dp.

### 6.2 Studio (lanskap/tablet): untuk menyunting

```
┌──────────────────────┬────────────────────────────────────────┐
│ LAYAR SEGMEN ringkas │ HALAMAN KONTEKS (ikut mode)            │
│ PAD 4x4 + grup A-D   │ - Sound  : gelombang, ADSR, filter     │
│ kenop X/Y, fader     │ - Pattern: grid 16 langkah x pad       │
│ transport            │ - Song   : adegan + urutan lagu        │
│                      │ - Mix    : 4 strip + master, meter     │
└──────────────────────┴────────────────────────────────────────┘
```

### 6.3 Halaman lain

| Halaman | Isi |
|---|---|
| Perpustakaan | Proyek, sampel (tag, cari, pratinjau), kit pabrik, impor/ekspor |
| Pengaturan | Audio (mode latensi, ukuran buffer, kalibrasi), MIDI, haptik, tema, bahasa |
| Kesehatan mesin | Latensi keluaran/masukan, beban callback, underrun, voice aktif, memori sampel |
| Tentang | Versi, lisensi semua dependensi, kredit kit pabrik |

---

## 7. Interaksi

| Aksi | Cara |
|---|---|
| Mainkan | Ketuk pad (multi-sentuh hingga 10 jari) |
| Pilih tanpa membunyikan | SHIFT + pad |
| Sunting suara | Tekan lama pad → halaman Sound |
| Rekam pola | REC → PLAY (count-in opsional) → mainkan pad |
| Hapus not saat merekam | Tahan ERASE + pad |
| Kuantisasi | TIMING → atur kekuatan dengan kenop X |
| Ganti grup | Ketuk A–D; tahan untuk mute |
| Punch-in FX | Mode FX → tahan pad efek; fader = intensitas |
| Kenop konteks | X/Y mengikuti halaman: Sound → pitch & filter; Mix → level & send |
| Undo / redo | UNDO / SHIFT + UNDO |

**Latensi sentuhan:** pad memakai `Listener` (event pointer mentah), bukan
`GestureDetector`, yang bisa menunda sentuhan sambil menunggu "arena gesture"
memutuskan jenis sentuhannya. Pad memanggil mesin **secara sinkron** lewat
FFI. Panggilan itu hanya memasukkan event ke antrean lock-free (hitungan
mikrodetik), tanpa melewati isolate lain.

---

## 8. Arsitektur

### 8.1 Gambaran

```
┌─ Flutter (Dart): tampilan saja ──────────────────────────────┐
│ design system · halaman · gesture · Riverpod                 │
└─────┬───────────────────────────────────▲────────────────────┘
      │ intent (FFI sync):                │ snapshot 60 Hz
      │ pad · parameter · transport       │ + tampilan berversi
      ▼                                   │
┌─ Rust: daw-ffi · daw-core · daw-io ─────┴────────────────────┐
│ Controller : Project · undo · serde · kompilasi → RtState    │
│ Audio      : sequencer · voices · DSP · FX · mixer           │
│              (tanpa alokasi, tanpa lock, tanpa I/O)          │
│ I/O        : dekode · resample · ekspor WAV/MIDI · .mxmdaw   │
│ Antrean    : UI → audio (pad, transport)                     │
│              controller → audio (RtState)                    │
│              audio → controller (RtEvent: not, meter)        │
└───────────────────────────────────────────────────▲──────────┘
                                                    │ render(buffer)
                                                    │ setiap burst
┌─ Native Android ──────────────────────────────────┴──────────┐
│ C++    : Oboe → AAudio, stream rendah-latensi                │
│ Kotlin : MIDI · izin mikrofon · SAF · share sheet            │
└──────────────────────────────────────────────────────────────┘
```

- **Dua jalur ke thread audio.** Pad dan transport datang langsung dari UI
  lewat antrean sendiri (jalur tercepat). Perubahan model lewat controller,
  yang mengompilasi `Project` menjadi `RtState` yang siap diputar.
- **Dart tidak menyalin seluruh proyek.** Setiap 60 Hz hanya snapshot kecil
  (playhead, meter, LED) yang dikirim. Data halaman, mis. not di pola yang
  sedang terbuka, diminta ulang hanya bila nomor versinya berubah.
- **Satu pustaka native per ABI.** Rust dibangun sebagai `staticlib`, lalu
  CMake menggabungkannya dengan adapter C++ Oboe kecil menjadi
  `libmxm_engine.so`. flutter_rust_bridge dan Kotlin memuat pustaka yang sama.

### 8.2 Keputusan inti (calon ADR di M0)

| # | Keputusan | Alasan | Alternatif yang ditolak |
|---|---|---|---|
| 1 | **Mesin audio di Rust**, bukan Dart | Dart memakai garbage collector dan tidak real-time; thread audio tidak boleh berhenti sesaat pun | Paket pemutar Flutter (tidak bisa menjadwalkan tepat ke sampel) |
| 2 | **Rust memegang model proyek** (Project, undo, simpan) | Satu sumber kebenaran; CLI bisa merender proyek yang sama untuk tes golden | Model di Dart + salinan di Rust (dua sumber, mudah tidak sinkron) |
| 3 | **Oboe (C++) sebagai adapter I/O**, digabung dengan Rust dalam satu `.so` | Oboe dari Google memuat bertahun-tahun penanganan bug perangkat Android | AAudio langsung dari Rust (crate `ndk`/`cpal`): lebih sedikit penanganan kasus perangkat; bisa ditinjau lagi lewat ADR baru |
| 4 | **flutter_rust_bridge 2** | Binding tergenerasi; fungsi `sync` untuk pad, `Stream` untuk snapshot | `dart:ffi` manual (lebih banyak kode lem) |
| 5 | **Riverpod** + state tak berubah | Rebuild terkendali, mudah dites | Bloc (lebih banyak boilerplate untuk kasus ini) |
| 6 | **Tanpa izin `INTERNET`** di build rilis, diperiksa CI | Privasi dijamin sistem operasi, bukan sekadar janji | Analitik/crash reporting (ditolak) |
| 7 | **Proyek = JSON berversi + sampel WAV** | Terbuka, bisa dibaca manusia, mudah dimigrasi | Database biner |
| 8 | **minSdk 27** (Android 8.1) | Oboe memakai AAudio mulai 8.1 (di 8.0 jatuh ke OpenSL ES); perangkat yang lebih tua sudah sangat jarang | minSdk lebih rendah (jalur OpenSL ES, latensi lebih buruk) |

ADR berikutnya ditulis saat keputusannya tiba, mis. resolusi sequencer (M3),
format rekaman (M6), dan jalur MIDI native (M8).

### 8.3 Struktur repo

```
mxm-daw/
├── AGENTS.md                  aturan untuk agen (§20)
├── README.md                  status tahap, cara memasang APK
├── docs/
│   ├── rencana.md             dokumen ini
│   ├── adr/                   0001-…: keputusan
│   ├── jurnal/                m0.md … m9.md
│   └── desain/                token, komponen, gambar bukti
├── engine/                    workspace Rust
│   └── crates/
│       ├── daw-core           model, sequencer, voices, DSP, FX (tanpa I/O)
│       ├── daw-io             dekode, resampling, ekspor WAV/MIDI, .mxmdaw
│       ├── daw-ffi            API untuk Flutter (flutter_rust_bridge)
│       └── daw-render         CLI: proyek/kit → WAV, golden, benchmark
├── app/                       Flutter
│   ├── lib/app/               routing, tema, lokalisasi
│   ├── lib/design_system/     token, Pad, Knob, Fader, SegmentDisplay, …
│   ├── lib/features/          pads · sound · pattern · song · fx · mix ·
│   │                          library · settings · health
│   ├── lib/engine/            binding hasil codegen + stream snapshot
│   └── android/app/src/main/
│       ├── cpp/               adapter Oboe + CMakeLists → libmxm_engine.so
│       └── kotlin/            MIDI, izin, SAF, share
├── kits/                      resep kit pabrik (dirender saat build)
└── tools/                     build-android.sh/.ps1, golden, ikon, cek manifest
```

### 8.4 Aturan thread

| Thread | Boleh | Tidak boleh |
|---|---|---|
| **Audio** (callback Oboe) | Membaca `RtState`, mengisi buffer, mengirim `RtEvent` | Alokasi memori, lock, log, I/O, syscall |
| **Controller** (Rust) | Mengubah `Project`, undo, kompilasi `RtState`, serde | Menyentuh buffer audio langsung |
| **I/O** (Rust, pool kecil) | Dekode/resample berkas, menulis ekspor | Mengubah `Project` tanpa lewat controller |
| **UI** (Dart) | Menggambar, mengirim intent, membaca snapshot | Menunggu mesin secara blocking (panggilan pad hanya mikrodetik) |

Sampel besar dibagikan ke thread audio sebagai data hanya-baca. Memorinya
dibebaskan di thread lain (*deferred reclamation*), supaya thread audio tidak
pernah membebaskan memori sendiri.

### 8.5 Pustaka dan alat (kandidat)

DSP (filter, envelope, FX) **ditulis sendiri**. Itu inti belajar proyek ini,
sama seperti CPU/PPU/APU di mxm-gameboy. Pustaka dipakai untuk hal yang bukan
inti:

| Lapisan | Pilihan | Lisensi | Guna |
|---|---|---|---|
| UI | Flutter (stable) | BSD-3-Clause | Tampilan, gesture |
| State | flutter_riverpod | MIT | State UI |
| Jembatan | flutter_rust_bridge 2 | MIT | Binding Dart ↔ Rust |
| Audio I/O | Oboe | Apache-2.0 | Stream rendah-latensi |
| Antrean RT | rtrb | MIT OR Apache-2.0 | Antrean SPSC lock-free |
| Pembebasan tertunda | basedrop | MIT OR Apache-2.0 | Membebaskan sampel di luar thread audio |
| Dekode | symphonia | MPL-2.0 | WAV/AIFF/FLAC/MP3/OGG |
| Resampling | rubato | MIT | Mengubah sample rate saat impor |
| WAV | hound | Apache-2.0 | Menulis ekspor |
| Berkas MIDI | midly | Unlicense | Membaca/menulis `.mid` |
| Proyek | serde, serde_json; zip | MIT OR Apache-2.0; MIT | JSON + `.mxmdaw` |
| Tes real-time | assert_no_alloc | BSD-2-Clause | Alokasi di thread audio = tes gagal |
| Tes properti | proptest | MIT OR Apache-2.0 | Berkas rusak tidak boleh bikin crash |
| Uji mutasi | cargo-mutants | MIT | Menemukan tes yang lemah |
| Lisensi | cargo-deny, cargo-about | MIT OR Apache-2.0 | Cek lisensi; daftar untuk halaman Tentang |
| Build | cargo-ndk | MIT OR Apache-2.0 | Rust untuk ABI Android |

Versi dipin di `Cargo.lock` dan `pubspec.lock`; lisensi diperiksa ulang oleh
`cargo-deny` di setiap CI.

---

## 9. Mesin audio

| Bagian | Spesifikasi |
|---|---|
| Format internal | 48 kHz (rate asli hampir semua HP Android modern), float 32-bit, stereo; Oboe mengonversi bila perangkat berbeda |
| Ukuran blok | Mengikuti *burst* perangkat (umumnya 96–256 frame); buffer mulai dari 2 × burst dan dinaikkan otomatis bila underrun (`LatencyTuner` Oboe) |
| Mode stream | `PerformanceMode::LowLatency`, `SharingMode::Exclusive` (jatuh ke Shared bila ditolak) |
| Voices | 32 polifoni; voice stealing (paling lama / paling pelan); choke group |
| Interpolasi | Hermite 4-titik untuk pitch; linear sebagai mode hemat |
| Envelope | ADSR eksponensial, tanpa klik (rampa minimum 1 ms) |
| Filter | State-variable (TPT/ZDF), LP/HP/BP, aman untuk modulasi cepat |
| Sequencer | 960 PPQN. Posisi frame setiap event dihitung dari tick absolut, bukan dijumlahkan berulang, supaya tidak melenceng setelah ribuan bar. Event jatuh di offset frame yang tepat di dalam blok |
| Swing | Not 1/16 kedua di setiap pasangan (langkah 2, 4, 6, … dihitung dari 1) digeser `(swing − 50 %) × 2 × durasi_langkah` |
| FX | Delay sinkron tempo, reverb FDN 8 jalur, chorus, kompresor RMS, limiter dengan look-ahead pendek (≤ 1,5 ms, ikut dihitung dalam latensi) |
| Memori sampel | Disimpan di RAM sebagai int16, dikonversi saat diputar; kumpulan 128 MB ≈ 11 menit stereo 48 kHz |
| Bluetooth | Terdeteksi → peringatan "latensi tinggi"; kompensasi rekaman menyesuaikan |
| Kesehatan | Hitungan underrun dari Oboe, beban callback (waktu render ÷ periode buffer), latensi terlapor |

**Rendering offline.** Mesin yang sama juga merender tanpa perangkat audio:
ekspor di aplikasi (di thread I/O), serta tes golden dan benchmark lewat CLI
`daw-render` di CI.

---

## 10. Model data dan berkas proyek

```json
{
  "schema": 1,
  "name": "Beat Sore",
  "bpm": 96.0,
  "swing": 0.56,
  "time_signature": [4, 4],
  "groups": [
    {
      "id": "A",
      "level": 0.8, "pan": 0.0, "mute": false,
      "sends": { "delay": 0.2, "reverb": 0.15 },
      "inserts": [ { "kind": "filter", "mode": "lp", "cutoff": 0.7, "resonance": 0.2 } ],
      "pads": [
        {
          "index": 0,
          "sample": "samples/kick-tape.wav",
          "start": 0, "end": 21840,
          "pitch": 0.0, "level": 1.0, "pan": 0.0,
          "envelope": { "attack": 0.001, "decay": 0.0, "sustain": 1.0, "release": 0.02 },
          "filter": { "mode": "lp", "cutoff": 1.0, "resonance": 0.0 },
          "play_mode": "one_shot", "choke": null
        }
      ],
      "patterns": [
        {
          "index": 1, "bars": 2,
          "quantize": { "grid": 240, "strength": 1.0 },
          "events": [ { "tick": 0, "pad": 0, "velocity": 1.0, "length": 240 } ]
        }
      ]
    }
  ],
  "scenes": [ { "index": 1, "patterns": { "A": 1, "B": 1, "C": 0, "D": 0 } } ],
  "song": [ { "scene": 1, "repeat": 4 } ],
  "master": { "delay_time": "1/8.", "reverb_size": 0.6, "compressor": 0.3, "limiter": true }
}
```

- **Satuan:** `tick` 960 per ketukan (jadi 240 = satu not 1/16); `start`/`end`
  dalam frame 48 kHz; waktu envelope dalam detik; level, cutoff, dan send 0–1.
- **Kuantisasi tidak merusak:** `tick` disimpan seperti saat direkam, sedangkan
  `grid` dan `strength` disimpan di pola. Posisi akhir dihitung saat
  kompilasi.
- `"C": 0` berarti grup C diam di adegan itu.
- `schema` naik setiap kali format berubah. Migrasi selalu **menambah**, tidak
  pernah menghapus data pengguna. Proyek dari versi yang lebih baru ditolak
  dengan pesan jelas, bukan dibaca salah (pelajaran dari save state
  mxm-gameboy).
- Simpan otomatis setiap ada perubahan (ditunda 500 ms): tulis ke berkas
  sementara lalu ganti nama secara atomik, supaya proyek tidak rusak bila
  aplikasi dimatikan paksa.
- `.mxmdaw` adalah zip berisi `project.json` + `samples/`.

---

## 11. Anggaran performa

Diukur di **HP pemilik** (perangkat acuan) dan dicatat di jurnal setiap tahap.

| Ukuran | Target | Cara ukur |
|---|---|---|
| Latensi keluaran (dilaporkan Oboe) | ≤ 20 ms di perangkat dengan jalur rendah-latensi; dicatat apa adanya di perangkat lain | Panel kesehatan mesin |
| Sentuh → bunyi | ≤ 50 ms (cita-cita ≤ 35 ms) | Rekam bunyi ketukan jari dan bunyi pad dengan mikrofon HP kedua, lalu ukur jaraknya di gelombang |
| Beban callback, 32 voice + semua FX | ≤ 50 % periode buffer | Panel kesehatan + benchmark `daw-render` |
| Underrun | 0 dalam 10 menit bermain | Penghitung Oboe |
| Frame UI | Build dan raster masing-masing ≤ 8 ms | Flutter DevTools, mode `--profile` |
| Mulai dingin | ≤ 1,5 s sampai pad bisa dimainkan | Log waktu |
| Ukuran APK arm64 | ≤ 25 MB | CI (gagal bila lewat) |
| Memori | ≤ 350 MB dengan kumpulan sampel penuh (128 MB) | `dumpsys meminfo` |
| Baterai | Stream berhenti saat aplikasi di latar dan tidak memutar | Tes manual 30 menit |

---

## 12. Strategi tes

| Lapisan | Apa | Kapan |
|---|---|---|
| Unit Rust | DSP, sequencer, model, undo, serde, migrasi skema | Setiap PR |
| **Timing tepat sampel** | Not di tick X mulai tepat di frame Y; tidak melenceng setelah 10.000 bar | Setiap PR |
| **Golden render** | Proyek contoh → WAV, dibandingkan dengan toleransi (selisih ≤ −100 dBFS) | Setiap PR |
| Tanpa alokasi | Render dijalankan di dalam `assert_no_alloc` | Setiap PR |
| Berkas rusak | Proyek/WAV terpotong atau acak ditolak dengan pesan, tidak pernah crash (`proptest`) | Setiap PR |
| Uji mutasi | `cargo-mutants` pada daw-core; mutan yang lolos = tes lemah | Per tahap (lama) |
| Widget Flutter | Komponen design system dan halaman | Setiap PR |
| Golden UI | Tangkapan komponen dalam tema terang dan gelap; hanya di CI Linux karena rendering font berbeda antar-OS | Setiap PR |
| Kontras token | Setiap pasangan teks/latar ≥ 4,5:1 (WCAG AA), dihitung di tes | Setiap PR |
| Manifest rilis | APK rilis tanpa `INTERNET`; izin lain persis sesuai daftar yang disetujui | Setiap PR |
| Integrasi | APK di emulator: buka, mainkan pad, rekam pola (audio emulator terbatas) | Merge ke `main` |
| **Perangkat nyata** | Latensi, underrun, haptik, MIDI, mikrofon | Setiap tahap, oleh pemilik |

**Kenapa golden memakai toleransi, bukan hash persis seperti di mxm-gameboy?**
Di Game Boy semuanya bilangan bulat, jadi hasilnya identik di mesin mana pun.
Di sini sinyalnya float, dan fungsi seperti `sin` atau `exp` bisa berbeda
sangat tipis antar-CPU. Selisih di bawah −100 dBFS tidak terdengar, sedangkan
bug sungguhan hampir selalu jauh lebih besar dari itu.

**Bukti dipisah per jenis** (sama seperti MXM): kode ditulis · tes lulus · CI
hijau · emulator · **HP pemilik**. Jenis bukti yang berbeda tidak pernah
dicampur.

---

## 13. Kerangka kerja: proses pengembangan

### 13.1 Alur satu langkah

1. **Rencana singkat** di percakapan → pemilik setuju.
2. **ADR** bila ada keputusan arsitektur. ADR yang sudah diterima tidak
   diedit; keputusan baru ditulis sebagai ADR baru.
3. **Tes dulu** bila memungkinkan, dan pastikan tesnya bisa gagal.
4. Kode → pemeriksaan lokal → **satu branch per langkah** dari `main`.
5. PR berisi: kenapa, apa yang berubah, **tabel bukti**, dan latihan untuk
   pemilik.
6. CI hijau → pemilik merge → APK pratinjau terbit otomatis.
7. **Bukti di HP pemilik** → jurnal tahap → tahap ditutup.

### 13.2 Definition of Done (per tahap)

- [ ] Fitur tahap bisa dimainkan di **HP pemilik** (tangkapan layar atau
      rekaman).
- [ ] Anggaran performa §11 yang relevan diukur dan dicatat.
- [ ] Tes unit, golden, dan widget ada; uji mutasi untuk bagian inti.
- [ ] Tanpa peringatan lint (`clippy -D warnings`, `flutter analyze`).
- [ ] ADR dan jurnal ditulis; README diperbarui.

### 13.3 CI (GitHub Actions)

| Job | Isi |
|---|---|
| `engine` | rustfmt, clippy pedantic, test, golden render, tanpa-alokasi, cargo-deny |
| `app` | `dart format`, `flutter analyze`, `flutter test` (widget + golden), kode hasil codegen tidak basi |
| `android` | cargo-ndk (arm64-v8a, x86_64) → CMake → APK; cek ukuran dan manifest; unggah artefak |
| `integration` | Emulator x86_64: alur dasar |
| `preview` | Setiap merge ke `main`: pre-release `preview-<build>` berisi APK arm64 + SHA-256 |
| `release` | Hanya dari tag; ditandatangani kunci rilis milik pemilik |

### 13.4 Versi, APK pratinjau, dan rilis

- SemVer (`0.x` sampai 1.0) + nomor build yang naik setiap build.
- **Pemilik tidak perlu memasang toolchain.** Buka halaman Releases di HP →
  unduh APK pratinjau terbaru → pasang.
- APK pratinjau memakai application ID terpisah (akhiran `.preview`) dan
  **kunci tanda tangan pratinjau yang tetap**. Ini penting: Android menolak
  memasang APK di atas versi lama bila tanda tangannya berbeda, dan menghapus
  aplikasi berarti menghapus proyek. Kunci dibuat pemilik dan disimpan sebagai
  GitHub secret; agen tidak pernah membuat, meminta, atau melihatnya.
- Rilis di GitHub Releases dengan catatan perubahan dan hash SHA-256.

---

## 14. Tahapan M0–M9

Setiap tahap berakhir dengan **sesuatu yang bisa dimainkan di HP pemilik**.

| Tahap | Isi | Bukti selesai |
|---|---|---|
| **M0** Fondasi | Repo, visi, ADR 1–8, token desain, CI, cek manifest, kunci pratinjau, aplikasi kosong bertema MXM-DAW yang memanggil Rust | APK pratinjau dari Releases terpasang di HP pemilik dan menampilkan versi mesin dari Rust |
| **M1** Mesin audio | Oboe + render Rust, satu pad memainkan kick sintetis, panel kesehatan mesin | Latensi dan underrun terukur di HP; 10 menit tanpa underrun |
| **M2** Pad & sampler dasar | Grup A 16 pad, kit pabrik, impor WAV, editor start/end, pitch, ADSR, filter, choke, velocity | Kit dimainkan di HP; sampel milik pemilik berhasil diimpor |
| **M3** Sequencer | Pola 1–16 bar, rekam langsung, metronom, timing correct, swing, grid langkah | Beat 4 bar direkam di HP; tes timing dan golden lulus |
| **M4** Grup, keys, proyek | Grup A–D, mode KEYS, note repeat, salin/tempel, undo/redo, simpan otomatis | Proyek tetap utuh setelah aplikasi ditutup paksa lalu dibuka lagi |
| **M5** FX & mixer | Insert, send, master, punch-in FX, mixer, sidechain | Beban CPU ≤ 50 % dengan semua FX menyala di HP |
| **M6** Sampling | Rekam mic/USB, threshold, kalibrasi latensi, chop, resample | Suara sekitar direkam → di-chop → dimainkan |
| **M7** Lagu & ekspor | Adegan, mode lagu, ekspor WAV/stem/MIDI, `.mxmdaw`, share | WAV hasil ekspor terbuka di aplikasi lain |
| **M8** MIDI | USB MIDI masuk/keluar, clock, MIDI learn | Kontroler USB memainkan pad (bila pemilik punya) |
| **M9** Premium & 1.0 | Mode Studio lanskap/tablet, aksesibilitas, lokalisasi, onboarding, optimasi, halaman lisensi | Rilis 1.0 di GitHub Releases, diuji di HP pemilik |

**MVP (sudah bisa membuat beat): akhir M4.** Tahap sesudahnya menambah
kedalaman, bukan dasar.

---

## 15. Legal, lisensi, privasi

| Hal | Keputusan |
|---|---|
| Lisensi kode | MIT OR Apache-2.0, sama seperti mxm-gameboy |
| Font | Inter dan JetBrains Mono (OFL); angka segmen digambar sendiri |
| Ikon | Dibuat dari kode/piksel sendiri |
| Kit pabrik | **Dibuat sendiri**: disintesis `daw-render` saat build (tanpa berkas biner di repo), atau direkam pemilik; semuanya CC0 |
| Dependensi | Hanya lisensi permisif (MIT/Apache/BSD/Zlib/Unlicense) atau MPL-2.0 tanpa modifikasi. Diperiksa `cargo-deny` (Rust) dan skrip kecil (paket Dart); semua lisensi tampil di halaman Tentang |
| Dihindari | Pustaka GPL (mis. Ableton Link, Rubber Band), kecuali ada ADR khusus |
| Merek | Tidak memakai nama, logo, tata letak persis, atau ikon produk lain |
| Izin Android | `RECORD_AUDIO` hanya diminta saat pertama merekam; tanpa `INTERNET` di build rilis (build debug Flutter memerlukannya untuk hot reload); berkas lewat SAF |
| Data | Proyek dan sampel tidak pernah diunggah; tanpa analitik; tanpa akun |
| Rahasia | Keystore dan kata sandinya tidak pernah ada di repo, log, artefak, atau percakapan |

---

## 16. Risiko dan mitigasi

| Risiko | Dampak | Mitigasi |
|---|---|---|
| Latensi tinggi di HP tertentu | Tidak enak dimainkan | Oboe low-latency + exclusive; diukur sejak M1; angkanya ditampilkan jujur |
| Jank UI mengganggu audio | Bunyi tersendat | Audio di thread native sendiri; UI tidak pernah memblokir mesin |
| Build lintas bahasa (Dart + Rust + C++ + Kotlin) | Lambat berkembang | Satu skrip build, CI sejak M0, satu `.so` per ABI |
| Fragmentasi perangkat | Bug di HP lain | Oboe; uji di emulator dan HP pemilik; panel kesehatan mesin |
| Ruang lingkup melebar | Tidak pernah selesai | MVP di M4; fitur ★ menunggu sesudah 1.0 |
| Lisensi dependensi | Masalah di repo publik | `cargo-deny` + skrip lisensi Dart di CI (gagal bila ada GPL tanpa ADR) |
| Proyek hilang | Pengguna kecewa | Simpan atomik, skema berversi, migrasi tambah-saja, ekspor `.mxmdaw` |
| Kunci APK pratinjau berganti | Harus uninstall, proyek ikut hilang | Kunci pratinjau tetap milik pemilik sejak M0; ekspor `.mxmdaw` sebelum uninstall |
| Baterai dan panas | Sesi bermain pendek | Stream berhenti saat diam; batas voice; mode hemat |

---

## 17. Keputusan yang menunggu pemilik

Dijawab di awal sesi baru (M0):

1. **Nama dan application ID.** Tetap "MXM-DAW"? Saran ID:
   `io.github.masalikovic.mxmdaw` (pratinjau: `….mxmdaw.preview`), karena
   berbasis domain GitHub milik pemilik sendiri.
2. **Perangkat acuan.** Merek, model, dan versi Android HP untuk semua
   pengukuran.
3. **Orientasi.** Pocket (potret) dulu, Studio (lanskap) di M9? Aplikasi MXM
   utama mengutamakan lanskap, tetapi alat musik genggam lebih nyaman
   dimainkan potret dengan ibu jari, seperti groovebox fisik.
4. **Putar di latar** (layar mati). Butuh layanan latar depan + izin tambahan.
   Perlu atau tidak?
5. **Kontroler MIDI USB.** Punya atau tidak? Ini menentukan bukti M8.
6. **Kit pabrik.** Disintesis seluruhnya, atau sebagian direkam pemilik?
7. **Kunci tanda tangan.** Pemilik membuat kunci pratinjau dan rilis dengan
   `keytool`, lalu menyimpannya sebagai GitHub secret (disarankan), atau
   menandatangani rilis secara lokal.
8. **ABI.** arm64-v8a saja (disarankan, mencakup hampir semua HP modern), atau
   juga armeabi-v7a untuk HP lama.

---

## 18. Memulai di sesi baru

**Disiapkan pemilik:**

1. Buat repo **publik** `mxm-daw` di GitHub (boleh kosong).
2. Pastikan Claude GitHub App punya akses ke repo itu:
   <https://github.com/apps/claude/installations/select_target>.
3. Buka sesi Claude Code baru dengan repo `mxm-daw`, lampirkan berkas ini, lalu
   kirim pesan pertama, misalnya:

   ```
   Ini rencana MXM-DAW (terlampir). Simpan sebagai docs/rencana.md, lalu mulai M0.
   Jawaban §17: 1) … 2) … 3) … 4) … 5) … 6) … 7) … 8) …
   ```

**Tentang lingkungan sesi cloud.** Rust bisa dites langsung di sesi. Flutter,
Android SDK, dan NDK berukuran beberapa GB, jadi APK paling andal dibangun oleh
CI GitHub Actions. Bila ingin membangun di sesi juga, atur lewat menu
lingkungan cloud di bilah judul sesi → **Edit**:

- **Setup script:** memasang Flutter dan Android SDK/NDK.
- **Network access:** izinkan domain unduhannya, mis. `storage.googleapis.com`,
  `pub.dev`, `dl.google.com`, `maven.google.com`, `services.gradle.org`,
  `plugins.gradle.org`, `repo.maven.apache.org` (panduan:
  <https://code.claude.com/docs/en/cloud-environments#network-access>).

Bila ada domain yang terlewat, sesi akan menyebutkan nama domain yang ditolak.

---

## 19. Glosarium

| Istilah | Arti singkat |
|---|---|
| Latensi | Jeda antara aksi (menekan pad) dan bunyi yang terdengar |
| Buffer / burst | Potongan audio yang diisi mesin sekali jalan. Makin kecil makin cepat, tetapi makin rawan putus |
| Callback audio | Fungsi yang dipanggil sistem setiap kali buffer perlu diisi |
| Underrun | Mesin terlambat mengisi buffer, terdengar sebagai klik atau putus |
| PPQN | *Pulses per quarter note*: resolusi waktu sequencer per ketukan |
| Timing correct / kuantisasi | Menggeser not ke grid terdekat, sebagian atau penuh |
| Swing | Menunda not off-beat supaya ritme terasa mengayun |
| Choke group | Kelompok suara yang saling memotong |
| ADSR | Attack, decay, sustain, release: bentuk naik-turunnya volume suara |
| Insert / send | Efek khusus satu kanal / efek bersama yang menerima kiriman dari beberapa kanal |
| Sidechain (ducking) | Satu suara mengecilkan suara lain, mis. kick menekan bass |
| Resample | Merekam keluaran MXM-DAW sendiri menjadi sampel baru (juga berarti mengubah sample rate) |
| dBFS | Desibel relatif terhadap batas digital penuh; 0 dBFS = paling keras |
| SPSC lock-free | Antrean satu penulis dan satu pembaca tanpa kunci, aman untuk thread audio |
| FFI | Cara kode Dart memanggil fungsi Rust secara langsung |

---

## 20. Lampiran: AGENTS.md awal

```markdown
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
```

---

*Dibuat dengan semangat MXM: belajar sambil membangun, satu langkah yang
dipahami pada satu waktu. Kali ini dengan suara.*
