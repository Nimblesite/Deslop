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
# [CORPUS-PIN] The one list of files beside the registers that hold settings,
# such as the judged pair record, rather than a register.
NOT_A_REPOSITORY="corpus/not-a-repository.json"
# Repositories queued for a first judging pass: scanned like a register, but
# with no verdicts yet, because a judge cannot rule on pairs nothing produced.
JUDGING_QUEUE="corpus/judging-queue.json"

# [CORPUS-HISTORY-SOURCE] The tags a release is cut under; any other tag names
# no version.
RELEASE_TAGS="v*"

log() { echo "==> $*" >&2; }
die() { echo "==> ERROR: $*" >&2; exit 1; }

short_sha() { printf '%.*s' "$SHORT_SHA_LENGTH" "$1"; }

# ---------------------------------------------------------------------------
# is-local: true off CI. A local run is held to the rules CI's fresh checkout
# and fresh build already keep ([CORPUS-HISTORY-SOURCE]).
is_local() { [ -z "${CI:-}" ]; }

# ---------------------------------------------------------------------------
# require-clean-tree: [CORPUS-HISTORY-SOURCE] refuse a run over uncommitted
# changes in the repository at $1. Every result is tied to a commit, and a
# commit does not name source nobody committed.
require_clean_tree() {
  local changes
  changes="$(git -C "$1" status --porcelain)" || die "cannot read the working tree at $1"
  [ -z "$changes" ] || die "uncommitted changes — every corpus run is tied to a commit, so \
commit or stash these first:
$changes"
}

# ---------------------------------------------------------------------------
# release-version: [CORPUS-HISTORY-SOURCE] the release tag pointing at commit
# $2 of the repository at $1, or nothing when the commit is not a release.
release_version() {
  git -C "$1" tag --points-at "$2" --list "$RELEASE_TAGS" | sort -V | tail -n 1
}

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
# registered-names: the file stem of every register under $1, sorted. A file
# corpus/not-a-repository.json names holds settings and is skipped, so the
# judged pair record is never read as a repository to scan.
registered_names() {
  node -e '
    const { readdirSync } = require("node:fs");
    const settings = new Set(require(process.argv[2]).files);
    const JSON_EXTENSION = ".json";
    readdirSync(process.argv[1])
      .filter((file) => file.endsWith(JSON_EXTENSION) && !settings.has(file))
      .sort()
      .forEach((file) => console.log(file.slice(0, -JSON_EXTENSION.length)));
  ' "$1/$REGISTER_DIR" "$1/$NOT_A_REPOSITORY"
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
    local listing
    listing="$(registered_names "$root")" || die "cannot list the registers under $root/$REGISTER_DIR"
    while IFS= read -r name; do [ -z "$name" ] || names+=("$name"); done <<<"$listing"
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
    ' "$register" || die "cannot read a target from $register"
  done
}

# ---------------------------------------------------------------------------
# judged-names: every register under $1 that judges at least one pair CLEARLY IN
# or CLEARLY OUT, sorted. A register holding only NOT CLEAR verdicts scores
# nothing, so scanning its repository would spend the time and measure nothing
# ([CORPUS-HISTORY]).
judged_names() {
  local listing name register
  listing="$(registered_names "$1")" || die "cannot list the registers under $1/$REGISTER_DIR"
  while IFS= read -r name; do
    [ -n "$name" ] || continue
    register="$1/$REGISTER_DIR/$name.json"
    node -e '
      const register = require(process.argv[1]);
      const judged = ["clearly_in", "clearly_out"]
        .reduce((total, verdict) => total + (register[verdict] ?? []).length, 0);
      if (judged > 0) console.log(process.argv[2]);
    ' "$register" "$name" || die "cannot read the register $register"
  done <<<"$listing"
}

# ---------------------------------------------------------------------------
# judged-targets: one `url#commit#language` spec per register judging at least
# one pair — the whole judged corpus, and nothing that has no judgments.
judged_targets() {
  local listing names=() name
  listing="$(judged_names "$1")" || die "cannot list the judged registers under $1/$REGISTER_DIR"
  while IFS= read -r name; do [ -z "$name" ] || names+=("$name"); done <<<"$listing"
  [ ${#names[@]} -gt 0 ] || die "no register under $1/$REGISTER_DIR judges any pair"
  register_targets "$1" "${names[@]}"
}

# ---------------------------------------------------------------------------
# require-judged: refuse every target in $2.. whose repository has no register
# under $1 judging a pair CLEARLY IN or CLEARLY OUT. A gate can only score what
# a judge ruled on, so scanning anything else measures nothing, and a gate that
# measured nothing must never pass ([CORPUS-SCORE-GATE-JUDGED]). A register is
# named after its repository in lower case, as the run manifest names it.
require_judged() {
  local root="$1"; shift
  local listing judged=() spec name
  listing="$(judged_names "$root")" || die "cannot list the judged registers under $root/$REGISTER_DIR"
  while IFS= read -r name; do [ -z "$name" ] || judged+=("$name"); done <<<"$listing"
  for spec in "$@"; do
    name="$(basename "${spec%%#*}" .git | tr '[:upper:]' '[:lower:]')"
    is_listed "$name" ${judged[@]+"${judged[@]}"} \
      || die "target '$spec' has no register under $root/$REGISTER_DIR judging any pair"
  done
}

# is-listed: true when $1 is one of $2..
is_listed() {
  local wanted="$1" item; shift
  for item in "$@"; do [ "$item" != "$wanted" ] || return 0; done
  return 1
}

# ---------------------------------------------------------------------------
# queued-targets: print one `url#commit#language` spec per repository waiting on
# its first judging pass. Without these a new repository could never enter the
# register at all: a workspace needs two reports, and a comparison would only
# ever scan repositories that already have a register. A queued repository that
# already holds a register (judged, but with no agreed CLEARLY IN yet) is
# scanned as that register, never a second time.
queued_targets() {
  local root="$1" queue="$1/$JUDGING_QUEUE"
  [ -f "$queue" ] || return 0
  node -e '
    const { existsSync } = require("node:fs");
    const queue = require(process.argv[1]);
    const registered = (name) => existsSync(`${process.argv[2]}/${name.toLowerCase()}.json`);
    for (const repository of queue.repositories.filter(({ name }) => !registered(name))) {
      for (const field of ["url", "sha", "language"]) {
        if (!repository[field]) throw new Error(`${repository.name} has no ${field}`);
      }
      process.stdout.write(`${repository.url}#${repository.sha}#${repository.language}\n`);
    }
  ' "$queue" "$root/$REGISTER_DIR"
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
