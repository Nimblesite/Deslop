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

Two tracks; land A0 first either way. Literal finding kinds remain on `LiteralFinding` and never widen a clone-cluster enum or cluster filter. B1 may land independently because `compare-pair` and cluster pagination do not depend on literal detection; B2/B3 depend on both tracks only where they render a separate literal-finding section. Wire changes always start in `docs/models/live-ipc.td` plus regeneration — never hand-edited generated code.

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

### A1 — Detection, finding kinds, mass, config, and census (L)

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
- `crates/deslop-core/src/literals/render.rs` — the one human-copy helper derived from `LiteralFinding`.
- `crates/deslop-core/src/config_literals.rs` — `[literals]` + `[workspace]` loading + validation
  ([LITERAL-CONFIG]); state overrides go in `crates/deslop-core/src/state.rs` (the only global-state file). No literal ranking keys or pair-classification overrides exist.

Steps:

1. Walker: thread `source: &[u8]` plus the `WalkHooks` struct (literal_kind fn, constant
   recogniser, `&mut SiteCollector` — one struct, not loose params) through
   `build_normalised_root` / `normalise_node` in `lang/shared.rs`; today the walker has neither
   source bytes nor language hooks. Capture points per [LITERAL-DETECT]; capture is
   config-independent — `enabled = false` skips the join, never the capture.
2. Cache: extend the per-file cached entry with the two site vectors (byte ranges only) +
   encode/decode; decode failure = cache miss, never an error. Round-trip E2E proves it.
3. Add `LiteralFindingKind` with the five wire labels from [CLONE-CATEGORY-REGISTRY]. It belongs only to `LiteralFinding`; clone clusters, pair classifications, cluster facets, and cluster severity never consume it.
4. Join hook: call `build_literal_findings` beside clone materialisation and return a separate mass-ranked `literal_findings` collection. [RANK-LITERAL-FAMILY] owns its unmodified mass; [METRICS-REPO] excludes it from clone line metrics and `clusters_total`.
5. Wire: `live-ipc.td` adds `LiteralFinding`, `LiteralOccurrence`, `LiteralFindingKind`, and `CanonicalTarget`; regenerate Rust and TypeScript. Do not add literal fields to `ReportCluster` or `ReportOccurrence`.
6. Flags: LSP `--literals-enabled` plus the CLI `--no-literals` mirror and VS Code setting per [LITERAL-CONFIG]. Delete literal ranking flags, multipliers, and boosts.
7. REPORTING-CONTEXT.md documents the separate literal-finding record, canonical target, mass, and count fields. It does not tell consumers to read literal kind from a cluster.
8. **Census** ([LITERAL-CENSUS]): run over this repo + the fixture corpus, tune, record numbers in
   [DECISION-LITERALS], bake the census E2E. Gate `enabled = true` on the exit criterion.
9. E2E suites 1–4 + 6 + 7 of [LITERAL-TESTING]. Existing #61/#62/#64/#66/#112/#169 fixtures stay
   green. Update `docs/snippets/agents-md-recipe.md` + site docs (en + zh) with the
   constant-prevention guidance.

Advances open issues: #70 / #79 (literal-aware signals are the prerequisite for
literal-only-variation demotion — note in those issues, don't close), #133 (constant-table FPs).

### A2 — Unused-public-constant marker (M)

[LITERAL-UNUSED-MARKER] + [RANK-UNUSED-PUBLIC]. Identifier + string-word indexes on the cached file
entry (mergeable counts, incremental per file change); suppression cascade exactly as specced — publishability via manifest checks (`publish = false`, `publish_to: none`, `IsPackable`, workspace membership), public-surface heuristics, string-token kill rule; confidence 60/75/90; and `[workspace] monorepo = "auto"` detection. The marker is occurrence metadata on a dedicated literal finding and never boosts, discounts, or reorders mass. E2E suite 5 of [LITERAL-TESTING] asserts the badge string "0 references found in this repo" verbatim. Never a deletion code-action.

## Track B — literal-finding surfaces and the seven-tool core MCP analysis surface

### B1 — core MCP analysis consolidation 12 → 7 (L)

[MCP-TOOLS], [MCP-TOOL-FILTERS], [MCP-TOOL-DUPLICATES], [MCP-TOOL-SESSION],
[DECISION-MCP-SURFACE]. Mechanics (tool names are plain strings in the registry + dispatch table —
no codegen):

1. Wire: in `live-ipc.td`, replace the four page/report payload models with one `DuplicatesPage`; slim `RescanPayload` to `generation` plus the page; keep `ClusterSummary` limited to identity, canonical extent, occurrence count, language/path projection, mass, and rank; add an endpoint-keyed `compare-pair` request/response; and keep literal findings in their own payload rather than adding a cluster category.
2. Schemas: one shared cluster filter builder for language, path, canonical extent, and mass severity; one matching implementation shared by every cluster consumer. Pair classification and literal finding kind are not cluster filters.
3. `ClusterSummary.language` derives from the **core parser registry's** extension map — delete the
   hand-maintained path→language copies in the MCP page builder and the HTML renderer in favour of
   one core helper (fixes Dart `language: "unknown"`; unblocks #164; verify-and-close #170/#198).
4. Handlers: `duplicates` = scope resolve → filter → sort → paginate → detail shape → budget;
   `rescan` = refresh + same path; `session` = action dispatch with the consent invariant verbatim;
   update the payload-cap `next_action` strings to name `duplicates`.
5. Tests: migrate every suite in `crates/deslop-mcp/tests/` to the new spellings via this
   capability map — **never delete a test; every behavioural assertion is preserved**; assertions
   on the tool surface itself (`tools/list` count/names/schemas) are updated to the seven-tool
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
     tool names; remap to the seven-tool spellings. Also fix the stale state-file-architecture text:
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

### B2 — VSIX cluster and literal-finding filters (M)

[FACET-TOP-OFFENDERS-FILTER], [FACET-TOP-OFFENDERS-FILTER-EMPTY], [FACET-GROUP-BY-TYPE], and [FACET-REPORT-WEBVIEW]: cluster views filter only by language, path, and mass severity and group only by cluster, file, folder, or language. Delete `filterBuckets`, `filterCategories`, `groupBy: "type"`, bucket/category selectors, category chips, and per-bucket severity. Dedicated literal-finding views may filter by literal finding kind without projecting that kind onto a clone cluster. Coarse E2E per [FACET-TESTING] proves the two record families remain separate.

### B3 — HTML + CLI facets (S)

[FACET-HTML] and [FACET-CLI] render clone clusters neutrally from membership and mass, while a separate literal-finding section may show literal kind and drift values. No `cat-*` class, pair classification, or literal kind appears on a clone cluster. E2E uses rendered-output assertions per [FACET-TESTING].

## Follow-ups (specced as future work, not scheduled)

Python `typing.Final` / C# enum members / Dart enum constants as constants ([LITERAL-DETECT-SITES]
scope line); approx-float-constant check
(clippy `approx_constant` model); literal-index-powered demotion for #70/#79; promote the
[AUTOFIX-CATALOG] "consolidate duplicate constant/literal" row to planned, consuming
`canonical_target` as the merge target; standalone (non-duplicated) unused-constant findings.

## TODO

- [ ] A0 literal-kind consolidation + normalizer audit
- [ ] A1 detection + dedicated finding kinds + mass + config + census + E2E (gate: [LITERAL-CENSUS])
- [ ] A2 unused-public-constant marker with no mass change
- [ ] B1 core MCP analysis 12→7 consolidation + shared cluster filter block + explicit pair tool + test migration + docs blast radius; preserve separately specified refactor tools
- [ ] B2 VSIX cluster filters plus a separate literal-finding view — #195 successor
- [ ] B3 Neutral clone HTML/CLI plus separate literal-finding sections
- [ ] Close/advance issue pass: verify-close #170/#198; note prerequisite on #70/#79/#133; #164
      unblocked by B1 step 3; record #195 disposition
