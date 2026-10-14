# 0011. Token warna, warna grup C/D, dan tes kontras

- Status: Diterima
- Tanggal: 2026-10-14

## Konteks

Rencana §5.2 mendefinisikan token warna untuk tema terang dan gelap, dan
menuntut setiap pasangan teks/latar memenuhi WCAG AA (≥ 4,5:1) yang
diperiksa tes. Sebelum token ditulis ke kode, semua pasangan dihitung dulu.
Hasilnya:

- Semua pasangan **teks** lolos, dan angka di rencana terbukti benar
  (tinta di atas oranye 8,23:1; `text.muted` di atas `body` 5,10:1 terang dan
  4,97:1 gelap).
- Warna **grup D** (cokelat `#7A3F17`) dipakai sebagai LED dan garis atas pad
  di atas `key.dark`. Kontrasnya hanya 2,13:1 (terang) dan 1,78:1 (gelap),
  di bawah batas WCAG 3:1 untuk elemen grafis (kriteria 1.4.11). LED yang
  tidak terlihat tidak ada gunanya.
- Warna **grup C** ditulis "abu-abu" tanpa kode warna.

## Pilihan yang dipertimbangkan

Grup D:
1. **Cokelat terang `#B0612F`**: 3,84:1 dan 3,21:1. Lebih dekat ke oranye
   grup A, tetapi huruf A–D selalu tampil di sebelahnya.
2. Tetap `#7A3F17`, dikecualikan dari tes: warna merek utuh, LED nyaris
   tidak terlihat.
3. Cokelat lebih terang `#C07440`: 4,86:1 dan 4,07:1, paling mirip oranye.

Grup C:
1. **`#8A857C`**, memakai ulang `text.muted` versi gelap (tanpa warna baru):
   4,79:1 dan 4,00:1.
2. `#A39E95`: lebih terang, tetapi lebih mirip cream grup B.

## Keputusan

Pemilik memilih (2026-10-14): **grup D `#B0612F`** dan **grup C `#8A857C`**.
Cokelat `#7A3F17` tetap menjadi aksen merek MXM untuk hal yang tidak
membutuhkan kontras (mis. ilustrasi), tetapi bukan warna grup.

Cara token dipakai di kode:

- Semua warna ada di `app/lib/design_system/tokens.dart` (`MxmTokens`,
  sebuah `ThemeExtension`). Widget mengambilnya lewat
  `MxmTokens.of(context)` dan tidak menulis kode warna sendiri.
- `app/test/design_system/tokens_test.dart` memeriksa, untuk kedua tema:
  setiap pasangan teks/latar ≥ 4,5:1, setiap warna grup di atas `key.dark`
  ≥ 3:1, segmen padam samar tetapi terlihat, dan semua nilai sama dengan
  rencana §5.2 + ADR ini.
- Pasangan teks/latar **baru** wajib ditambahkan ke daftar di tes itu.

## Kontras terukur

| Pasangan | Terang | Gelap |
|---|---|---|
| `legend.onDark` / `key.dark` | 14,84 | 12,41 |
| `legend.onLight` / `key.light` | 16,31 | 10,00 |
| `legend.onAccent` / `key.accent` | 8,23 | 8,23 |
| `legend.onLight` / `body` | 15,53 | 15,41 |
| `text.muted` / `body` | 5,10 | 4,97 |
| `display.on` / `display.bg` | 16,63 | 17,22 |
| `display.accent` / `display.bg` | 8,39 | 8,69 |
| Grup A oranye / `key.dark` | 7,49 | 6,26 |
| Grup B cream / `key.dark` | 14,84 | 12,41 |
| Grup C `#8A857C` / `key.dark` | 4,79 | 4,00 |
| Grup D `#B0612F` / `key.dark` | 3,84 | 3,21 |

## Konsekuensi

- (+) Aturan aksesibilitas rencana §4.7 dan §5.2 dijaga tes, bukan ingatan.
- (+) Mengganti satu warna cukup di satu berkas, dan tes langsung memberi
  tahu bila kontrasnya rusak.
- (−) Grup A (oranye) dan grup D (cokelat terang) berhue mirip. Perbedaannya
  ditanggung huruf A–D, sesuai aturan "status tidak hanya lewat warna".
- (−) Angka di tabel ini dihitung dengan rumus WCAG 2.x. Bila suatu saat
  pindah ke metode lain (mis. APCA), perlu ADR baru.
