# Panduan: membuat kunci pratinjau dan menyimpannya di GitHub

Panduan ini untuk **pemilik** repo. Hasilnya: setiap merge ke `main`
menerbitkan APK pratinjau yang ditandatangani kunci milik Anda, sehingga APK
baru selalu bisa dipasang di atas yang lama tanpa kehilangan proyek
(ADR-0012).

> **Aturan:** keystore (`.jks`) dan kata sandinya **tidak pernah** dikirim ke
> agen, ditulis di percakapan, di-commit, atau ditempel di issue/PR. Yang boleh
> dibagikan hanya **sidik jari SHA-256**, karena sidik jari itu publik (siapa
> pun bisa membacanya dari APK).

Anda butuh `keytool` (bagian dari Java). Pilih salah satu jalur di bawah.

## Langkah 1. Pasang keytool

**Termux di HP (Redmi 13C):**

```sh
pkg install openjdk-21 gh
```

**Windows 10:** pasang JDK 17 atau lebih baru (mis. Eclipse Temurin). Sesudah
itu `keytool` tersedia di PowerShell. Bila tidak ditemukan, buka ulang
PowerShell. Untuk langkah 3, pasang juga GitHub CLI (`gh`).

## Langkah 2. Buat kunci

Jalankan di folder **di luar** repo (mis. folder Dokumen):

```sh
keytool -genkeypair -v -keystore mxmdaw-pratinjau.jks -storetype PKCS12 \
  -keyalg RSA -keysize 4096 -validity 10000 -alias pratinjau
```

Di Windows PowerShell, tulis dalam satu baris tanpa `\`.

- `keytool` menanyakan kata sandi. Buat kata sandi yang panjang dan **simpan di
  pengelola kata sandi**. Format PKCS12 memakai satu kata sandi untuk keystore
  dan kunci.
- Pertanyaan nama/organisasi boleh diisi seadanya (mis. nama Anda). Isinya
  tercantum di sertifikat APK dan bisa dibaca orang lain.
- `-validity 10000` berarti kunci berlaku sekitar 27 tahun.

## Langkah 3. Simpan sebagai GitHub secret

Dibutuhkan tiga secret:

| Nama secret | Isi |
|---|---|
| `MXMDAW_PRATINJAU_KEYSTORE_B64` | Isi `mxmdaw-pratinjau.jks` dalam bentuk base64 |
| `MXMDAW_PRATINJAU_PASSWORD` | Kata sandi dari langkah 2 |
| `MXMDAW_PRATINJAU_ALIAS` | `pratinjau` |

**Cara A: GitHub CLI (disarankan, tanpa menyalin teks panjang):**

> **Jalankan satu per satu.** Tunggu `gh auth login` selesai (`✓ Logged in
> as …`) sebelum perintah berikutnya, dan tunggu `✓ Set Actions secret …`
> setelah setiap `gh secret set`. Bila semuanya ditempel sekaligus, perintah
> yang berjalan sebelum login selesai tidak menyimpan apa pun (kejadian nyata
> di M0, lihat docs/jurnal/m0.md).

```sh
gh auth login
```
```sh
gh secret set MXMDAW_PRATINJAU_KEYSTORE_B64 --repo masalikovic/MXM-DAW < <(base64 -w 0 mxmdaw-pratinjau.jks)
```
```sh
gh secret set MXMDAW_PRATINJAU_PASSWORD --repo masalikovic/MXM-DAW
```
```sh
gh secret set MXMDAW_PRATINJAU_ALIAS --repo masalikovic/MXM-DAW --body pratinjau
```
```sh
gh secret list --repo masalikovic/MXM-DAW   # harus menampilkan 3 nama
```

Perintah kedua menanyakan kata sandi secara tersembunyi, jadi kata sandi tidak
tersimpan di riwayat shell.

Di Windows PowerShell, ganti perintah pertama dengan:

```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes("mxmdaw-pratinjau.jks")) | gh secret set MXMDAW_PRATINJAU_KEYSTORE_B64 --repo masalikovic/MXM-DAW
```

**Cara B: lewat situs GitHub:** buka repo → **Settings** → **Secrets and
variables** → **Actions** → **New repository secret**, lalu isi ketiga secret
di atas. Untuk isi base64, buat dulu berkasnya (`base64 -w 0
mxmdaw-pratinjau.jks > kunci.b64`), salin isinya, lalu **hapus `kunci.b64`**
sesudahnya.

## Langkah 4. Kirim sidik jari (publik) ke agen

```sh
keytool -list -v -keystore mxmdaw-pratinjau.jks -alias pratinjau
```

Salin **hanya** baris yang diawali `SHA256:`, lalu kirim ke agen atau tulis di
PR. Agen memasukkannya ke `tools/sidik-kunci-pratinjau.txt`, dan sejak itu CI
menolak APK pratinjau yang ditandatangani kunci lain.

## Langkah 5. Cadangkan

Simpan `mxmdaw-pratinjau.jks` **dan** kata sandinya di **dua tempat** yang
berbeda, misalnya pengelola kata sandi dan flashdisk. Bila keduanya hilang,
APK pratinjau baru tidak bisa dipasang di atas yang lama: aplikasi harus
di-uninstall, dan proyek ikut terhapus.

## Bila ada masalah

| Gejala | Penyebab | Solusi |
|---|---|---|
| CI memberi peringatan "Secret … belum diatur" | Secret belum ada atau namanya salah | Periksa nama ketiga secret di langkah 3 |
| `cek-tanda-tangan.sh`: "belum berisi sidik jari" | Langkah 4 belum dilakukan | Cocokkan sidik jari di pesan CI dengan keluaran `keytool`, lalu kirim ke agen |
| `cek-tanda-tangan.sh`: "ditandatangani kunci lain" | Secret berisi keystore yang berbeda | Atur ulang secret dengan keystore yang benar; **jangan** mengganti sidik jari |
| Gradle: "keystore password was incorrect" | Kata sandi salah | Atur ulang `MXMDAW_PRATINJAU_PASSWORD` |
| `gh secret list`: "no secrets found" | Perintah dijalankan sebelum login selesai | Ulangi langkah 3 satu per satu |
| Job "Pratinjau" gagal dengan HTTP 500 | Galat sementara di server GitHub | `gh run rerun <id> --failed --repo masalikovic/MXM-DAW` |
| Nama langkah CI berisi `***` | GitHub menyamarkan teks yang sama dengan isi secret (alias `pratinjau`) | Normal, tidak perlu diperbaiki |
