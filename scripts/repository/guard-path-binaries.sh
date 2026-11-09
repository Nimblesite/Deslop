#!/usr/bin/env bash
# PATH scrub for Deslop's own executables. [DEPLOY-EXTERNAL-MCP-CONSUMER]
#
# The VSIX is the only legitimate distribution surface. The VS Code extension,
# Claude Code MCP, Codex MCP, and every other host must resolve `deslop`,
# `deslop-lsp`, and `deslop-mcp` from the unpacked VSIX `bin/<platform>/`
# directory by absolute path. A copy left on PATH by `cargo install` or a
# package manager shadows the Shipwright-versioned bundle and drifts analysis
# off the wire contract, so `make test` and every `_vsix-*` target scrub first.
#
# Issue #474: this scrub used to detect with `command -v`, which reports only a
# *resolvable executable*. A `~/.local/bin/deslop-mcp` symlink pointing at a
# deleted `deslop-live-0.15.0-darwin-arm64` bundle is not resolvable, so the
# scrub never saw it, deleted nothing, and still exited 0 — a gate reporting
# success while the very name it exists to remove sat on disk. Detection now
# walks PATH itself and tests `-L` (the link, dangling or not) as well as `-f`,
# and the exit status comes from re-running that detector after the deletions.
#
# Detection uses bash builtins only, never `tr` or `sort`: a scrub that has to
# find helper programs on PATH cannot be trusted to report honestly about that
# same PATH. Written for bash 3.2, the version macOS ships — no associative
# arrays, no `mapfile`.
#
# Usage:
#   scrub-path-binaries.sh           delete every shadowing entry; non-zero if one survives
#   scrub-path-binaries.sh --list    print what is shadowing, delete nothing; non-zero if any

set -euo pipefail

# Executables the VSIX bundles, and which must never be resolvable elsewhere.
BINARY_NAMES=(deslop deslop-lsp deslop-mcp)

# Windows hosts spell the same names with an extension.
BINARY_SUFFIXES=("" ".exe")

# Deletions can race a package manager writing the name back, and a name the
# scrub cannot remove at all must not spin forever; give up after this many
# rounds and fail loudly with whatever survived.
MAX_ATTEMPTS=10

# Every directory on PATH, one per line. An empty PATH field means "." in
# POSIX, so it is emitted as such rather than silently dropped.
path_directories() {
  local IFS=':'
  local -a fields=()
  read -ra fields <<< "${PATH:-}"
  local field
  for field in ${fields[@]+"${fields[@]}"}; do
    printf '%s\n' "${field:-.}"
  done
}

# True when "$1" already appears among the remaining arguments.
contains() {
  local needle="$1"
  shift
  local item
  for item in ${@+"$@"}; do
    if [ "$item" = "$needle" ]; then return 0; fi
  done
  return 1
}

# True when `$1` is a name that shadows the bundled binary. A dangling symlink
# counts — that is the whole point of #474 — and so does a regular file that
# happens not to be executable. A directory (or a symlink to one) does not:
# it can never be executed, so removing it would be outside this gate's remit.
is_shadowing() {
  local candidate="$1"
  if [ -d "$candidate" ]; then return 1; fi
  if [ -L "$candidate" ] || [ -f "$candidate" ]; then return 0; fi
  return 1
}

# Every shadowing path on PATH, in PATH order, without repeats.
shadowing_paths() {
  local -a found=()
  local directory name suffix candidate
  while IFS= read -r directory; do
    for name in "${BINARY_NAMES[@]}"; do
      for suffix in "${BINARY_SUFFIXES[@]}"; do
        candidate="$directory/$name$suffix"
        if is_shadowing "$candidate" && ! contains "$candidate" ${found[@]+"${found[@]}"}; then
          found+=("$candidate")
        fi
      done
    done
  done < <(path_directories)
  if [ "${#found[@]}" -gt 0 ]; then printf '%s\n' "${found[@]}"; fi
}

# Removes the copies a package manager owns, before walking PATH.
uninstall_packaged() {
  if command -v brew >/dev/null 2>&1; then
    brew uninstall --force deslop >/dev/null 2>&1 || true
  fi
  local name
  for name in "${BINARY_NAMES[@]}"; do
    cargo uninstall "$name" >/dev/null 2>&1 || true
    if [ -n "${HOME:-}" ]; then
      rm -f "$HOME/.cargo/bin/$name" "$HOME/.cargo/bin/$name.exe" || true
    fi
  done
}

# Explains what survived and what the developer has to do about it.
report_survivors() {
  echo "FAIL: these PATH entries still shadow the VSIX-bundled binaries:"
  printf '  %s\n' "$@"
  echo "Remove them before running tests; extension tests must use bundled binaries by absolute path."
  echo "Re-check with: bash scripts/repository/scrub-path-binaries.sh --list"
}

# Reads the detector's output into the `remaining` array in the caller's scope.
collect_shadowing() {
  remaining=()
  local candidate
  while IFS= read -r candidate; do
    remaining+=("$candidate")
  done < <(shadowing_paths)
}

# Deletes until PATH is clear, or fails with everything that survived.
scrub() {
  local attempt=0 candidate
  local -a remaining=()
  while :; do
    collect_shadowing
    if [ "${#remaining[@]}" -eq 0 ]; then return 0; fi
    if [ "$attempt" -ge "$MAX_ATTEMPTS" ]; then
      report_survivors "${remaining[@]}"
      return 1
    fi
    for candidate in "${remaining[@]}"; do
      echo "    deleting $candidate"
      rm -f "$candidate" || true
    done
    hash -r 2>/dev/null || true
    attempt=$((attempt + 1))
  done
}

# Second, independent check: nothing may resolve as a command either, which
# also catches a shell function or builtin shadowing the name.
assert_nothing_resolves() {
  local name found
  for name in "${BINARY_NAMES[@]}"; do
    found="$(command -v "$name" 2>/dev/null || true)"
    if [ -n "$found" ]; then
      report_survivors "$found"
      return 1
    fi
  done
}

# Prints what is shadowing right now, and fails if anything is.
list_shadowing() {
  local -a remaining=()
  collect_shadowing
  if [ "${#remaining[@]}" -eq 0 ]; then return 0; fi
  printf '%s\n' "${remaining[@]}"
  return 1
}

case "${1:-}" in
  --list) list_shadowing ;;
  "")
    echo "==> Removing Deslop binaries from PATH..."
    uninstall_packaged
    scrub
    assert_nothing_resolves
    echo "    PATH is clear of ${BINARY_NAMES[*]}"
    ;;
  *)
    echo "usage: $0 [--list]" >&2
    exit 2
    ;;
esac
