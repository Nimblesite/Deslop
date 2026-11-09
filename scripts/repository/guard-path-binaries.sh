#!/usr/bin/env bash
# PATH guard for Deslop's own executables. [DEPLOY-EXTENSION-BUNDLED-TESTS]
#
# Tests run the binaries bundled in the extension, by absolute path. A
# `deslop` resolved from PATH (a Homebrew, Scoop or curl-installer copy, or a
# forbidden `cargo install`) would mask a missing or stale bundle. So every test
# recipe puts this guard directory first on PATH: each name resolves to a stub
# that refuses to run and says why, and a test that reaches for PATH fails
# loudly instead of passing on another build.
#
# Nothing here uninstalls or deletes anything (#611). Homebrew, Scoop and the
# curl installer are the supported PATH channels for the agents on this machine
# ([DEPLOY-EXTERNAL-MCP-CONSUMER]). The scrub this replaces removed them, and
# every agent's MCP server then failed to start, in every project.
#
# Written for bash 3.2, the version macOS ships.
#
# Usage: guard-path-binaries.sh <guard-dir>
#   Writes the stubs into <guard-dir>, then fails unless every name resolves
#   to its stub on the current PATH.

set -euo pipefail

# Executables the VSIX bundles, which a test must never resolve from PATH.
BINARY_NAMES=(deslop deslop-lsp deslop-mcp)

# The arguments this script takes: exactly the guard directory.
EXPECTED_ARGUMENTS=1

# Status for a call that names no guard directory.
USAGE_STATUS=2

# Why a stub refused, for the test log that shows it.
REFUSAL="refused: tests run the extension's bundled binaries by absolute path, never from PATH ([DEPLOY-EXTENSION-BUNDLED-TESTS])."

# A POSIX stub and a cmd.exe stub for `$2`, written into directory `$1`.
write_stubs() {
  local directory="$1" name="$2"
  printf '#!/bin/sh\necho "%s: %s" >&2\nexit 1\n' "$name" "$REFUSAL" > "$directory/$name"
  chmod +x "$directory/$name"
  printf '@echo %s: %s 1>&2\r\n@exit /b 1\r\n' "$name" "$REFUSAL" > "$directory/$name.cmd"
}

# Fails, naming each culprit, unless every name resolves to its stub in `$1`.
assert_guarded() {
  local directory="$1" name resolved failed=0
  hash -r 2>/dev/null || true
  for name in "${BINARY_NAMES[@]}"; do
    resolved="$(command -v "$name" 2>/dev/null || true)"
    if [ "$resolved" != "$directory/$name" ]; then
      echo "FAIL: $name resolves to '${resolved:-nothing}', not the guard $directory/$name."
      failed=1
    fi
  done
  if [ "$failed" -ne 0 ]; then
    echo "Put $directory first on PATH for every test recipe (the Makefile's GUARDED_PATH)."
    return 1
  fi
}

if [ "$#" -ne "$EXPECTED_ARGUMENTS" ] || [ -z "$1" ]; then
  echo "usage: $0 <guard-dir>" >&2
  exit "$USAGE_STATUS"
fi

mkdir -p "$1"
GUARD_DIR="$(cd "$1" && pwd)"
for binary in "${BINARY_NAMES[@]}"; do write_stubs "$GUARD_DIR" "$binary"; done
assert_guarded "$GUARD_DIR"
echo "==> PATH guard: ${BINARY_NAMES[*]} resolve to $GUARD_DIR"
