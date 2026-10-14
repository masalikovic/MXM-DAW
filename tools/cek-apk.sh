#!/usr/bin/env bash
# Pemeriksaan APK sebelum dipercaya (ADR-0003, ADR-0006, ADR-0009, rencana §11).
#
#   tools/cek-apk.sh <berkas.apk>
#
# Gagal (kode keluar 1) bila:
#   - ada izin di luar tools/izin-rilis.txt, termasuk INTERNET (ADR-0006);
#   - libmxm_engine.so tidak ada di lib/arm64-v8a/ (ADR-0003);
#   - ada ABI selain arm64-v8a (ADR-0009 butir 8);
#   - ada fungsi native yang dicari Dart (flutter_rust_bridge) tetapi tidak
#     diekspor libmxm_engine.so; Dart lalu gagal memanggil Rust di HP;
#   - ukuran APK melebihi 25 MB (rencana §11).
#
# Variabel lingkungan (dipakai tools/tes-cek-apk.sh):
#   AAPT2         bawaannya aapt2 terbaru di $ANDROID_HOME/build-tools
#   SIMBOL_WAJIB  berkas daftar simbol; bawaannya keluaran tools/simbol-frb.sh
#   IZIN_RILIS    bawaannya tools/izin-rilis.txt
#   BATAS_BYTE    bawaannya 26214400 (25 MB)
set -euo pipefail

apk="${1:?Pemakaian: tools/cek-apk.sh <berkas.apk>}"
akar="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
daftar_izin="${IZIN_RILIS:-$akar/tools/izin-rilis.txt}"
batas_byte="${BATAS_BYTE:-26214400}" # 25 MB
gagal=0

salah() { echo "GAGAL: $*"; gagal=1; }
lulus() { echo "ok   : $*"; }

if [ -z "${AAPT2:-}" ]; then
  AAPT2="$(ls -d "${ANDROID_HOME:?ANDROID_HOME belum diatur}"/build-tools/*/aapt2 | sort -V | tail -1)"
fi

# 1. Izin. Baris aapt2: "uses-permission: name='android.permission.X'".
izin="$("$AAPT2" dump permissions "$apk" |
  sed -n "s/^uses-permission: name='\([^']*\)'.*/\1/p" | sort -u)"
echo "Izin di APK:"
echo "${izin:-  (tidak ada)}" | sed 's/^/  /'
if grep -qx 'android.permission.INTERNET' <<<"$izin"; then
  salah "APK meminta INTERNET; build rilis dilarang memakainya (ADR-0006)"
fi
# `|| true`: daftar boleh kosong (hanya komentar); grep lalu keluar dengan
# kode 1, dan tanpa ini `set -e` menghentikan skrip diam-diam.
diizinkan="$({ grep -v '^\s*#' "$daftar_izin" || true; } | sed '/^\s*$/d' | sort -u)"
lebih="$(comm -23 <(echo "$izin" | sed '/^$/d') <(echo "$diizinkan"))"
if [ -n "$lebih" ]; then
  salah "izin di luar tools/izin-rilis.txt: $(echo $lebih)"
else
  lulus "semua izin ada di tools/izin-rilis.txt"
fi

# 2. Pustaka native dan ABI.
isi="$(unzip -Z1 "$apk")"
if grep -qx 'lib/arm64-v8a/libmxm_engine.so' <<<"$isi"; then
  lulus "lib/arm64-v8a/libmxm_engine.so ada"
else
  salah "lib/arm64-v8a/libmxm_engine.so tidak ada di APK (ADR-0003)"
fi
abi="$({ grep -o '^lib/[^/]*/' <<<"$isi" || true; } | sort -u | sed 's#lib/##; s#/##')"
if [ "$(echo $abi)" = "arm64-v8a" ]; then
  lulus "hanya ABI arm64-v8a"
else
  salah "ABI di APK: $(echo $abi); seharusnya hanya arm64-v8a (ADR-0009)"
fi

# 3. Setiap fungsi native yang dicari Dart diekspor libmxm_engine.so.
#    Tanpa --whole-archive di CMakeLists.txt, linker membuang semuanya.
sementara="$(mktemp -d)"
trap 'rm -rf "$sementara"' EXIT
if [ -n "${SIMBOL_WAJIB:-}" ]; then
  cp "$SIMBOL_WAJIB" "$sementara/wajib"
else
  "$akar/tools/simbol-frb.sh" >"$sementara/wajib"
fi
sort -u -o "$sementara/wajib" "$sementara/wajib"
if unzip -q -o "$apk" 'lib/arm64-v8a/libmxm_engine.so' -d "$sementara" 2>/dev/null; then
  readelf --dyn-syms --wide "$sementara/lib/arm64-v8a/libmxm_engine.so" |
    awk '$5 == "GLOBAL" && $7 != "UND" {print $8}' | sort -u >"$sementara/ekspor"
  hilang="$(comm -23 "$sementara/wajib" "$sementara/ekspor")"
  jumlah="$(wc -l <"$sementara/wajib")"
  if [ "$jumlah" -eq 0 ]; then
    salah "daftar simbol wajib kosong; pemeriksaan ini tidak membuktikan apa-apa"
  elif [ -z "$hilang" ]; then
    lulus "$jumlah fungsi yang dicari Dart diekspor libmxm_engine.so"
  else
    salah "fungsi yang dicari Dart tidak diekspor: $(echo $hilang)"
  fi
fi

# 4. Ukuran.
ukuran="$(stat -c %s "$apk")"
mb="$(awk -v b="$ukuran" 'BEGIN {printf "%.1f", b / 1048576}')"
if [ "$ukuran" -le "$batas_byte" ]; then
  lulus "ukuran ${mb} MB (batas $((batas_byte / 1048576)) MB)"
else
  salah "ukuran ${mb} MB melebihi batas $((batas_byte / 1048576)) MB (rencana §11)"
fi

exit "$gagal"
