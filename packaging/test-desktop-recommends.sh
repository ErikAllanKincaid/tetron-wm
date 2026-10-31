#!/bin/sh
# Install tetron-wm and tetron-wm-desktop together in a clean container WITH
# Recommends (as `apt install tetron-wm-desktop` does by default), then check
# that every Recommends of tetron-wm-desktop is installed; for an `a | b`
# alternative, one of them. The Recommends list is read from the .deb itself.
# mkdeb's own install test uses --no-install-recommends, so it does not cover
# this (WMDEB-007).
# Usage: packaging/test-desktop-recommends.sh TETRON_WM.deb TETRON_WM_DESKTOP.deb [IMAGE]
set -eu
[ $# -ge 2 ] || { echo "usage: $0 tetron-wm.deb tetron-wm-desktop.deb [image]" >&2; exit 2; }
wm="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
desk="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
image="${3:-debian:trixie}"
recs="$(dpkg-deb -f "$desk" Recommends)"
[ -n "$recs" ] || { echo "FAIL: $desk has no Recommends" >&2; exit 1; }
echo "== $image: tetron-wm-desktop with Recommends"
docker run --rm -v "$wm:/debs/wm.deb:ro" -v "$desk:/debs/desk.deb:ro" -e RECS="$recs" "$image" sh -c '
set -e
apt-get update -qq >/dev/null 2>&1
if ! DEBIAN_FRONTEND=noninteractive apt-get install -y -qq /debs/wm.deb /debs/desk.deb >/tmp/apt.log 2>&1; then
  tail -20 /tmp/apt.log; echo "FAIL: apt install"; exit 1
fi
fail=0
IFS=,
for alt in $RECS; do
  ok=""
  IFS="|"
  for p in $alt; do
    p="$(echo "$p" | sed "s/(.*//; s/ //g")"
    [ -n "$p" ] && dpkg -s "$p" >/dev/null 2>&1 && ok="$p"
  done
  IFS=,
  if [ -n "$ok" ]; then echo "  ok   $ok"; else echo "  FAIL $alt not installed"; fail=1; fi
done
[ "$fail" = 0 ] && echo "ALL PASS"
exit "$fail"
'
