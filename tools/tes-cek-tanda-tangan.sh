#!/usr/bin/env bash
# Tes untuk tools/cek-tanda-tangan.sh dengan apksigner tiruan.
set -euo pipefail

akar="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
kerja="$(mktemp -d)"
trap 'rm -rf "$kerja"' EXIT
gagal=0
touch "$kerja/uji.apk"

# apksigner tiruan: mencetak satu baris "Signer" per sidik jari di $kerja/penanda.
cat >"$kerja/apksigner" <<'TIRU'
#!/usr/bin/env bash
i=1
while read -r s; do
  echo "Signer #$i certificate DN: CN=Pemilik"
  echo "Signer #$i certificate SHA-256 digest: $s"
  i=$((i + 1))
done <"$(dirname "$0")/penanda"
TIRU
chmod +x "$kerja/apksigner"

A=aa11bb22cc33dd44ee55ff6600112233445566778899aabbccddeeff00112233
B=0000000000000000000000000000000000000000000000000000000000000000
KEYTOOL_A="AA:11:BB:22:CC:33:DD:44:EE:55:FF:66:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33"

# skenario <nama> <kode harapan> <sidik di berkas> <sidik penanda APK, dipisah koma>
skenario() {
  printf '# komentar\n%s\n' "$3" >"$kerja/sidik.txt"
  tr ',' '\n' <<<"$4" >"$kerja/penanda"
  local kode=0
  APKSIGNER="$kerja/apksigner" SIDIK_KUNCI="$kerja/sidik.txt" \
    "$akar/tools/cek-tanda-tangan.sh" "$kerja/uji.apk" >"$kerja/log" 2>&1 || kode=$?
  if [ "$kode" = "$2" ]; then
    echo "lulus: $1 (kode $kode)"
  else
    echo "GAGAL: $1: kode $kode, seharusnya $2"; sed 's/^/       /' "$kerja/log"; gagal=1
  fi
}

skenario "kunci yang sama diterima" 0 "$A" "$A"
skenario "format keytool (AA:11:…) diterima" 0 "$KEYTOOL_A" "$A"
skenario "kunci lain ditolak" 1 "$A" "$B"
skenario "sidik jari belum diisi ditolak" 1 "" "$A"
skenario "dua penanda tangan ditolak" 1 "$A" "$A,$B"
skenario "APK tanpa tanda tangan ditolak" 1 "$A" ""

exit "$gagal"
