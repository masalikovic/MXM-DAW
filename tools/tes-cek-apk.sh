#!/usr/bin/env bash
# Tes untuk tools/cek-apk.sh: pemeriksa yang tidak pernah gagal tidak
# membuktikan apa-apa. Setiap skenario membuat APK tiruan (zip berisi ELF
# kecil yang dikompilasi cc) dan aapt2 tiruan, lalu memeriksa kode keluarnya.
#
# Butuh: bash, cc, zip, unzip, readelf. Tidak butuh Android SDK.
set -euo pipefail

akar="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
kerja="$(mktemp -d)"
trap 'rm -rf "$kerja"' EXIT
gagal=0

# aapt2 tiruan: mencetak izin dari berkas $kerja/izin (satu nama per baris).
cat >"$kerja/aapt2" <<'AAPT'
#!/usr/bin/env bash
[ "$1" = dump ] && [ "$2" = permissions ] || exit 2
echo "package: io.github.masalikovic.mxmdaw"
while read -r n; do
  if [ -n "$n" ]; then echo "uses-permission: name='$n'"; fi
done <"$(dirname "$0")/izin"
AAPT
chmod +x "$kerja/aapt2"

# Simbol wajib tiruan, dan ELF yang mengekspor semuanya / sebagian / tidak ada.
printf '%s\n' frb_get_rust_content_hash frb_pde_ffi_dispatcher_sync >"$kerja/wajib"
export SIMBOL_WAJIB="$kerja/wajib"
cat >"$kerja/ada.c" <<'C'
int frb_get_rust_content_hash(void) { return 0; }
int frb_pde_ffi_dispatcher_sync(void) { return 0; }
C
echo 'int frb_get_rust_content_hash(void) { return 0; }' >"$kerja/sebagian.c"
echo 'int fungsi_lain(void) { return 0; }' >"$kerja/tanpa.c"
for n in ada sebagian tanpa; do cc -shared -fPIC -o "$kerja/$n.so" "$kerja/$n.c"; done

# buat_apk <so|-> [abi tambahan]
buat_apk() {
  local d="$kerja/apk" so="$1" tambahan="${2:-}"
  rm -rf "$d" "$kerja/uji.apk"
  mkdir -p "$d/lib/arm64-v8a"
  echo x >"$d/AndroidManifest.xml"
  [ "$so" != - ] && cp "$kerja/$so.so" "$d/lib/arm64-v8a/libmxm_engine.so"
  if [ -n "$tambahan" ]; then
    mkdir -p "$d/lib/$tambahan"
    cp "$kerja/ada.so" "$d/lib/$tambahan/libmxm_engine.so"
  fi
  (cd "$d" && zip -qr "$kerja/uji.apk" .)
}

# skenario <nama> <kode yang diharapkan> <izin (dipisah koma)> <so|-> [abi] [env...]
skenario() {
  local nama="$1" harap="$2" izin="$3" so="$4" abi="${5:-}"
  shift 5 || shift $#
  tr ',' '\n' <<<"$izin" >"$kerja/izin"
  buat_apk "$so" "$abi"
  local kode=0
  env AAPT2="$kerja/aapt2" "$@" "$akar/tools/cek-apk.sh" "$kerja/uji.apk" >"$kerja/log" 2>&1 || kode=$?
  if [ "$kode" = "$harap" ]; then
    echo "lulus: $nama (kode $kode)"
  else
    echo "GAGAL: $nama: kode $kode, seharusnya $harap"
    sed 's/^/       /' "$kerja/log"
    gagal=1
  fi
}

echo 'android.permission.RECORD_AUDIO' >"$kerja/izin-mic.txt"

skenario "APK bersih diterima" 0 "" ada ""
skenario "INTERNET ditolak (ADR-0006)" 1 "android.permission.INTERNET" ada ""
skenario "izin tak terdaftar ditolak" 1 "android.permission.RECORD_AUDIO" ada ""
skenario "izin terdaftar diterima" 0 "android.permission.RECORD_AUDIO" ada "" IZIN_RILIS="$kerja/izin-mic.txt"
skenario "INTERNET ditolak walau terdaftar" 1 "android.permission.INTERNET" ada "" \
  IZIN_RILIS=<(echo android.permission.INTERNET)
skenario "tanpa libmxm_engine.so ditolak" 1 "" - ""
skenario "ABI tambahan ditolak (ADR-0009)" 1 "" ada x86_64
skenario "tanpa fungsi untuk Dart ditolak" 1 "" tanpa ""
skenario "sebagian fungsi untuk Dart hilang ditolak" 1 "" sebagian ""
skenario "daftar simbol wajib kosong ditolak" 1 "" ada "" SIMBOL_WAJIB=/dev/null
skenario "melebihi batas ukuran ditolak" 1 "" ada "" BATAS_BYTE=100

exit "$gagal"
