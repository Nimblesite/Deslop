#!/usr/bin/env bash
## compare-versions: scan the SAME target repository with deslop built at two
## different commits, one full cycle per commit, so the two reports can only
## differ because the engines differ. Per cycle, in order:
##
##   clean all build artifacts → delete all deslop cache →
##   fresh-extract the commit's source → clean rebuild →
##   run the analysis → produce that run's report
##
## Only after BOTH cycles does it produce the comparison summary. Each run
## records the sha256 of the exact binary that executed; the script refuses
## to compare if both cycles produced the same binary (the build-isolation
## regression this guard exists for).
##
## Usage:   scripts/compare-versions.sh <commit-a> <commit-b> [repo-url] [repo-tag]
## Example: scripts/compare-versions.sh f92300e5e1004ef6c53a94174a0d7e842232ec80 \
##                                       b5273c16351cf2dd0ec7c0a946c8122289e095b3
## Example: scripts/compare-versions.sh f92300e5 b5273c16 \
##                                       https://github.com/tornadoweb/tornado.git v6.5.8
##
## With no repo-url, the default target (ripgrep, pinned) is scanned. With a
## repo-url but no repo-tag, the remote default branch HEAD is analysed and
## its resolved sha recorded in the summary, so the run stays reproducible
## even if upstream moves. The deslop source for each commit is extracted
## with `git archive` from the local repository — the working tree and
## branches are never touched. Clones and reports live under
## `.corpus/version-compare/` (git-ignored, same policy as the corpus clone
## cache); build artifacts under `target/` and wiped before every build.

set -euo pipefail

# [COMPARE-VERSIONS-CONSTANTS] Default target and behaviour, overridable by
# environment; positional arguments take precedence over both.
DEFAULT_REPO_URL="${COMPARE_REPO_URL:-https://github.com/BurntSushi/ripgrep.git}"
DEFAULT_REPO_TAG="${COMPARE_REPO_TAG:-14.1.1}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="${COMPARE_WORK_DIR:-$REPO_ROOT/.corpus/version-compare}"
TARGET_DIR="$REPO_ROOT/target/version-compare"

# The one analysis config both runs use. --no-incremental keeps the on-disk
# fingerprint cache out of the picture; the cache directory is also deleted
# outright before each run.
RUN_FLAGS=(
  --no-incremental
  --no-fail-over
  --no-color
  --embeddings off
)

log() { echo "==> $*" >&2; }
die() { echo "==> ERROR: $*" >&2; exit 1; }

usage() {
  grep '^## ' "${BASH_SOURCE[0]}" | cut -c4-
}

# Millisecond wall clock; node is already a hard dependency (build.rs).
now_ms() { node -e 'process.stdout.write(String(Date.now()))'; }

# ---------------------------------------------------------------------------
# resolve-commit: print the full sha of $1, failing if it is not a commit in
# the local deslop repository.
resolve_commit() {
  local commit="$1"
  git -C "$REPO_ROOT" rev-parse --verify --quiet "${commit}^{commit}" ||
    die "commit '$commit' not found in $REPO_ROOT (fetch it first)"
}

# ---------------------------------------------------------------------------
# repo-slug: filesystem-safe name for a repo url, used to namespace clones,
# reports, and summaries per target repository.
repo_slug() {
  basename "$1" .git
}

# ---------------------------------------------------------------------------
# resolve-remote-sha: print the commit the remote actually serves for $1
# (url) + $2 (tag, empty = default branch HEAD). Prefers the peeled entry,
# so annotated tags resolve to their commit, never the tag object.
resolve_remote_sha() {
  local url="$1" tag="$2"
  if [ -n "$tag" ]; then
    git ls-remote "$url" "refs/tags/$tag" "refs/tags/$tag^{}" | tail -n1 | cut -f1
  else
    git ls-remote "$url" HEAD | cut -f1
  fi
}

# ---------------------------------------------------------------------------
# prepare-target-repo: clone the target once and verify its HEAD against the
# sha the remote served when the run started, exactly like the corpus
# manifests. A moved or re-cut tag fails loudly instead of silently
# re-baselining the comparison. Sets REPO_DIR.
prepare_target_repo() {
  local url="$1" tag="$2" sha="$3"
  REPO_DIR="$WORK_DIR/repos/$(repo_slug "$url")"
  if [ -d "$REPO_DIR/.git" ]; then
    local head
    head="$(git -C "$REPO_DIR" rev-parse HEAD)"
    if [ "$head" = "$sha" ]; then
      log "target repo already at $sha, reusing"
      return
    fi
    log "target repo at $head, remote served $sha — re-cloning"
    rm -rf "$REPO_DIR"
  fi
  mkdir -p "$(dirname "$REPO_DIR")"
  if [ -n "$tag" ]; then
    log "cloning $url at tag $tag (commit $sha)"
    git clone --quiet --depth 1 --branch "$tag" "$url" "$REPO_DIR"
  else
    log "cloning $url default branch (commit $sha)"
    git clone --quiet --depth 1 "$url" "$REPO_DIR"
  fi
  local head
  head="$(git -C "$REPO_DIR" rev-parse HEAD)"
  [ "$head" = "$sha" ] ||
    die "clone resolved to $head, remote served $sha for tag '${tag:-HEAD}'"
}

# ---------------------------------------------------------------------------
# clean-build-artifacts: wipe the entire cargo target dir so every build
# compiles from scratch — nothing can leak from the other commit's build.
clean_build_artifacts() {
  log "cleaning build artifacts ($TARGET_DIR)"
  rm -rf "$TARGET_DIR"
}

# ---------------------------------------------------------------------------
# delete-deslop-cache: remove every deslop cache under the scan root so the
# analysis cannot read state from a previous run.
delete_deslop_cache() {
  local cache_dir="$REPO_DIR/.deslop"
  if [ -d "$cache_dir" ]; then
    log "deleting deslop cache ($cache_dir)"
    rm -rf "$cache_dir"
  fi
}

# ---------------------------------------------------------------------------
# extract-source: fresh-extract $1 (commit) via git archive into its own
# tree. rm -rf first, so the tree always matches the commit exactly.
extract_source() {
  local commit="$1"
  local short src_dir
  short="$(printf '%.12s' "$commit")"
  src_dir="$WORK_DIR/src/$short"
  log "extracting deslop source at $short"
  rm -rf "$src_dir"
  mkdir -p "$src_dir"
  git -C "$REPO_ROOT" archive "$commit" | tar -x -C "$src_dir"
  printf '%s' "$src_dir"
}

# ---------------------------------------------------------------------------
# compile: clean-rebuild the release CLI from $1 (source tree) into the
# just-wiped target dir and print the binary path.
compile() {
  local src_dir="$1"
  log "rebuilding release CLI at $THIS_SHORT from clean"
  (cd "$src_dir" && CARGO_TARGET_DIR="$TARGET_DIR" cargo build --release --bin deslop)
  printf '%s' "$TARGET_DIR/release/deslop"
}

# ---------------------------------------------------------------------------
# run-cycle: the full per-commit cycle — clean artifacts, delete caches,
# fresh extract, clean rebuild, scan, report. Sets THIS_SHORT,
# THIS_BINARY_SHA and THIS_REPORT_DIR for the caller.
run_cycle() {
  local commit="$1"
  THIS_SHORT="$(printf '%.12s' "$commit")"
  THIS_REPORT_DIR="$REPORTS_BASE/$THIS_SHORT"
  log "=== cycle deslop@$THIS_SHORT: clean → rebuild → scan"
  clean_build_artifacts
  delete_deslop_cache
  local src_dir binary
  src_dir="$(extract_source "$commit")"
  binary="$(compile "$src_dir")"
  THIS_BINARY_SHA="$(shasum -a 256 "$binary" | awk '{print $1}')"
  log "binary sha256: $THIS_BINARY_SHA"
  rm -rf "$THIS_REPORT_DIR"
  scan "$REPO_DIR" "$binary" "$THIS_REPORT_DIR"
}

# ---------------------------------------------------------------------------
# scan: run the freshly built $2 (binary) over $1 (the target repo), writing
# reports under $3. Records wall time and the binary's sha256 as timing.json.
scan() {
  local scan_root="$1" binary="$2" report_dir="$3"
  local started_ms ended_ms
  mkdir -p "$report_dir"
  log "scanning with deslop@$THIS_SHORT (sha ${THIS_BINARY_SHA:0:16})"
  started_ms="$(now_ms)"
  "$binary" "$scan_root" --output "$report_dir/report" "${RUN_FLAGS[@]}"
  ended_ms="$(now_ms)"
  node -e '
    const [path, started, ended, binarySha] = process.argv.slice(1);
    require("fs").writeFileSync(path, JSON.stringify({
      started_at_epoch_ms: Number(started),
      ended_at_epoch_ms: Number(ended),
      elapsed_ms: Number(ended) - Number(started),
      binary_sha256: binarySha,
    }, null, 2) + "\n");
  ' "$report_dir/timing.json" "$started_ms" "$ended_ms" "$THIS_BINARY_SHA"
}

# ---------------------------------------------------------------------------
# print-summary: write SUMMARY.md beside the reports and echo it to stdout.
# Every figure is lifted verbatim from the engine's own JSON — this script
# computes nothing except the wall-time differences already recorded in each
# timing.json.
print_summary() {
  local summary_path="$1" short_a="$2" subject_a="$3" short_b="$4" subject_b="$5"
  local target_url="$6" target_tag="$7" target_sha="$8" flags="$9"
  local report_a_path="${10}" report_b_path="${11}"
  node -e '
    const fs = require("fs");
    const readJson = (p) => JSON.parse(fs.readFileSync(p, "utf8"));
    const [summaryPath, shortA, subjectA, shortB, subjectB, url, tag, sha, flags,
      reportAPath, reportBPath] = process.argv.slice(1);
    const meta = { shortA, subjectA, shortB, subjectB, url, tag, sha, flags };
    const reportA = readJson(reportAPath);
    const reportB = readJson(reportBPath);
    const timingA = readJson(reportAPath.replace(/report\.json$/, "timing.json"));
    const timingB = readJson(reportBPath.replace(/report\.json$/, "timing.json"));
    const stat = (r, t) => [
      ["binary_sha256", t.binary_sha256],
      ["tool_version", r.tool_version],
      ["files_analysed", r.files_analysed],
      ["analysed_loc", r.metrics.analysed_loc],
      ["duplicated_loc", r.metrics.duplicated_loc],
      ["duplication_percent (engine)", r.metrics.duplication_percent],
      ["clusters_total", r.metrics.clusters_total],
      ["clusters_hidden", r.clusters_hidden ?? "n/a"],
      ["analysis_wall_ms (cold, no cache)", t.elapsed_ms],
    ];
    const statsA = stat(reportA, timingA);
    const statsB = stat(reportB, timingB);
    // Only fields present in BOTH reports are comparable across versions —
    // anything the schema dropped or gained between commits is struck.
    const statRows = statsA
      .map(([label, a], i) => [label, a, statsB[i][1]])
      .filter(([, a, b]) => a !== undefined && b !== undefined);
    const struckStats = statsA
      .map(([label, a], i) => [label, a, statsB[i][1]])
      .filter(([, a, b]) => a === undefined || b === undefined)
      .map(([label]) => label);
    // Cluster overlap by id: the real accuracy-movement signal between the
    // two engines — how many published clusters both report, and how many
    // each reports that the other does not.
    const idsOf = (r) => new Set((r.clusters ?? []).map((c) => c.id));
    const idsA = idsOf(reportA);
    const idsB = idsOf(reportB);
    const sharedCount = [...idsA].filter((x) => idsB.has(x)).length;
    const onlyACount = idsA.size - sharedCount;
    const onlyBCount = idsB.size - sharedCount;
    // Per-version cluster columns: each version is rendered with the fields
    // its own schema reports, so no cell is ever an approximation of another
    // schema field. Column shown when any cluster of that report has it.
    const CLUSTER_DISPLAY = [
      ["id", "id", (c) => c.id],
      ["rank", "rank", (c) => c.rank],
      ["rank_band", "band", (c) => c.rank_band],
      ["mass", "mass", (c) => c.mass],
      ["weight", "weight", (c) => c.weight],
      ["size", "size", (c) => c.size],
      ["bucket", "bucket", (c) => c.bucket],
      ["category", "category", (c) => c.category],
      ["canonical_node_count", "nodes", (c) => c.canonical_node_count],
      ["occurrence_count", "occurrences", (c) => c.occurrence_count],
      ["occurrences_total", "occurrences_total", (c) => c.occurrences_total],
      ["signals.fused", "fused", (c) => (c.signals ? c.signals.fused : undefined)],
    ];
    const columnsFor = (r) => CLUSTER_DISPLAY.filter(([, , get]) =>
      (r.clusters ?? []).some((c) => get(c) !== undefined));
    const clusterTable = (r, cols) => (r.clusters ?? []).slice(0, 5).map((c) =>
      `| ${cols.map(([, , get]) => get(c)).join(" | ")} |`).join("\n");
    const colsA = columnsFor(reportA);
    const colsB = columnsFor(reportB);
    // Schema delta: what each report shape has that the other lacks, at both
    // top level and per cluster.
    const onlyInTop = (a, b) => Object.keys(a).filter((k) => !(k in b));
    const clusterKeys = (r) => [...new Set(r.clusters.flatMap((c) => Object.keys(c)))];
    const keysA = clusterKeys(reportA);
    const keysB = clusterKeys(reportB);
    const onlyInACluster = keysA.filter((k) => !keysB.includes(k));
    const onlyInBCluster = keysB.filter((k) => !keysA.includes(k));
    const schemaLine = (label, top, cluster) =>
      `- ${label} — top level: ${top.length ? top.join(", ") : "none"}; per cluster: ${cluster.length ? cluster.join(", ") : "none"}`;
    const identical = fs.readFileSync(reportAPath, "utf8") === fs.readFileSync(reportBPath, "utf8");
    const lines = [
      "# Deslop version comparison",
      "",
      `- Commits: \`${meta.shortA}\` vs \`${meta.shortB}\``,
      `- Commit subjects: "${meta.subjectA}" / "${meta.subjectB}"`,
      `- Target repo: ${meta.url}${meta.tag ? ` @ tag \`${meta.tag}\`` : " (default branch)"} (sha \`${meta.sha}\`)`,
      `- Config: \`${meta.flags}\``,
      `- Each run: clean build artifacts → delete deslop cache → clean rebuild → scan`,
      `- Distinct binaries verified: shaA ${timingA.binary_sha256.slice(0, 16)}, shaB ${timingB.binary_sha256.slice(0, 16)}`,
      `- Canonical JSON reports byte-identical: **${identical ? "yes" : "no"}**`,
      "",
      "## Stats",
      "",
      `| metric | deslop@${meta.shortA} | deslop@${meta.shortB} |`,
      "|---|---|---|",
      ...statRows.map(([label, a, b]) => `| ${label} | ${a} | ${b} |`),
      ...(struckStats.length ? ["", `Not comparable (absent in one schema): ${struckStats.join(", ")}`] : []),
      "",
      `- Published clusters shared by id: **${sharedCount}** · only in ${meta.shortA}: **${onlyACount}** · only in ${meta.shortB}: **${onlyBCount}**`,
      "",
      `## Top 5 clusters — deslop@${meta.shortA} (fields its schema reports)`,
      "",
      `| ${colsA.map(([, label]) => label).join(" | ")} |`,
      `|${colsA.map(() => "---").join("|")}|`,
      clusterTable(reportA, colsA),
      "",
      `## Top 5 clusters — deslop@${meta.shortB} (fields its schema reports)`,
      "",
      `| ${colsB.map(([, label]) => label).join(" | ")} |`,
      `|${colsB.map(() => "---").join("|")}|`,
      clusterTable(reportB, colsB),
      "",
      "## Report schema changes",
      "",
      schemaLine(`Only in ${meta.shortA}`, onlyInTop(reportA, reportB), onlyInACluster),
      schemaLine(`Only in ${meta.shortB}`, onlyInTop(reportB, reportA), onlyInBCluster),
      "",
      `Reports: \`${reportAPath}\` and \`${reportBPath}\` (each with .txt/.html siblings and logs).`,
      "",
      "Timing caveat: the two scans run sequentially, so the second benefits from warm OS caches — treat small wall-time deltas as noise.",
      "",
    ];
    fs.writeFileSync(summaryPath, lines.join("\n"));
    process.stdout.write(lines.join("\n"));
  ' "$summary_path" "$short_a" "$subject_a" "$short_b" "$subject_b" \
    "$target_url" "$target_tag" "$target_sha" "$flags" \
    "$report_a_path" "$report_b_path"
}

main() {
  [ $# -ge 2 ] && [ $# -le 4 ] || { usage; exit 2; }
  command -v cargo >/dev/null || die "cargo not on PATH"
  command -v node >/dev/null || die "node not on PATH (build.rs regenerates the wire models)"
  command -v shasum >/dev/null || die "shasum not on PATH (each run fingerprints its binary)"
  COMMIT_A="$(resolve_commit "$1")"
  COMMIT_B="$(resolve_commit "$2")"
  [ "$COMMIT_A" != "$COMMIT_B" ] || die "the two commits are identical: $COMMIT_A"

  if [ -n "${3:-}" ]; then
    TARGET_URL="$3"
    TARGET_TAG="${4:-}"
  else
    TARGET_URL="$DEFAULT_REPO_URL"
    TARGET_TAG="$DEFAULT_REPO_TAG"
  fi
  TARGET_SHA="$(resolve_remote_sha "$TARGET_URL" "$TARGET_TAG")"
  [ -n "$TARGET_SHA" ] || die "could not resolve ${TARGET_TAG:-HEAD} on $TARGET_URL"

  prepare_target_repo "$TARGET_URL" "$TARGET_TAG" "$TARGET_SHA"

  local slug reports_base
  slug="$(repo_slug "$TARGET_URL")"
  reports_base="$WORK_DIR/reports/$slug"
  REPORTS_BASE="$reports_base"

  run_cycle "$COMMIT_A"
  local sha_a="$THIS_BINARY_SHA"
  run_cycle "$COMMIT_B"
  local sha_b="$THIS_BINARY_SHA"

  # Build-isolation guard: two different commits must not produce the same
  # binary. If they do, one cycle scanned a stale or duplicated engine and
  # the comparison would be fiction.
  [ "$sha_a" != "$sha_b" ] ||
    die "both cycles produced the identical binary (sha $sha_a) — builds are not isolated"

  echo
  print_summary "$reports_base/SUMMARY.md" \
    "${COMMIT_A:0:12}" "$(git -C "$REPO_ROOT" log -1 --format=%s "$COMMIT_A")" \
    "${COMMIT_B:0:12}" "$(git -C "$REPO_ROOT" log -1 --format=%s "$COMMIT_B")" \
    "$TARGET_URL" "$TARGET_TAG" "$TARGET_SHA" \
    "${RUN_FLAGS[*]}" \
    "$reports_base/${COMMIT_A:0:12}/report.json" "$reports_base/${COMMIT_B:0:12}/report.json"
  echo
  log "summary:  $reports_base/SUMMARY.md"
  log "reports:  $reports_base/${COMMIT_A:0:12}/report.{json,txt,html}"
  log "          $reports_base/${COMMIT_B:0:12}/report.{json,txt,html}"
}

main "$@"
