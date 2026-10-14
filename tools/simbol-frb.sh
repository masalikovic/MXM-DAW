#!/usr/bin/env bash
# Mencetak nama fungsi native yang dicari Dart (flutter_rust_bridge) di
# pustaka mesin, satu per baris. Daftar dibaca dari paket flutter_rust_bridge
# yang benar-benar terpasang (lewat app/.dart_tool/package_config.json) dan
# dari binding hasil generator, jadi ikut berubah bila versinya naik.
#
# Butuh: `flutter pub get` sudah dijalankan di app/, python3.
set -euo pipefail

akar="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
paket="$(python3 - "$akar/app/.dart_tool/package_config.json" <<'PY'
import json, sys
from urllib.parse import urlparse, unquote
for p in json.load(open(sys.argv[1]))["packages"]:
    if p["name"] == "flutter_rust_bridge":
        print(unquote(urlparse(p["rootUri"]).path))
        break
else:
    sys.exit("flutter_rust_bridge tidak ada di package_config.json")
PY
)"

grep -rhoE "'(frb_[a-z0-9_]+|store_dart_post_cobject)'" \
  "$paket/lib" "$akar/app/lib/engine/bridge" | tr -d "'" | sort -u
