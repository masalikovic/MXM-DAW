#!/usr/bin/env bash
# Memastikan APK ditandatangani kunci pratinjau pemilik (ADR-0012).
#
#   tools/cek-tanda-tangan.sh <berkas.apk>
#
# Membandingkan sidik jari SHA-256 sertifikat penanda tangan APK dengan
# tools/sidik-kunci-pratinjau.txt. Gagal bila berbeda, bila APK ditandatangani
# lebih dari satu kunci, atau bila sidik jari belum diisi (pesannya memuat
# sidik jari APK supaya pemilik bisa mencocokkannya dengan keytool).
#
# Variabel lingkungan (dipakai tools/tes-cek-tanda-tangan.sh):
#   APKSIGNER   bawaannya apksigner terbaru di $ANDROID_HOME/build-tools
#   SIDIK_KUNCI bawaannya tools/sidik-kunci-pratinjau.txt
set -euo pipefail

apk="${1:?Pemakaian: tools/cek-tanda-tangan.sh <berkas.apk>}"
akar="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
berkas_sidik="${SIDIK_KUNCI:-$akar/tools/sidik-kunci-pratinjau.txt}"
if [ -z "${APKSIGNER:-}" ]; then
  APKSIGNER="$(ls -d "${ANDROID_HOME:?ANDROID_HOME belum diatur}"/build-tools/*/apksigner | sort -V | tail -1)"
fi

# Bentuk baku: huruf kecil, tanpa titik dua (keytool menulis AA:BB:…).
baku() { tr -d ': \r' | tr 'A-F' 'a-f'; }

keluaran="$("$APKSIGNER" verify --print-certs "$apk")"
# Format baris berbeda antarversi apksigner, mis. "Signer #1 certificate
# SHA-256 digest: …" atau "Signer (minSdkVersion=…) certificate SHA-256
# digest: …" (skema v3.1). Satu sertifikat bisa muncul di beberapa baris,
# jadi yang dihitung adalah sertifikat yang berbeda.
sidik_apk="$(sed -n 's/.*[Cc]ertificate SHA-256 digest: *//p' <<<"$keluaran" | baku | sort -u)"
jumlah="$(grep -c . <<<"$sidik_apk" || true)"
if [ "$jumlah" -ne 1 ]; then
  echo "GAGAL: APK harus ditandatangani tepat satu kunci; ditemukan $jumlah."
  echo "       Baris sertifikat dari apksigner (informasi publik):"
  { grep -i 'certificate\|signer' <<<"$keluaran" || echo "(tidak ada)"; } | sed 's/^/         /'
  exit 1
fi

harapan="$({ grep -v '^\s*#' "$berkas_sidik" || true; } | sed '/^\s*$/d' | head -1 | baku)"
if [ -z "$harapan" ]; then
  echo "GAGAL: tools/sidik-kunci-pratinjau.txt belum berisi sidik jari."
  echo "       Sidik jari APK ini: $sidik_apk"
  echo "       Cocokkan dengan baris SHA256 dari 'keytool -list -v' milik pemilik"
  echo "       sebelum menyalinnya ke berkas itu (docs/panduan-kunci-pratinjau.md)."
  exit 1
fi
if [ "$sidik_apk" = "$harapan" ]; then
  echo "ok   : sidik jari penanda tangan APK cocok dengan $(basename "$berkas_sidik") ($sidik_apk)"
else
  echo "GAGAL: APK ditandatangani kunci lain."
  echo "       diharapkan: $harapan"
  echo "       di APK    : $sidik_apk"
  exit 1
fi
