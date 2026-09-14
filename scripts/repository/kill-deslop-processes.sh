#!/usr/bin/env bash
# Process scrub for Deslop's own executables. [DEPLOY-EXTENSION-BUNDLED-TESTS]
#
# A `deslop`, `deslop-lsp`, or `deslop-mcp` left running by a previous editor
# session or an abandoned test shadows the freshly built VSIX bundle, starves
# socket-bound integration tests, and — on Windows, where the loader holds an
# open handle to every running image — makes `cargo clean` unable to delete
# `target/release/*.exe` at all. So the rebuild targets scrub first.
#
# Matching is by exact process name, never by command line: `cargo build -p
# deslop-lsp` names the binary in its arguments and must survive untouched.
#
# The two backends exist because Git Bash on Windows ships no `pgrep`/`pkill`
# and Windows PIDs are not visible to `kill -0`; `tasklist`/`taskkill` are the
# native equivalents. `MSYS2_ARG_CONV_EXCL` stops the MSYS runtime rewriting
# `/FI`-style switches into filesystem paths before those tools see them.
#
# Idempotent — exits 0 when nothing matches. Non-zero only when a process
# survives being force-killed, which is a real problem the developer must see.
#
# Usage:
#   kill-deslop-processes.sh          terminate every match; non-zero if one survives
#   kill-deslop-processes.sh --list   print matching PIDs, kill nothing; always 0

set -euo pipefail

# Executables the VSIX bundles, and which must not be running during a rebuild.
PROCESS_NAMES=(deslop deslop-lsp deslop-mcp)

# How long a terminate request is given before survivors are force-killed.
GRACE_SECONDS=1

case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) IS_WINDOWS=1 ;;
  *) IS_WINDOWS=0 ;;
esac

export MSYS2_ARG_CONV_EXCL='*'

# PIDs of every running process whose image is exactly "$1" (plus `.exe`).
pids_named() {
  local name="$1"
  if [ "$IS_WINDOWS" -eq 1 ]; then
    tasklist /FI "IMAGENAME eq $name.exe" /FO CSV /NH 2>/dev/null |
      awk -F'","' -v want="\"$name.exe" '$1 == want { print $2 }'
  else
    pgrep -x "$name" 2>/dev/null || true
  fi
}

# Asks every process named "$1" to exit. Never fails: a console process with no
# window refuses the polite request, and the force pass below is what answers.
request_exit() {
  local name="$1"
  if [ "$IS_WINDOWS" -eq 1 ]; then
    taskkill /IM "$name.exe" >/dev/null 2>&1 || true
  else
    pkill -x "$name" 2>/dev/null || true
  fi
}

# True while PID "$1" is still running.
is_alive() {
  local pid="$1"
  if [ "$IS_WINDOWS" -eq 1 ]; then
    [ -n "$(tasklist /FI "PID eq $pid" /FO CSV /NH 2>/dev/null | grep '^"' || true)" ]
  else
    kill -0 "$pid" 2>/dev/null
  fi
}

# Terminates PID "$1" with prejudice.
force_kill() {
  local pid="$1"
  if [ "$IS_WINDOWS" -eq 1 ]; then
    taskkill /F /PID "$pid" >/dev/null 2>&1 || true
  else
    kill -9 "$pid" 2>/dev/null || true
  fi
}

# Every PID matching any bundled name, deduplicated, one per line.
matching_pids() {
  local name
  for name in "${PROCESS_NAMES[@]}"; do
    pids_named "$name"
  done | sort -u | grep -v '^$' || true
}

# Echoes back whichever of the given PIDs are still running.
survivors_of() {
  local pid
  for pid in "$@"; do
    if is_alive "$pid"; then printf '%s\n' "$pid"; fi
  done
}

# Force-kills "$@" and fails loudly if any of them outlive that.
finish_off() {
  echo "    force-killing holdouts: $*"
  local pid
  for pid in "$@"; do force_kill "$pid"; done
  sleep "$GRACE_SECONDS"
  local final
  final="$(survivors_of "$@" | tr '\n' ' ')"
  if [ -n "${final// /}" ]; then
    echo "FAIL: PIDs alive after a forced kill: $final"
    return 1
  fi
}

# Terminates everything matching, then proves it is gone.
scrub() {
  local initial
  initial="$(matching_pids | tr '\n' ' ')"
  if [ -z "${initial// /}" ]; then echo "    (none running)"; return 0; fi
  echo "    initial PIDs: $initial"
  local name
  for name in "${PROCESS_NAMES[@]}"; do request_exit "$name"; done
  sleep "$GRACE_SECONDS"
  local survivors
  survivors="$(survivors_of $initial | tr '\n' ' ')"
  if [ -n "${survivors// /}" ]; then finish_off $survivors; fi
  echo "    all targeted processes are dead (VSCode may auto-respawn — that is fine)"
}

case "${1:-}" in
  --list) matching_pids ;;
  "")
    echo "==> Killing any running ${PROCESS_NAMES[*]} processes..."
    scrub
    ;;
  *)
    echo "usage: $0 [--list]" >&2
    exit 2
    ;;
esac
