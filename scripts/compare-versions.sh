#!/usr/bin/env bash
## compare-versions: scan the SAME target repositories with deslop built at two
## different commits, one full cycle per commit, so two reports of the same
## repository can only differ because the engines differ. Per cycle, in order:
##
##   clean all build artifacts → fresh-extract the commit's source →
##   clean rebuild → for each target: delete its deslop cache, scan, report
##
## Only after BOTH cycles does it produce the comparison documents. Each run
## records the sha256 of the exact binary that executed; the script refuses to
## compare if both cycles produced the same binary (the build-isolation
## regression this guard exists for). Every document it writes is stamped with
## both deslop commit ids in full and the target repository's exact commit.
##
## Every scan runs under peak-RSS measurement, so the scorecard reports wall
## time, peak memory and CPU seconds beside the accuracy figures. The scorer is
## built from the WORKING TREE, never from either compared commit: two engines
## must be scored by one identical scorer ([CORPUS-SCORE]). The run ends with
## SCORE.md, and exits non-zero when a target breaches
## `corpus/register/score-thresholds.json` — the documents are written first, so
## a failing gate still leaves a readable report.
##
## Usage:   scripts/compare-versions.sh [commit-a commit-b] [url#commit#language ...]
## Example: scripts/compare-versions.sh
## Example: scripts/compare-versions.sh b5273c163 e8a215e99
## Example: scripts/compare-versions.sh b5273c163 e8a215e99 \
##            https://github.com/pallets/click.git#8b19813f2bfca99f1018a587a8cf54fc959f2e5d#Python
##
## With no commits it compares **the last tagged release against the current
## HEAD**, which is the question this script exists to answer: has anything
## merged since the release changed what the detector reports? The tag only
## selects a commit; every document below records the resolved commit id.
##
## [CORPUS-PIN] A target is `url#<commit-id>#language`, and the commit id is the
## full 40-character git object name — never a tag, never a branch, never a
## version. With no targets at all, every judged clone register is scanned, each
## at the exact commit its judge read. The deslop source for each commit is
## extracted with `git archive` from the local repository — the working tree and
## branches are never touched. Checkouts and reports live under
## `.corpus/version-compare/` (git-ignored, same policy as the corpus clone
## cache); build artifacts under `target/` and wiped before every build.

set -euo pipefail

# [COMPARE-VERSIONS-CONSTANTS] Default behaviour, overridable by environment;
# positional arguments take precedence over both. With COMPARE_TARGET unset the
# targets are every judged clone register, so the default run is the one that
# produces a score rather than a description.
COMPARE_TARGET="${COMPARE_TARGET:-}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="${COMPARE_WORK_DIR:-$REPO_ROOT/.corpus/version-compare}"
# shellcheck source=scripts/corpus/target-repos.sh
source "$REPO_ROOT/scripts/corpus/target-repos.sh"
REPORTS_ROOT="$WORK_DIR/reports"
TARGET_DIR="$REPO_ROOT/target/version-compare"
SUMMARY_RENDERER="$REPO_ROOT/scripts/compare-versions-summary.mjs"
# [CORPUS-SCORE] The scorer is built from the WORKING TREE, never from either
# compared commit: two engines must be scored by one identical scorer, and a
# scorer that ships inside the thing it measures cannot do that.
SCORER_TARGET_DIR="$REPO_ROOT/target/corpus-score"
SCORER="$SCORER_TARGET_DIR/release/corpus-score"

# The one analysis config every run uses. --no-incremental keeps the on-disk
# fingerprint cache out of the picture; the cache directory is also deleted
# outright before each scan.
RUN_FLAGS=(
  --no-incremental
  --no-fail-over
  --no-color
  --embeddings off
)

usage() {
  grep '^## ' "${BASH_SOURCE[0]}" | cut -c4-
}

# ---------------------------------------------------------------------------
# resolve-commit: print the full sha of $1, failing if it is not a commit in
# the local deslop repository.
resolve_commit() {
  git -C "$REPO_ROOT" rev-parse --verify --quiet "${1}^{commit}" ||
    die "commit '$1' not found in $REPO_ROOT (fetch it first)"
}

# ---------------------------------------------------------------------------
# last-release-commit: the commit the most recent release tag points at. The
# tag is only a way to choose a commit; from here on the run knows the commit
# and nothing else, so a re-cut tag can never move a comparison after the fact.
last_release_commit() {
  local tag
  tag="$(git -C "$REPO_ROOT" describe --tags --abbrev=0 2>/dev/null)" ||
    die "no release tag in $REPO_ROOT — name both commits explicitly"
  log "baseline: last release $tag"
  resolve_commit "$tag"
}

# ---------------------------------------------------------------------------
# build-scorer: compile the working tree's corpus-score into its own target dir
# so the per-cycle `rm -rf $TARGET_DIR` never wipes it mid-run.
build_scorer() {
  log "building the scorer from the working tree"
  (cd "$REPO_ROOT" && CARGO_TARGET_DIR="$SCORER_TARGET_DIR" \
    cargo build --release -p deslop-test-support --bin corpus-score >/dev/null)
  [ -x "$SCORER" ] || die "scorer missing at $SCORER"
}

# ---------------------------------------------------------------------------
# extract-source: fresh-extract $1 (commit) via git archive into its own tree.
# rm -rf first, so the tree always matches the commit exactly.
extract_source() {
  local src_dir="$WORK_DIR/src/$(short_sha "$1")"
  log "extracting deslop source at $(short_sha "$1")"
  rm -rf "$src_dir"
  mkdir -p "$src_dir"
  git -C "$REPO_ROOT" archive "$1" | tar -x -C "$src_dir"
  printf '%s' "$src_dir"
}

# ---------------------------------------------------------------------------
# compile: clean-rebuild the release CLI from $1 (source tree) into the
# just-wiped target dir and print the binary path.
compile() {
  log "rebuilding release CLI at $THIS_SHORT from clean"
  (cd "$1" && CARGO_TARGET_DIR="$TARGET_DIR" cargo build --release --bin deslop)
  printf '%s' "$TARGET_DIR/release/deslop"
}

# ---------------------------------------------------------------------------
# scan: run $2 (the freshly built binary) over $1 (a target checkout), writing
# reports under $3. Deletes that checkout's deslop cache first, and records
# wall time plus the binary's sha256 as timing.json.
scan() {
  local scan_root="$1" binary="$2" report_dir="$3"
  rm -rf "$scan_root/.deslop" "$report_dir"
  mkdir -p "$report_dir"
  log "scanning $(basename "$scan_root") with deslop@$THIS_SHORT (sha ${THIS_BINARY_SHA:0:16})"
  "$SCORER" measure \
    --timing "$report_dir/timing.json" \
    --binary-sha "$THIS_BINARY_SHA" \
    "$binary" "$scan_root" --output "$report_dir/report" "${RUN_FLAGS[@]}"
}

# ---------------------------------------------------------------------------
# run-cycle: the full per-commit cycle — clean artifacts, fresh extract, clean
# rebuild, then scan every target with that one binary. Sets THIS_SHORT and
# THIS_BINARY_SHA for the caller.
run_cycle() {
  local commit="$1" src_dir binary index
  THIS_SHORT="$(short_sha "$commit")"
  log "=== cycle deslop@$THIS_SHORT: clean → rebuild → scan ${#TARGET_URLS[@]} target(s)"
  log "cleaning build artifacts ($TARGET_DIR)"
  rm -rf "$TARGET_DIR"
  src_dir="$(extract_source "$commit")"
  binary="$(compile "$src_dir")"
  THIS_BINARY_SHA="$(shasum -a 256 "$binary" | awk '{print $1}')"
  log "binary sha256: $THIS_BINARY_SHA"
  for index in "${!TARGET_URLS[@]}"; do
    scan "${TARGET_DIRS[$index]}" "$binary" \
      "$REPORTS_ROOT/${TARGET_SLUGS[$index]}/$THIS_SHORT"
  done
}

# ---------------------------------------------------------------------------
# write-meta: record everything the renderer needs — both deslop commits in
# full, and every target's url, pinned commit and language.
write_meta() {
  local meta_path="$1" commit_a="$2" commit_b="$3" index
  local fields=()
  for index in "${!TARGET_URLS[@]}"; do
    fields+=("${TARGET_SLUGS[$index]}" "${TARGET_URLS[$index]}" "${TARGET_SHAS[$index]}" \
      "${TARGET_LANGS[$index]}")
  done
  node -e '
    const FIELDS_PER_TARGET = 4;
    const SHORT = Number(process.env.SHORT_SHA_LENGTH);
    const [metaPath, shaA, subjectA, shaB, subjectB, flags, reportsRoot, indexPath,
      ...rest] = process.argv.slice(1);
    const targets = [];
    for (let at = 0; at < rest.length; at += FIELDS_PER_TARGET) {
      const [slug, url, sha, language] = rest.slice(at, at + FIELDS_PER_TARGET);
      const dir = (commit) => `${reportsRoot}/${slug}/${commit.slice(0, SHORT)}`;
      targets.push({
        slug, url, sha, language,
        reports: { a: `${dir(shaA)}/report.json`, b: `${dir(shaB)}/report.json` },
        timings: { a: `${dir(shaA)}/timing.json`, b: `${dir(shaB)}/timing.json` },
        summary_path: `${reportsRoot}/${slug}/SUMMARY.md`,
      });
    }
    const generated_at = new Date().toISOString();
    const fs = require("fs");
    fs.writeFileSync(metaPath, JSON.stringify({
      deslop: {
        a: { sha: shaA, subject: subjectA },
        b: { sha: shaB, subject: subjectB },
      },
      flags,
      generated_at,
      index_path: indexPath,
      targets,
    }, null, 2) + "\n");
    // [CORPUS-SCORE] The engine-agnostic run manifest the scorer reads. One
    // shape whether one engine ran or two, so the CI gate and this comparison
    // are scored by the same code path.
    const engineId = (sha) => sha.slice(0, SHORT);
    fs.writeFileSync(metaPath.replace(/meta\.json$/, "run.json"), JSON.stringify({
      generated_at,
      engines: [
        { id: engineId(shaA), label: `deslop@${engineId(shaA)}` },
        { id: engineId(shaB), label: `deslop@${engineId(shaB)}` },
      ],
      targets: targets.map((target) => ({
        name: target.slug,
        language: target.language,
        sha: target.sha,
        register: `corpus/register/${target.slug.toLowerCase()}.json`,
        runs: {
          [engineId(shaA)]: { report: target.reports.a, timing: target.timings.a },
          [engineId(shaB)]: { report: target.reports.b, timing: target.timings.b },
        },
      })),
    }, null, 2) + "\n");
  ' "$meta_path" \
    "$commit_a" "$(git -C "$REPO_ROOT" log -1 --format=%s "$commit_a")" \
    "$commit_b" "$(git -C "$REPO_ROOT" log -1 --format=%s "$commit_b")" \
    "${RUN_FLAGS[*]}" "$REPORTS_ROOT" "$REPORTS_ROOT/INDEX.md" \
    "${fields[@]}"
}

# ---------------------------------------------------------------------------
# is-commitish: true when $1 names a commit in the local deslop repository. A
# target spec never does, which is what separates the two argument forms.
is_commitish() {
  git -C "$REPO_ROOT" rev-parse --verify --quiet "${1}^{commit}" >/dev/null 2>&1
}

# ---------------------------------------------------------------------------
# read-commits: set COMMIT_A and COMMIT_B from the first two arguments, or from
# the last release and HEAD when none were given, and record how many arguments
# it consumed.
read_commits() {
  if is_commitish "${1:-}"; then
    is_commitish "${2:-}" || die "'$1' names a commit but '${2:-nothing}' does not — \
name both sides, or neither and compare the last release against HEAD"
    COMMIT_A="$(resolve_commit "$1")"
    COMMIT_B="$(resolve_commit "$2")"
    CONSUMED=2
  else
    COMMIT_A="$(last_release_commit)"
    COMMIT_B="$(resolve_commit HEAD)"
    CONSUMED=0
  fi
  [ "$COMMIT_A" != "$COMMIT_B" ] ||
    die "both sides resolve to $COMMIT_A — there is nothing to compare. HEAD is the \
last release; compare two named commits instead"
}

main() {
  case "${1:-}" in -h | --help) usage; exit 0 ;; esac
  command -v cargo >/dev/null || die "cargo not on PATH"
  command -v node >/dev/null || die "node not on PATH (build.rs regenerates the wire models)"
  command -v shasum >/dev/null || die "shasum not on PATH (each run fingerprints its binary)"
  local commit_a commit_b index sha_a sha_b
  read_commits "$@"
  commit_a="$COMMIT_A"; commit_b="$COMMIT_B"
  shift "$CONSUMED"
  if [ $# -eq 0 ]; then
    local targets=()
    if [ -n "$COMPARE_TARGET" ]; then
      targets=("$COMPARE_TARGET")
    else
      while IFS= read -r spec; do targets+=("$spec"); done < <(default_targets "$REPO_ROOT")
    fi
    set -- "${targets[@]}"
  fi
  parse_targets "$@"

  build_scorer

  TARGET_DIRS=()
  for index in "${!TARGET_URLS[@]}"; do
    TARGET_DIRS+=("$(prepare_target_repo "${TARGET_URLS[$index]}" "${TARGET_SHAS[$index]}")")
  done

  run_cycle "$commit_a"; sha_a="$THIS_BINARY_SHA"
  run_cycle "$commit_b"; sha_b="$THIS_BINARY_SHA"

  # Build-isolation guard: two different commits must not produce the same
  # binary. If they do, one cycle scanned a stale or duplicated engine and
  # every comparison below it would be fiction.
  [ "$sha_a" != "$sha_b" ] ||
    die "both cycles produced the identical binary (sha $sha_a) — builds are not isolated"

  mkdir -p "$REPORTS_ROOT"
  write_meta "$REPORTS_ROOT/meta.json" "$commit_a" "$commit_b"
  # [CORPUS-SCORE] Score first: the summary renderer reads score.json rather
  # than recomputing anything, so the two documents cannot disagree.
  local gate_status=0
  "$SCORER" score "$REPORTS_ROOT/run.json" --out "$REPORTS_ROOT" --gate >/dev/null ||
    gate_status=$?
  echo
  node "$SUMMARY_RENDERER" "$REPORTS_ROOT/meta.json"
  echo
  log "index:      $REPORTS_ROOT/INDEX.md"
  log "scorecard:  $REPORTS_ROOT/SCORE.md"
  for index in "${!TARGET_SLUGS[@]}"; do
    log "summary:    $REPORTS_ROOT/${TARGET_SLUGS[$index]}/SUMMARY.md"
  done
  [ "$gate_status" -eq 0 ] || die "the corpus score gate failed — see $REPORTS_ROOT/SCORE.md"
}

main "$@"
