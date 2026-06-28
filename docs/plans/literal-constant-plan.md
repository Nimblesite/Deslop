# Literal & constant duplication + facets + MCP consolidation — execution plan

Specs (read first, in this order): [literals.md](../specs/literals.md)
(`[LITERAL-DETECT]`, `[LITERAL-DETECT-SITES]`, `[LITERAL-VALUE-NORM]`,
`[LITERAL-CATEGORY-MAGIC]`, `[LITERAL-CATEGORY-SHADOWED]`,
`[LITERAL-CATEGORY-CONST-DUP]`, `[LITERAL-CATEGORY-CONST-DRIFT]`,
`[LITERAL-CATEGORY-CONST-ALIAS]`, `[LITERAL-NOISE]`, `[LITERAL-CANONICAL]`,
`[LITERAL-WIRE]`, `[LITERAL-CACHE]`, `[LITERAL-UNUSED-MARKER]`,
`[LITERAL-CONFIG]`, `[LITERAL-CENSUS]`, `[LITERAL-TESTING]`),
[taxonomy.md §CLONE-CATEGORY-REGISTRY](../specs/taxonomy.md#clone-category-registry),
[pipeline.md §RANK-LITERAL-FAMILY / §RANK-UNUSED-PUBLIC](../specs/pipeline.md#rank-literal-family),
[facets.md](../specs/facets.md)
(`[FACET-MODEL]`, `[FACET-TOP-OFFENDERS-FILTER]`,
`[FACET-TOP-OFFENDERS-FILTER-EMPTY]`, `[FACET-GROUP-BY-TYPE]`,
`[FACET-REPORT-WEBVIEW]`, `[FACET-HTML]`, `[FACET-CLI]`, `[FACET-MCP]`,
`[FACET-TESTING]`), [mcp.md](../specs/mcp.md),
[decisions.md §DECISION-LITERALS / §DECISION-MCP-SURFACE](../specs/decisions.md#decision-literals).

Two tracks; land A0 first either way. **Sequencing rule:** B1 depends on A1 steps 3 and 5 for
`CloneCategory::all()` (the method does not exist yet), the `constant_name` / `literal_value` wire
fields behind `name_contains` / `value_contains`, and the literal-family fixtures behind the
categories-filter tests. If B1 must land before A1: ship the filter block without
`name_contains`/`value_contains`, add `CloneCategory::all()` over the existing two variants in B1,
and defer the `magic_literal` filter test to A1 (the enums are registry-derived at schema-build
time, so they widen automatically when A1 lands). B2/B3 depend on both tracks. Every phase = one
PR, green `make ci`, coarse E2E proof per the spec's testing section, no co-author stamps. Wire
changes always start in `docs/models/live-ipc.td` + regen — never hand-edited generated code.

## Track A — the literal/constant finding family

### A0 — Literal-kind consolidation (S)

Goal: one `literal_kind(raw: &str) -> Option<LiteralKind>` per language module is the single
source of truth for "what is a literal" ([LITERAL-DETECT] point 1).

1. Audit each `crates/deslop-core/src/lang/<lang>.rs` against its pinned grammar: enumerate every
   literal node kind the grammar exposes; diff against the kinds the module currently collapses.
   Known suspects from the design survey (verify before fixing): C# `raw_string_literal` not
   collapsed; a dead C# `interpolated_string_text` arm; string-content kinds inconsistently listed.
2. Add `LiteralKind { Str, Number, Bool, Null, Char }` + per-language `literal_kind()` in the new
   `crates/deslop-core/src/literals/` module tree; make `normalise_kind`'s literal arm and the
   highlight renderer's literal classification delegate to it. Delete the superseded lists.
3. Any normalisation change is fingerprint-changing: bump invalidates caches automatically; update
   affected golden reports in the same PR and say so in the PR body.
4. E2E: per-language fixture asserting each literal kind normalises to `__literal__` and
   fingerprints stably across an edit to a raw/verbatim string. // [PIPELINE-NORMALIZE-AST]

### A1 — Detection, categories, ranking, config, census (L)

The core of [literals.md]. Suggested file layout (all new files < 500 lines):

- `crates/deslop-core/src/literals/mod.rs` — `LiteralKind`, `LiteralSite`, `ConstSite`,
  `ContainerKind`, `SiteCollector`.
- `crates/deslop-core/src/literals/{csharp,rust,python,dart}.rs` — `literal_kind()` + the constant
  recogniser per [LITERAL-DETECT-SITES] (~40 lines each). **Move** (never copy) the existing Python
  UPPER_SNAKE/constant-value predicates out of `cluster_filters` and re-import.
- `crates/deslop-core/src/literals/value.rs` — [LITERAL-VALUE-NORM] normalisation; fallback to
  raw-text equality on any parse failure; no `unwrap`.
- `crates/deslop-core/src/literals/join.rs` — the four group-bys + `shadowed_constant` join +
  [LITERAL-NOISE] gates + [LITERAL-CANONICAL] pick + `max_findings` cap.
- `crates/deslop-core/src/literals/copy.rs` — the one summary/interpretation helper.
- `crates/deslop-core/src/config_literals.rs` — `[literals]` + `[workspace]` loading + validation
  ([LITERAL-CONFIG]); `[ranking]` keys join the existing ranking-policy code; state overrides go in
  `crates/deslop-core/src/state.rs` (the only global-state file) following the
  `set_structural_only_override` first-write-wins pattern.

Steps:

1. Walker: thread `source: &[u8]` plus the `WalkHooks` struct (literal_kind fn, constant
   recogniser, `&mut SiteCollector` — one struct, not loose params) through
   `build_normalised_root` / `normalise_node` in `lang/shared.rs`; today the walker has neither
   source bytes nor language hooks. Capture points per [LITERAL-DETECT]; capture is
   config-independent — `enabled = false` skips the join, never the capture.
2. Cache: extend the per-file cached entry with the two site vectors (byte ranges only) +
   encode/decode; decode failure = cache miss, never an error. Round-trip E2E proves it.
3. `CloneCategory`: five new variants + wire labels + chips + action sentences per
   [CLONE-CATEGORY-REGISTRY]; add `CloneCategory::all()`; every schema/facet enum derives from it.
4. Join hook: call `build_literal_clusters` at the render stage beside existing cluster
   materialisation; output enters the same ranked stream. [RANK-LITERAL-FAMILY] weight formula +
   the three policy knobs; [METRICS-REPO] exclusion.
5. Wire: `live-ipc.td` — `ReportCluster.{constant_name, literal_value, canonical_target}`,
   `ReportOccurrence.{constant_value, container, unused_confidence}` (the last lands inert until
   A2), `CanonicalTarget`; regen Rust + TS; mirror `Category`/`CATEGORIES`/`categoryLabels` in
   `clients/vscode/src/types/report.ts`.
6. Flags: LSP `--literals-enabled`, `--ranking-magic-literals`, `--ranking-constant-findings`
   (+ CLI mirrors incl. `--no-literals`); VS Code settings per [LITERAL-CONFIG]; reject-list and
   startup-parse tests like the structural-only flags.
7. REPORTING-CONTEXT.md (ships inside the binary): document the five categories, the
   signals-are-zero rule, `canonical_target`, and the `categories` filter guidance — same PR as the
   code so the shipped schema_doc never lies.
8. **Census** ([LITERAL-CENSUS]): run over this repo + the fixture corpus, tune, record numbers in
   [DECISION-LITERALS], bake the census E2E. Gate `enabled = true` on the exit criterion.
9. E2E suites 1–4 + 6 + 7 of [LITERAL-TESTING]. Existing #61/#62/#64/#66/#112/#169 fixtures stay
   green. Update `docs/snippets/agents-md-recipe.md` + site docs (en + zh) with the
   constant-prevention guidance.

Advances open issues: #70 / #79 (literal-aware signals are the prerequisite for
literal-only-variation demotion — note in those issues, don't close), #133 (constant-table FPs).

### A2 — Unused-public-constant marker (M)

[LITERAL-UNUSED-MARKER] + [RANK-UNUSED-PUBLIC]. Identifier + string-word indexes on the cached file
entry (mergeable counts, incremental per file change); suppression cascade exactly as specced —
publishability via manifest checks (`publish = false`, `publish_to: none`, `IsPackable`,
workspace membership), public-surface heuristics, string-token kill rule; confidence 60/75/90;
`[workspace] monorepo = "auto"` detection; boost knob + validation `[1.0, 10.0]`;
`--ranking-unused-public` + `deslop.ranking.unusedPublic`. E2E suite 5 of [LITERAL-TESTING] —
the badge string "0 references found in this repo" asserted verbatim. Never a deletion code-action.

## Track B — facets + the six-tool MCP surface

### B1 — MCP consolidation 12 → 6 (L)

[MCP-TOOLS], [MCP-TOOL-FILTERS], [MCP-TOOL-DUPLICATES], [MCP-TOOL-SESSION],
[DECISION-MCP-SURFACE]. Mechanics (tool names are plain strings in the registry + dispatch table —
no codegen):

1. Wire: in `live-ipc.td`, replace the four page/report payload models with one `DuplicatesPage`;
   slim `RescanPayload` to `generation` + `summary` + the page; `ClusterSummary` gains `category`;
   the filter echo gains `buckets`/`categories`/`languages`/`name_contains`/`value_contains`.
   Fix the mcp.md-prose-vs-model drift (summary fields are exactly the [MCP-TOOL-DUPLICATES] list).
2. Schemas: one shared filter-block builder + shape-block builder; **all three enums derived from
   the registries** (`ClusterKind::all()`, `CloneCategory::all()`, language registry). One
   `matches_filters` implementation shared by every consumer.
3. `ClusterSummary.language` derives from the **core parser registry's** extension map — delete the
   hand-maintained path→language copies in the MCP page builder and the HTML renderer in favour of
   one core helper (fixes Dart `language: "unknown"`; unblocks #164; verify-and-close #170/#198).
4. Handlers: `duplicates` = scope resolve → filter → sort → paginate → detail shape → budget;
   `rescan` = refresh + same path; `session` = action dispatch with the consent invariant verbatim;
   update the payload-cap `next_action` strings to name `duplicates`.
5. Tests: migrate every suite in `crates/deslop-mcp/tests/` to the new spellings via this
   capability map — **never delete a test; every behavioural assertion is preserved**; assertions
   on the tool surface itself (`tools/list` count/names/schemas) are updated to the six-tool
   registry and strengthened to assert the retired names are absent. Add the new
   filter/sort/scope cases from [MCP-TESTING].

   | Retired tool | Equivalent call |
   |---|---|
   | `top-offenders` | `duplicates {}` (defaults: `limit 5`, `detail "full"`, `max_occurrences 15`) |
   | `report-get` | `duplicates { offset, limit, detail: "summary" }` |
   | `report-query` | `duplicates { offset, limit, …filters }` |
   | `report-for-file` | `duplicates { path, detail: "full" }` |
   | `report-for-range` | `duplicates { path, start_byte, end_byte, detail: "full" }` |
   | `list-embedding-models` | `session { action: "list-embedding-models" }` |
   | `set-embedding-model` | `session { action: "set-embedding-model", … }` |
   | `session-config` | `session {}` |
6. Docs blast radius — the explicit checklist (grep for each retired tool name when done; en + zh
   site mirrors both):
   - `docs/specs/live.md` — the [MCP-IPC-DISCOVERY] consumer column maps IPC methods to retired
     tool names; remap to the six-tool spellings. Also fix the stale state-file-architecture text:
     the line-3 "deslop-mcp reads that state file" claim, the McpProc "state-file reader +
     in-memory cache" diagram node, and the [LIVE-WATCHER] sentence saying the MCP watches
     `live-report.json` — all contradict [MCP-IPC-CLIENT]/[MCP-NOTIFICATIONS] (no file reads, no
     watcher, no caches) and live.md's own [LIVE-NOTIFICATIONS].
   - `docs/specs/autofix-extract.md` — the `session-config` reference → `session {}`.
   - `docs/specs/SPEC.md` — the MCP surface row enumerating retired tool names.
   - `docs/models/live-ipc.td` — retired spec-ID comment anchors ([MCP-TOOL-REPORT-PAGINATION],
     [MCP-TOOL-REPORT-QUERY], etc.) → [MCP-TOOLS]/[MCP-TOOL-DUPLICATES].
   - README tool table; CLAUDE.md tool-name directives ("use `top-offenders` and `cluster-by-id`"
     → `duplicates` / `cluster-by-id`); `docs/snippets/agents-md-recipe.md` (incl. the "other
     Deslop tools" section); `site/src/docs/*` + `site/src/zh/docs/*` + blog posts enumerating
     tools; `site/src/index.njk` / `zh` if they list tools.
   - `find-similar` keeps its name everywhere — it is brand surface.

### B2 — VSIX facets (M)

[FACET-TOP-OFFENDERS-FILTER], [FACET-TOP-OFFENDERS-FILTER-EMPTY],
[FACET-GROUP-BY-TYPE], [FACET-REPORT-WEBVIEW]:
`filterBuckets`/`filterCategories` settings + `chooseFilter` QuickPick + context key + status row;
`groupBy: "type"` reusing the bucket-group node machinery; webview bucket/category selects +
registry-derived language options (Dart) + shared severity helper; category chips/icons through the
one label helper; Copy Context For AI payloads include the literal-family fields. Coarse E2E per
[FACET-TESTING]; toolbar `navigation@N` pin assertions updated in-PR (Choose Filter lands at
`navigation@4`, shifting Expand/Collapse/Refresh to `@5`–`@7`). In the same PR, make severity.md's
bucket enumerations match `ClusterKind::all()` (five buckets — vsix.md's settings rows already
list `structuralOnly`; severity.md still says four). Delivers the unbuilt asks of #195 (file a
successor issue or reopen — maintainer's call recorded in the PR body; the category-not-bucket
`type` grouping divergence is recorded in [FACET-GROUP-BY-TYPE]); advances #162.

### B3 — HTML + CLI facets (S)

[FACET-HTML] CSS-only inputs + `cat-*` classes + breakdown clause + per-occurrence drift values;
[FACET-CLI] literal-family breakdown line. E2E: rendered-output assertions per [FACET-TESTING].

## Follow-ups (specced as future work, not scheduled)

Python `typing.Final` / C# enum members / Dart enum constants as constants ([LITERAL-DETECT-SITES]
scope line); category-keyed severity override (ties into #177); approx-float-constant check
(clippy `approx_constant` model); literal-index-powered demotion for #70/#79; promote the
[AUTOFIX-CATALOG] "consolidate duplicate constant/literal" row to planned, consuming
`canonical_target` as the merge target; standalone (non-duplicated) unused-constant findings.

## TODO

- [ ] A0 literal-kind consolidation + normalizer audit
- [ ] A1 detection + categories + ranking + config + census + E2E (gate: [LITERAL-CENSUS])
- [ ] A2 unused-public-constant marker + boost
- [ ] B1 MCP 12→6 consolidation + shared filter block + test migration + docs blast radius
- [ ] B2 VSIX facets (filter, type grouping, webview) — #195 successor
- [ ] B3 HTML/CLI facets
- [ ] Close/advance issue pass: verify-close #170/#198; note prerequisite on #70/#79/#133; #164
      unblocked by B1 step 3; record #195 disposition
