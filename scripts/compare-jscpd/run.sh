#!/bin/bash
# Scans each judged corpus checkout with deslop and jscpd, recording wall
# time and peak memory (macOS /usr/bin/time -l). Checkouts are the existing
# pinned judging sources; nothing is cloned or pulled.
#
# usage: ./run.sh [repo ...]   (default: every repo in repos.tsv)
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
DESLOP="${DESLOP:-$REPO/target/release/deslop}"
JSCPD="$HERE/node_modules/.bin/jscpd"
SOURCES="${SOURCES:-$HOME/clone-judging-queued-claude}"
OUT="$REPO/target/compare-jscpd/results"
# Every language both tools support; jscpd otherwise also scans markdown,
# YAML and the rest, which no register judges.
FORMATS="c,csharp,dart,fsharp,go,javascript,jsx,php,python,rust,tsx,typescript"
mkdir -p "$OUT"
printf 'deslop %s at %s\njscpd %s\n' "$("$DESLOP" --version)" "$(git -C "$REPO" rev-parse --short HEAD)" \
  "$("$JSCPD" --version)" >"$OUT/builds.txt"

measure() { # <time-file> <command...>
  local file="$1"; shift
  /usr/bin/time -l "$@" >/dev/null 2>"$file.stderr"
  local status=$?
  {
    echo "status=$status"
    awk '/ real /{print "wall_seconds="$1} /maximum resident set size/{print "peak_rss_bytes="$1}' "$file.stderr"
  } >"$file"
  rm -f "$file.stderr"
}

while IFS=$'\t' read -r register dir; do
  [ -z "$register" ] || [[ "$register" == \#* ]] && continue
  if [ "$#" -gt 0 ] && ! printf '%s\n' "$@" | grep -qx "$register"; then continue; fi
  src="$SOURCES/$dir/source"
  echo "==> $register ($src)"
  ( cd "$src" && measure "$OUT/$register.deslop.time" \
      "$DESLOP" . --no-incremental --nohtml --notext --no-fail-over \
      --output "$OUT/$register.deslop" )
  rm -rf "$OUT/logs"
  rm -rf "$OUT/$register.jscpd" && mkdir -p "$OUT/$register.jscpd"
  ( cd "$src" && measure "$OUT/$register.jscpd.time" \
      "$JSCPD" . --silent --reporters json --format "$FORMATS" \
      --ignore "**/node_modules/**,**/.git/**" \
      --output "$OUT/$register.jscpd" )
  cat "$OUT/$register.deslop.time" "$OUT/$register.jscpd.time" | tr '\n' ' '; echo
done <"$HERE/repos.tsv"
