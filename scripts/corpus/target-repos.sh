#!/usr/bin/env bash
## target-repos: the pieces every corpus run needs — logging, target parsing,
## pinned checkouts of the repositories being scanned, and the register-backed
## default target list. Sourced by `scripts/compare-versions.sh` and
## `scripts/corpus/score-gate.sh`, which would otherwise carry two copies of the
## same fetch-and-verify dance.
##
## [CORPUS-PIN] A target is `url#<commit-id>#language`, and the commit id is a
## full 40-character git object name — never a tag, never a branch, never a
## version string. A version label is a name somebody can re-point at different
## source; a commit id IS the source. Every register is judged at one commit and
## scored against a scan of that same commit, so a pin that can move is a silent
## way to score an engine against code the judge never read.
##
## A caller must set WORK_DIR before sourcing.

UNSPECIFIED_LANGUAGE="unspecified"
: "${SHORT_SHA_LENGTH:=12}"
export SHORT_SHA_LENGTH
# A git object name in full. Nothing shorter is accepted: an abbreviation is
# ambiguous across repositories, and a tag or branch name is not a pin at all.
COMMIT_ID_LENGTH=40
# Where the register-backed default target list is read from.
REGISTER_DIR="corpus/register"
# Repositories queued for a first judging pass: scanned like a register, but
# with no verdicts yet, because a judge cannot rule on pairs nothing produced.
JUDGING_QUEUE="corpus/judging-queue.json"

log() { echo "==> $*" >&2; }
die() { echo "==> ERROR: $*" >&2; exit 1; }

short_sha() { printf '%.*s' "$SHORT_SHA_LENGTH" "$1"; }

# ---------------------------------------------------------------------------
# is-commit-id: true when $1 is a full, lowercase, hexadecimal git object name.
# Glob, not a pattern match on source: this reads one configuration value.
is_commit_id() {
  [ "${#1}" -eq "$COMMIT_ID_LENGTH" ] || return 1
  case "$1" in *[!0-9a-f]*) return 1 ;; esac
}

# ---------------------------------------------------------------------------
# parse-targets: split each `url#commit#language` argument into the parallel
# TARGET_* arrays, refusing anything not pinned to a commit id.
parse_targets() {
  TARGET_URLS=(); TARGET_LANGS=(); TARGET_SLUGS=(); TARGET_SHAS=()
  local spec url rest sha language
  for spec in "$@"; do
    # Split on the first two '#' only. `read -r` with IFS='#' would eat a
    # trailing delimiter and turn the language `C#` into `C`.
    url="${spec%%#*}"
    rest="${spec#"$url"}"; rest="${rest#\#}"
    sha="${rest%%#*}"
    language="${rest#"$sha"}"; language="${language#\#}"
    [ -n "$url" ] || die "target '$spec' names no repository url"
    is_commit_id "$sha" || die "target '$spec' is pinned by '${sha:-nothing}', which is not a \
$COMMIT_ID_LENGTH-character commit id — pin the commit, never a tag or a version"
    [ -n "$language" ] || language="$UNSPECIFIED_LANGUAGE"
    TARGET_URLS+=("$url")
    TARGET_LANGS+=("$language")
    TARGET_SLUGS+=("$(basename "$url" .git)")
    TARGET_SHAS+=("$sha")
  done
}

# ---------------------------------------------------------------------------
# register-targets: print one `url#commit#language` spec per register named in
# $@, or for every register when called with none. The url and the commit come
# from the register itself, so a scan can only ever read the source the judge
# read; a pin repeated in a caller could drift away from it.
register_targets() {
  local root="$1"; shift
  local names=("$@") name register
  if [ ${#names[@]} -eq 0 ]; then
    while IFS= read -r register; do names+=("$(basename "$register" .json)"); done \
      < <(find "$root/$REGISTER_DIR" -maxdepth 1 -name '*.json' ! -name 'score-*' | sort)
  fi
  for name in "${names[@]}"; do
    register="$root/$REGISTER_DIR/$name.json"
    [ -f "$register" ] || die "no register for '$name' at $register"
    node -e '
      const register = require(process.argv[1]);
      for (const field of ["url", "sha", "language"]) {
        if (!register[field]) throw new Error(`${process.argv[1]} has no ${field}`);
      }
      process.stdout.write(`${register.url}#${register.sha}#${register.language}\n`);
    ' "$register"
  done
}

# ---------------------------------------------------------------------------
# queued-targets: print one `url#commit#language` spec per repository waiting on
# its first judging pass. Without these a new repository could never enter the
# register at all: a workspace needs two reports, and a comparison would only
# ever scan repositories that already have a register.
queued_targets() {
  local root="$1" queue="$1/$JUDGING_QUEUE"
  [ -f "$queue" ] || return 0
  node -e '
    const queue = require(process.argv[1]);
    for (const repository of queue.repositories) {
      for (const field of ["url", "sha", "language"]) {
        if (!repository[field]) throw new Error(`${repository.name} has no ${field}`);
      }
      process.stdout.write(`${repository.url}#${repository.sha}#${repository.language}\n`);
    }
  ' "$queue"
}

# ---------------------------------------------------------------------------
# default-targets: every repository a wide comparison should scan — the judged
# registers first, then the ones queued to become registers.
default_targets() {
  register_targets "$1"
  queued_targets "$1"
}

# ---------------------------------------------------------------------------
# fetch-commit: pull commit $2 of remote $1 into the initialised repository $3.
# A shallow single-commit fetch is the fast path; remotes that refuse to serve
# an arbitrary commit fall back to a full fetch, which lands on the same commit.
fetch_commit() {
  local url="$1" sha="$2" repo_dir="$3"
  git -C "$repo_dir" remote add origin "$url"
  git -C "$repo_dir" fetch --quiet --depth 1 origin "$sha" 2>/dev/null && return
  log "$url will not serve a single commit; falling back to a full fetch"
  git -C "$repo_dir" fetch --quiet origin
}

# ---------------------------------------------------------------------------
# prepare-target-repo: check $1 (url) out at $2 (commit id) once and verify the
# resulting HEAD. Prints the checkout path.
prepare_target_repo() {
  local url="$1" sha="$2" repo_dir slug head
  slug="$(basename "$url" .git)"
  repo_dir="$WORK_DIR/repos/$slug"
  if [ -d "$repo_dir/.git" ] && [ "$(git -C "$repo_dir" rev-parse HEAD 2>/dev/null)" = "$sha" ]; then
    log "target $slug already at $sha, reusing"
    printf '%s' "$repo_dir"
    return
  fi
  rm -rf "$repo_dir"
  mkdir -p "$repo_dir"
  log "fetching $url at commit $sha"
  git -c init.defaultBranch=main -C "$repo_dir" init --quiet
  fetch_commit "$url" "$sha" "$repo_dir"
  git -C "$repo_dir" checkout --quiet "$sha"
  head="$(git -C "$repo_dir" rev-parse HEAD)"
  [ "$head" = "$sha" ] || die "$slug checked out $head, not the pinned $sha"
  printf '%s' "$repo_dir"
}
