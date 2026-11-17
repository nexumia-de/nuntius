#!/usr/bin/env bash
# Regenerates the vendored cargo sources of the Flatpak manifest
# build-aux/de.nexumia.Nuntius.json, so the build needs no network:
#
#   build-aux/nuntius-cargo-sources.json  from Cargo.lock
#   build-aux/grass-cargo-sources.json    from grass' Cargo.lock at the
#                                         commit the manifest pins
#
# Run it after every change of Cargo.lock (also after an upstream merge)
# or of the grass module. With --check it changes nothing and fails if a
# file is out of date (CI). Needs git, python3 and network access to
# github.com and pypi.org. See docs/flatpak.md.
set -euo pipefail

# flatpak-builder-tools (MIT), pinned; docs/dependencies.md.
TOOLS_URL=https://github.com/flatpak/flatpak-builder-tools.git
TOOLS_COMMIT=9d86d336a39b9af38b86f114b6ce5820fe260f91

check=false
[[ ${1:-} == --check ]] && check=true

root=$(cd "$(dirname "$0")/.." && pwd)
manifest=$root/build-aux/de.nexumia.Nuntius.json
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fetch() { # url commit dir
  git init -q "$3"
  git -C "$3" fetch -q --depth 1 "$1" "$2"
  git -C "$3" checkout -q FETCH_HEAD
}

fetch "$TOOLS_URL" "$TOOLS_COMMIT" "$work/tools"
python3 -m venv "$work/venv"
"$work/venv/bin/pip" install -q --disable-pip-version-check \
  -r "$root/build-aux/cargo-sources-requirements.txt"

read -r grass_url grass_commit < <(python3 -I -c '
import json, sys
m = json.load(open(sys.argv[1]))
src = next(x for x in m["modules"] if x["name"] == "grass")["sources"][0]
print(src["url"], src["commit"])' "$manifest")
fetch "$grass_url" "$grass_commit" "$work/grass"

stale=0
generate() { # Cargo.lock output
  local out=$root/build-aux/$2
  "$work/venv/bin/python" -I "$work/tools/cargo/flatpak-cargo-generator.py" \
    "$1" -o "$work/$2"
  if $check; then
    cmp -s "$work/$2" "$out" ||
      { echo "::error file=build-aux/$2::build-aux/$2 is out of date; run build-aux/update-cargo-sources.sh."; stale=1; }
  else
    mv "$work/$2" "$out"
    echo "Wrote build-aux/$2"
  fi
}

generate "$root/Cargo.lock" nuntius-cargo-sources.json
generate "$work/grass/Cargo.lock" grass-cargo-sources.json
exit $stale
