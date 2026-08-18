# Diff-scoped reporting — `--diff` / `--only-changed`

Implements [gh #364](https://github.com/Nimblesite/Deslop/issues/364): scope a report to the code a change actually touches, so pre-merge CI flags new duplication without tripping on legacy debt. Specified by [`cli.md §CLI-ARG-DIFF`](../specs/cli.md), [`pipeline.md §PIPELINE-DIFF-INGEST`](../specs/pipeline.md), [`pipeline.md §OUTPUT-SCHEMA-DIFF-TAGS`](../specs/pipeline.md), and [`pipeline.md §METRICS-DIFF-SCOPE`](../specs/pipeline.md).

## Shape

```bash
git diff main...HEAD | deslop src/ --diff - --only-changed
deslop src/ --diff change.patch --only-changed
```

The scan is always the **whole tree** — cross-file clones between changed code and untouched helpers are the second half of the ask, and the warm parse store ([PIPELINE-INCREMENTAL]) makes the full scan cheap. The diff scopes the *report*, never the *analysis*.

## Decisions — settled here, not during coding

**A over B.** We scope by diff line ranges (what the issue asks for), not by diffing against a persisted prior report. Baseline diffing answers "what changed since CI last ran", fails open on a cold cache or rebased base branch, and inherits an id-stability defect: the cluster id is the minimum member hash (`cluster.rs::cluster_id_source`), so editing that one member re-ids the whole cluster and a legacy cluster reports as newly introduced. Diff scoping is stateless and deterministic. `ReportDelta` stays what it is — the live-session generation delta.

**The diff is parsed, not pattern-matched.** A hand-written line-oriented parser in `deslop-core` (module `diff_scope`) consumes the unified-diff grammar: `diff --git` / `---` / `+++` file headers, rename and `Binary files` lines, `@@ -l[,n] +l[,n] @@` hunk headers, and ` `/`+`/`-`/`\` body lines. No regex anywhere — every token is recognised by exact structural prefix and integer parsing, the same class of code as the TOML config loader. `tree-sitter-diff` 0.1.0 exists but is experimental, and tree-sitter's error-recovery would turn a malformed diff into silently wrong spans; a strict parser that **rejects** anything it does not recognise is the accuracy-correct tool. Output: `path → merged, sorted new-side added-line spans`.

**Stale diffs are refused, not tolerated.** The hunk body carries the new-side content. For every hunk, every context and added line must byte-match the scanned file at the line number the hunk claims (content compared exactly as carried, `\n` terminator excluded). First mismatch → exit `2` naming the file and line. A diff that disagrees with the tree would tag the wrong occurrences, and under `--only-changed` a mis-tag is a silent false negative in a merge gate — the one outcome the accuracy rule exists to prevent.

**Tags are `Option`, never defaulted-false.** `in_diff`, `intersects_diff`, `is_newly_introduced`, `clusters_outside_diff`, and `metrics.diff` are all `Option<...>`, absent unless `--diff` was given. A run without a diff must not assert `is_newly_introduced: false` about anything — that is a claim it has no evidence for.

**`duplication_percent` never changes meaning.** `metrics` stays repo-wide and byte-identical with and without `--diff` (test invariant, same as the [METRICS-REPO-WEIGHTED] no-knob rule). The diff-scoped figure is a separate `metrics.diff` block with its own denominator: duplicated added lines over added lines in analysed files. Under `--only-changed`, `--fail-over` gates on the diff-scoped percent and the report header names which number gated.

**Out of scope.** Persisting baselines across runs (rejected above); the GH-action cache step ([gh #381](https://github.com/Nimblesite/Deslop/issues/381) — independent, lands separately); tagging in live/LSP/MCP sessions (fields stay `None`; a later issue can thread a diff through the session config); `--from-report` + `--diff` (conflict, exit `2` — re-rendering has no tree to verify the diff against).

## Semantics

- Diff paths resolve against the invocation working directory after stripping the `a/`/`b/` prefixes, then re-relativise to the scan root — the form `ReportOccurrence.path` carries. Diff files outside the scan root or absent from the corpus are ignored for tagging and counted on the `diff ingested` tracing event; a repo-root diff legitimately touches files the scan never sees.
- Only **new-side added lines** scope the report (`+` lines; context and deletions do not). A pure rename with no content change adds no lines and tags nothing. Binary hunks tag nothing.
- Intersection is closed-interval on 1-indexed lines — occurrences already carry `start_line`/`end_line` in exactly that form (`report_metrics.rs::byte_range_to_line_range`). One added line inside a 40-line occurrence tags it: touching a clone counts as touching the clone.
- Cluster rollups ignore `hidden` occurrences, matching [METRICS-REPO]'s projection: `intersects_diff` = any non-hidden occurrence in diff; `is_newly_introduced` = all non-hidden occurrences in diff.
- `--only-changed` drops clusters where `intersects_diff != true` from `clusters` before ranking output, counts them in `clusters_outside_diff`, and leaves `metrics` untouched.

## Phases

Every phase is test-first: the E2E tests are written against fixture repos with committed `.patch` files, watched red, then the code lands. Fixtures live beside the existing incremental fixtures; each scenario asserts exact cluster ids, occurrence paths and line ranges, tag values, counts, and exit codes.

### Phase 1 — diff ingest ([PIPELINE-DIFF-INGEST])
`diff_scope` module: parser, path resolution, span merge, tree-verification refusal. `--diff <path|->` accepted and validated; tags not yet emitted.
Exit: unit suite over the grammar (renames, quoted paths, CRLF content, `\ No newline`, binary, malformed input rejected); E2E: stale diff refused with exit `2`; matching diff accepted.

### Phase 2 — tagging ([OUTPUT-SCHEMA-DIFF-TAGS])
Wire fields added in `live-ipc.td` (regenerated, never hand-written); intersection pass stamps occurrences and clusters at render time.
Exit: E2E over the four populations — new duplicate wholly in diff (`is_newly_introduced: true`), changed code cloning an untouched helper (`intersects_diff: true`, `is_newly_introduced: false`, the untouched occurrence `in_diff: false`), legacy cluster (`intersects_diff: false`), and a no-`--diff` run whose JSON carries none of the fields.

### Phase 3 — filtering and the gate ([METRICS-DIFF-SCOPE])
`--only-changed` (usage error without `--diff`), `clusters_outside_diff`, `metrics.diff`, gate rerouting under `--only-changed`.
Exit: E2E: legacy-heavy fixture passes the gate under `--only-changed` with an empty diff and fails it when the diff introduces a clone; `metrics` byte-identical across `--diff` on/off; threshold summary names the diff scope.

### Phase 4 — renderers
Text delta summary (newly-introduced count, cross-file count), occurrence badges (`[in diff]` / `[existing]`) through the one shared occurrence renderer, HTML CSS-only "only diff-affected" toggle. JSON stays canonical; both views derived.
Exit: rendered `.txt`/`.html` assertions in the same E2E fixtures.

## Checklist

- [x] Decisions recorded; specs updated in the same change ([CLI-ARG-DIFF], [CLI-ARG-ONLY-CHANGED], [PIPELINE-DIFF-INGEST], [OUTPUT-SCHEMA-DIFF-TAGS], [METRICS-DIFF-SCOPE])
- [ ] Phase 1 — parser + refusal, unit + E2E red→green
- [ ] Phase 2 — wire fields + tagging E2E
- [ ] Phase 3 — `--only-changed`, `diff_metrics`, gate
- [ ] Phase 4 — text summary, badges, HTML toggle
- [ ] Close #364 with a worked `git diff | deslop` example in the issue
