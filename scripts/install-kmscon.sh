#!/usr/bin/env bash
# Moved: the real script is now install-kmscon.sh at the repo root. This stub
# stays so old links and curl one-liners keep working: it downloads the
# current one from main and runs it with the same arguments.
set -euo pipefail
url="https://raw.githubusercontent.com/${REPO:-ErikAllanKincaid/tetron-wm}/${BRANCH:-main}/install-kmscon.sh"
tmp="$(mktemp)"; trap 'rm -f "$tmp"' EXIT
if command -v curl >/dev/null 2>&1; then curl -fsSL "$url" > "$tmp"
elif command -v wget >/dev/null 2>&1; then wget -qO- "$url" > "$tmp"
else echo "scripts/install-kmscon.sh: need curl or wget" >&2; exit 1; fi
bash "$tmp" "$@"
