#!/usr/bin/env bash
## score-gate: scan the register-backed target repositories with the CURRENT
## working tree's deslop and hold every judged pair to its recorded outcome in
## `corpus/register/outcomes.json`, writing the record back
## ([CORPUS-SCORE-OUTCOMES]).
##
## This is the accuracy gate. It answers one question — does this build report
## every CLEARLY IN pair and stay silent on every CLEARLY OUT pair the judges
## recorded — and fails when a pair regresses or fails without being on record
## as failing. Cluster totals and duplication percentages are printed as
## description and gate nothing.
##
## Usage:   scripts/corpus/score-gate.sh [url#commit#language ...]
##          scripts/corpus/score-gate.sh --all-judged
## Example: scripts/corpus/score-gate.sh
## Inspect: scripts/corpus/score-gate.sh --list-default-targets
##          scripts/corpus/score-gate.sh --list-judged-targets
##
## `--all-judged` scans every register that judges at least one pair — the
## whole judged corpus, and no repository without judgments — and holds it to
## the run history in `corpus/history/corpus-runs.jsonl` as well as the record:
## one judged pair that passed in any earlier run of the same commit and fails
## now fails the run. The run is then appended to the history with every judged
## pair's outcome, each repository's cost, the machine and the source commit
## ([CORPUS-HISTORY]). Its scorecard is `score-corpus-<UTC timestamp>.md`.
##
## With no targets it scans the default slice: judged repositories across the
## core languages, each pinned to the exact commit its register was judged at,
## which is what CI runs. Checkouts live under `.corpus/score-gate/`
## (git-ignored); every run writes its own scorecard,
## `.corpus/score-gate/score-gate-<UTC timestamp>.md` and `.json`, so no run
## overwrites another ([CORPUS-REPORT]).

set -euo pipefail

# [CORPUS-SCORE-GATE-SLICE] The CI slice, named by register. Each repository carries a
# substantive judged body of pairs. The rest of the corpus is scored by
# `make compare`. Widening this list is deliberate and costs CI time.
#
# [CORPUS-PIN] The url and the COMMIT are read from each register rather than
# repeated here: a register is judged at one commit, and a slice pinned anywhere
# else — a tag above all, which upstream can re-cut — would score the engine
# against source the judge never read. The scorer refuses that outright.
DEFAULT_SLICE=(click cobra axios deslop polly fsharp.data)

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK_DIR="${SCORE_GATE_WORK_DIR:-$REPO_ROOT/.corpus/score-gate}"
# shellcheck source=scripts/corpus/target-repos.sh
source "$REPO_ROOT/scripts/corpus/target-repos.sh"

BINARY="$REPO_ROOT/target/release/deslop"
SCORER="$REPO_ROOT/target/release/corpus-score"
# The same analysis config the version comparison uses, so a figure measured
# here and one measured there are comparable.
RUN_FLAGS=(--no-incremental --no-fail-over --no-color --embeddings off)

usage() { grep '^## ' "${BASH_SOURCE[0]}" | cut -c4-; }

# ---------------------------------------------------------------------------
# build: compile the engine and the scorer from the working tree.
build() {
  log "building deslop and the scorer from the working tree"
  (cd "$REPO_ROOT" && cargo build --release --bin deslop)
  (cd "$REPO_ROOT" && cargo build --release -p deslop-test-support --bin corpus-score)
  [ -x "$BINARY" ] || die "engine missing at $BINARY"
  [ -x "$SCORER" ] || die "scorer missing at $SCORER"
}

# ---------------------------------------------------------------------------
# scan: run the engine over $1 under the corpus measurement ([CORPUS-MEASURE]),
# writing the report and its wall time, CPU time, peak CPU and peak memory
# under $2.
scan() {
  local scan_root="$1" report_dir="$2"
  rm -rf "$scan_root/.deslop" "$report_dir"
  mkdir -p "$report_dir"
  log "scanning $(basename "$scan_root")"
  "$SCORER" measure \
    --timing "$report_dir/timing.json" \
    --binary-sha "$ENGINE_SHA" \
    "$BINARY" "$scan_root" --output "$report_dir/report" "${RUN_FLAGS[@]}"
}

# ---------------------------------------------------------------------------
# write-run: the engine-agnostic run manifest the scorer reads. One engine
# here, two in a version comparison, one shape either way.
write_run() {
  local run_path="$1" index
  local fields=()
  for index in "${!TARGET_SLUGS[@]}"; do
    fields+=("${TARGET_SLUGS[$index]}" "${TARGET_LANGS[$index]}" "${TARGET_SHAS[$index]}" \
      "$REPORTS_ROOT/${TARGET_SLUGS[$index]}")
  done
  node -e '
    const FIELDS_PER_TARGET = 4;
    const [runPath, engineId, engineLabel, ...rest] = process.argv.slice(1);
    const targets = [];
    for (let at = 0; at < rest.length; at += FIELDS_PER_TARGET) {
      const [name, language, sha, dir] = rest.slice(at, at + FIELDS_PER_TARGET);
      targets.push({
        name, language, sha,
        register: `corpus/register/${name.toLowerCase()}.json`,
        runs: { [engineId]: { report: `${dir}/report.json`, timing: `${dir}/timing.json` } },
      });
    }
    require("fs").writeFileSync(runPath, JSON.stringify({
      generated_at: new Date().toISOString(),
      engines: [{ id: engineId, label: engineLabel }],
      targets,
    }, null, 2) + "\n");
  ' "$run_path" "$ENGINE_ID" "$ENGINE_LABEL" "${fields[@]}"
}

# ---------------------------------------------------------------------------
# source-flags: [CORPUS-HISTORY] the scorer flags naming the commit this build
# came from, and whether uncommitted changes were built with it.
source_flags() {
  local commit
  commit="$(git -C "$REPO_ROOT" rev-parse HEAD)" || die "cannot read the source commit"
  SCORE_FLAGS+=(--history-commit "$commit")
  [ -z "$(git -C "$REPO_ROOT" status --porcelain)" ] || SCORE_FLAGS+=(--history-dirty)
}

# ---------------------------------------------------------------------------
# targets: the specs main scans — the arguments, every judged register under
# --all-judged, or the default slice.
targets() {
  case "${1:-}" in
    --all-judged) judged_targets "$REPO_ROOT" ;;
    "") register_targets "$REPO_ROOT" "${DEFAULT_SLICE[@]}" ;;
    *) printf '%s\n' "$@" ;;
  esac
}

main() {
  case "${1:-}" in
    -h | --help) usage; exit 0 ;;
    --list-default-targets) register_targets "$REPO_ROOT" "${DEFAULT_SLICE[@]}"; exit 0 ;;
    --list-judged-targets) judged_targets "$REPO_ROOT"; exit 0 ;;
  esac
  command -v cargo >/dev/null || die "cargo not on PATH"
  command -v node >/dev/null || die "node not on PATH"
  RUN_NAME=score-gate
  SCORE_FLAGS=(--gate)
  if [ "${1:-}" = "--all-judged" ]; then
    RUN_NAME=score-corpus
    source_flags
  fi
  local specs=() spec
  while IFS= read -r spec; do specs+=("$spec"); done < <(targets "$@")
  parse_targets "${specs[@]}"
  build

  ENGINE_SHA="$(shasum -a 256 "$BINARY" | awk '{print $1}')"
  ENGINE_ID="$(short_sha "$ENGINE_SHA")"
  ENGINE_LABEL="deslop (binary ${ENGINE_ID})"
  REPORTS_ROOT="$WORK_DIR/reports"
  log "engine binary sha256: $ENGINE_SHA"

  local index repo_dir
  for index in "${!TARGET_URLS[@]}"; do
    repo_dir="$(prepare_target_repo "${TARGET_URLS[$index]}" "${TARGET_SHAS[$index]}")"
    scan "$repo_dir" "$REPORTS_ROOT/${TARGET_SLUGS[$index]}"
  done

  mkdir -p "$REPORTS_ROOT"
  write_run "$REPORTS_ROOT/run.json"
  echo
  "$SCORER" score "$REPORTS_ROOT/run.json" --out "$WORK_DIR" --name "$RUN_NAME" "${SCORE_FLAGS[@]}"
}

main "$@"
