#!/usr/bin/env bash
## [CLONE-BUCKETS-THRESHOLDS] Score one dial setting against the corpus register.
##
## Scans every repository of the last version comparison with one frozen
## binary and one dial setting, scores it against the same reports'
## baseline engine with the corpus scorer, and appends one mechanically
## written row to corpus/tuning-sweep.md. Nothing here types a figure: every
## number in the row is read from the scorer's JSON scorecard.
##
## Usage:   scripts/tuning-sweep.sh <label> <min-nodes> [key=value ...]
## Example: scripts/tuning-sweep.sh shape-085 30 nearly_identical_min_shape=0.85
## Example: scripts/tuning-sweep.sh baseline 30
##
## Keys are the four [tuning.routing] dials. A repository's own .deslop.toml is
## kept and the dials appended to a copy of it, so exclusions do not change.
## Inputs: .corpus/version-compare/reports/run.json (the baseline manifest, its
## last engine) and target/version-compare/release/deslop, frozen on first use
## into .corpus/tuning-sweep/bin so a later comparison cannot swap the engine.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPARE_DIR="$REPO_ROOT/.corpus/version-compare"
BASE_MANIFEST="$COMPARE_DIR/reports/run.json"
SWEEP_DIR="$REPO_ROOT/.corpus/tuning-sweep"
FROZEN_BIN="$SWEEP_DIR/bin/deslop"
SOURCE_BIN="$REPO_ROOT/target/version-compare/release/deslop"
SCORER="$REPO_ROOT/target/corpus-score/release/corpus-score"
RESULTS="$REPO_ROOT/corpus/tuning-sweep.md"
RUN_FLAGS=(--no-incremental --no-fail-over --no-color --embeddings off --notext --nohtml)
DIAL_KEYS=" nearly_identical_min_shape nearly_identical_min_content similar_min_content shape_only_max_content "

die() { echo "tuning-sweep: $*" >&2; exit 1; }
log() { echo "==> $*" >&2; }

# Copies the comparison build aside once, so every setting runs one engine.
freeze_binary() {
  [ -x "$FROZEN_BIN" ] && return
  [ -x "$SOURCE_BIN" ] || die "no comparison build at $SOURCE_BIN; run scripts/compare-versions.sh first"
  mkdir -p "$(dirname "$FROZEN_BIN")"
  cp "$SOURCE_BIN" "$FROZEN_BIN"
}

# Writes the [tuning.routing] block for the given key=value dials.
dial_block() {
  [ "$#" -eq 0 ] && return
  echo "[tuning.routing]"
  local pair
  for pair in "$@"; do
    case "$DIAL_KEYS" in *" ${pair%%=*} "*) ;; *) die "unknown dial: ${pair%%=*}" ;; esac
    echo "${pair%%=*} = ${pair#*=}"
  done
}

# Scans one repository with the frozen binary and the merged config.
scan_repo() {
  local name="$1" min_nodes="$2" out="$3" block="$4"
  local root="$COMPARE_DIR/repos/$name" config="$out/deslop.toml"
  mkdir -p "$out"
  rm -rf "$root/.deslop"
  { [ -f "$root/.deslop.toml" ] && cat "$root/.deslop.toml"; echo; echo "$block"; } > "$config"
  log "scanning $name"
  "$SCORER" measure --timing "$out/timing.json" --binary-sha "$BIN_SHA" \
    "$FROZEN_BIN" "$root" --output "$out/report" --config "$config" \
    --min-nodes "$min_nodes" "${RUN_FLAGS[@]}"
  rm -rf "$root/.deslop"
}

# Writes a two-engine manifest: the baseline's last engine, then this setting.
write_manifest() {
  local label="$1" dir="$2"
  node -e '
    const [base, label, dir] = process.argv.slice(1);
    const run = JSON.parse(require("fs").readFileSync(base, "utf8"));
    const engine = run.engines.at(-1);
    const targets = run.targets.map((t) => ({ ...t, runs: {
      [engine.id]: t.runs[engine.id],
      [label]: { report: `${dir}/${t.name}/report.json`, timing: `${dir}/${t.name}/timing.json` },
    } }));
    const tuned = { id: label, label: `tuned ${label}`, commit: engine.commit, version: null };
    process.stdout.write(JSON.stringify({ generated_at: new Date().toISOString(),
      engines: [engine, tuned], targets }, null, 2));
  ' "$BASE_MANIFEST" "$label" "$dir" > "$dir/run.json"
}

# Appends one row read from the scorecard JSON to the results table.
append_row() {
  local label="$1" min_nodes="$2" dials="$3" scorecard="$4"
  node -e '
    const fs = require("fs");
    const [results, label, minNodes, dials, card] = process.argv.slice(1);
    const s = JSON.parse(fs.readFileSync(card, "utf8"));
    const [base, tuned] = s.engines.map((e) => s.totals[e.id]);
    const cov = (t) => `${t.matched_in_coverage.covered_lines}/${t.matched_in_coverage.judged_lines}`;
    const delta = (a, b) => (b - a > 0 ? "+" : "") + (b - a);
    if (!fs.existsSync(results)) fs.writeFileSync(results, [
      "# Tuning dial sweep",
      "",
      "Written by `scripts/tuning-sweep.sh`; every figure is read from the corpus scorer.",
      "Each row scores one dial setting against the corpus register beside the baseline engine.",
      "",
      "| setting | min nodes | dials | correct / judged | Δ correct | FN | FP | matched IN coverage | scorecard |",
      "|---|---|---|---|---|---|---|---|---|",
      "",
    ].join("\n"));
    fs.appendFileSync(results, `| ${label} | ${minNodes} | ${dials || "defaults"} | ` +
      `${tuned.correct}/${tuned.judged} | ${delta(base.correct, tuned.correct)} | ` +
      `${tuned.false_negatives} (${delta(base.false_negatives, tuned.false_negatives)}) | ` +
      `${tuned.false_positives} (${delta(base.false_positives, tuned.false_positives)}) | ` +
      `${cov(tuned)} (base ${cov(base)}) | ${card.split("/").pop()} |\n`);
  ' "$RESULTS" "$label" "$min_nodes" "$dials" "$scorecard"
}

main() {
  [ "$#" -ge 2 ] || { grep '^## ' "${BASH_SOURCE[0]}" | cut -c4-; exit 2; }
  local label="$1" min_nodes="$2"; shift 2
  [[ "$label" =~ ^[a-z0-9-]+$ ]] || die "label must be lowercase letters, digits and dashes"
  [ -f "$BASE_MANIFEST" ] || die "no baseline manifest at $BASE_MANIFEST"
  [ -x "$SCORER" ] || die "no scorer at $SCORER"
  freeze_binary
  BIN_SHA="$(shasum -a 256 "$FROZEN_BIN" | awk '{print $1}')"
  local dir="$SWEEP_DIR/$label" block name card
  [ -e "$dir" ] && die "setting $label already ran: $dir"
  block="$(dial_block "$@")"
  for name in $(node -e 'JSON.parse(require("fs").readFileSync(process.argv[1],"utf8")).targets.forEach((t)=>console.log(t.name))' "$BASE_MANIFEST"); do
    scan_repo "$name" "$min_nodes" "$dir/$name" "$block"
  done
  write_manifest "$label" "$dir"
  card="$("$SCORER" score "$dir/run.json" --out "$dir" --name "tune-$label" --print-json-path)"
  append_row "$label" "$min_nodes" "$*" "$card"
  log "scorecard: ${card%.json}.md"
  log "results:   $RESULTS"
}

main "$@"
