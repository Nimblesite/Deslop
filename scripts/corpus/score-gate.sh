#!/usr/bin/env bash
## score-gate: scan the register-backed target repositories with the CURRENT
## working tree's deslop and hold the result to the thresholds in
## `corpus/register/score-thresholds.json`.
##
## This is the accuracy gate. It answers one question — does this build report
## every CLEARLY IN pair and stay silent on every CLEARLY OUT pair the judges
## recorded — and fails when it does not. Cluster totals and duplication
## percentages are printed as description and gate nothing.
##
## Usage:   scripts/corpus/score-gate.sh [url#commit#language ...]
## Example: scripts/corpus/score-gate.sh
##
## With no targets it scans the default slice: the smallest repositories that
## carry a register, each pinned to the exact commit its register was judged at,
## which is what CI runs. Checkouts live under `.corpus/score-gate/`
## (git-ignored); the scorecard is written to `.corpus/score-gate/SCORE.md` and
## `score.json`.

set -euo pipefail

# [CORPUS-SCORE] The CI slice, named by register. Deliberately the smallest
# register-backed repositories in the corpus: every one of them clones and scans
# in seconds on a hosted runner, and every one carries judged pairs. The rest of
# the corpus is scored by `make compare`, which nobody waits on. Widening this
# list is a deliberate change, and it costs CI minutes on every push.
#
# [CORPUS-PIN] The url and the COMMIT are read from each register rather than
# repeated here: a register is judged at one commit, and a slice pinned anywhere
# else — a tag above all, which upstream can re-cut — would score the engine
# against source the judge never read. The scorer refuses that outright.
DEFAULT_SLICE=(click cobra axios)

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
# scan: run the engine over $1 under peak-RSS measurement, writing the report
# and the measured cost under $2.
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

main() {
  case "${1:-}" in -h | --help) usage; exit 0 ;; esac
  command -v cargo >/dev/null || die "cargo not on PATH"
  command -v node >/dev/null || die "node not on PATH"
  if [ $# -eq 0 ]; then
    local slice=()
    while IFS= read -r spec; do slice+=("$spec"); done \
      < <(register_targets "$REPO_ROOT" "${DEFAULT_SLICE[@]}")
    set -- "${slice[@]}"
  fi
  parse_targets "$@"
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
  "$SCORER" score "$REPORTS_ROOT/run.json" --out "$WORK_DIR" --gate
}

main "$@"
