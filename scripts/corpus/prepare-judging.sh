#!/usr/bin/env bash
## prepare-judging: build the folder a clone judge is handed — the repositories,
## the reports, and the judging skill — one workspace per repository the last
## comparison scanned.
##
## The preparer role runs this from inside this repository. The judge role never
## does: it is handed the folder and nothing else, in a session that has never
## read this codebase. See `.agents/skills/clone-register-prepare`.
##
## Usage:   scripts/corpus/prepare-judging.sh <folder> [seed] [name ...]
## Example: scripts/corpus/prepare-judging.sh ~/clone-judging 1 click cobra
##
## With no names, every repository in the comparison is prepared. The folder must
## be OUTSIDE this repository — a judge who can walk up into this source is
## contaminated, and every verdict from that pass is void. One small directory is
## created beside it and never inside it: `<folder>.keys`, the record of which
## engine got which letter. The judge must not have it, and nothing needs it to
## judge.
##
## No repository is cloned twice. The comparison already checked every target out
## at its pinned commit in order to scan it, and that checkout is what gets copied
## into each workspace, so the only copy of a repository that this step creates is
## the `source/` tree the judge reads.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPARE_REPORTS="${COMPARE_REPORTS:-$REPO_ROOT/.corpus/version-compare/reports}"
COMPARE_META="$COMPARE_REPORTS/meta.json"
# shellcheck source=scripts/corpus/target-repos.sh
source "$REPO_ROOT/scripts/corpus/target-repos.sh"

usage() { grep '^## ' "${BASH_SOURCE[0]}" | cut -c4-; }

# ---------------------------------------------------------------------------
# compare-targets: print one tab-separated `slug url sha language reportA reportB`
# row per requested repository, read from the manifest the last comparison wrote.
# The manifest is the only honest source: it pairs each repository with the two
# reports produced for it in one run, so a workspace can never mix a report from
# one run with a report from another.
compare_targets() {
  [ -f "$COMPARE_META" ] || die "no comparison manifest at $COMPARE_META — run \`make compare\` first"
  node -e '
    const meta = require(process.argv[1]);
    const wanted = process.argv.slice(2);
    const bySlug = new Map(meta.targets.map((target) => [target.slug, target]));
    for (const name of wanted) {
      if (!bySlug.has(name)) throw new Error(`${name} was not in the last comparison`);
    }
    const chosen = wanted.length > 0 ? wanted.map((name) => bySlug.get(name)) : meta.targets;
    for (const target of chosen) {
      const { a, b } = target.reports;
      if (!a || !b) throw new Error(`${target.slug} has only one report; two are needed`);
      process.stdout.write(
        [target.slug, target.url, target.sha, target.language, a, b].join("\t") + "\n",
      );
    }
  ' "$COMPARE_META" "$@"
}

main() {
  case "${1:-}" in "" | -h | --help) usage; exit 0 ;; esac
  mkdir -p "$1"
  local root; root="$(cd "$1" && pwd)"; shift
  local seed="${1:-1}"; [ $# -eq 0 ] || shift
  case "$root" in "$REPO_ROOT"*) die "$root is inside this repository; a judge must not be able \
to walk up into the source that produced the reports" ;; esac
  # The comparison's own checkout directory. Its clones are already at the pinned
  # commits — the reports were produced by scanning them — so reusing them is not
  # just faster, it removes any way for a judge to read a tree the scan did not.
  WORK_DIR="$(dirname "$COMPARE_REPORTS")"

  local slug url sha language report_a report_b checkout
  while IFS=$'\t' read -r slug url sha language report_a report_b; do
    is_commit_id "$sha" || die "$slug is pinned by '$sha', which is not a commit id"
    checkout="$(prepare_target_repo "$url" "$sha")"
    log "workspace for $slug ($language) at $(short_sha "$sha")"
    node "$REPO_ROOT/scripts/corpus/register-workspace.mjs" \
      --workspace "$root/$slug" --source "$checkout" \
      --report-one "$report_a" --report-two "$report_b" \
      --url "$url" --sha "$sha" --seed "$seed" --keys "$root.keys" \
      ${NOMINATIONS_DIR:+--nominations "$NOMINATIONS_DIR/$slug.json"}
  done < <(compare_targets "$@")

  log "handed-over folder: $root"
  log "keys, NOT for the judge: $root.keys"
}

main "$@"
