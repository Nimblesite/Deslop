# Deslop Plans

This directory tracks remaining work only. The old mega-plan mixed completed
history, stale checklists, and future ideas in one file; that history is now
left to the code, specs, and tests.

## Current Baseline

Implemented work intentionally not repeated here:

- Core CLI, C# / Rust / Python parsing, normalization, fingerprinting, LSH,
  embeddings, exclusion config, metrics, fail-over, incremental cache, and
  human-readable HTML reports.
- P-LANG-0 tree-sitter runtime upgrade to 0.26.8, including modern
  `LanguageFn` grammar pins and Rust / Python AST-golden coverage. The
  completed checklist was deleted; the durable baseline lives in
  [LANG-ROADMAP.md §LANG-ROADMAP-RUNTIME-UPGRADE](LANG-ROADMAP.md#lang-roadmap-runtime-upgrade).
- Live analysis core, LSP diagnostics, LSP hover, LSP code lens, LSP custom
  `deslop/*` methods, MCP server, and VS Code extension v0.1.
- VS Code schema docs: `deslop.showSchemaDoc` now prefers the LSP RPC/live
  report path and falls back to a packaged `dist/schema_doc.md` copy with
  unit, E2E, and `.vsix` packaging proof.
- VSIX tree context menus (cluster + occurrence rows): Copy Context For AI,
  Copy Human Location, Copy Cluster Locations, Copy Source Snippet, Reveal
  Occurrence In Explorer, Open All Occurrences, Open Cluster Details, and
  Compare With Canonical. Closes gh issues #11, #12, #13, #15, #16, #17, #19
  with unit + E2E coverage.
- Canonical clone buckets in `deslop-core::buckets`, `cluster.bucket` on the
  report, HTML / CLI / VSIX bucket rendering, and AI-match badging.
- JetBrains plugin scaffold: Gradle project, LSP support provider, descriptor,
  binary resolver, and Make targets.
- MCP architecture fix: LSP writes `.deslop-cache/live-report.json`, exposes
  `.deslop-cache/deslop.sock`, and MCP reads/delegates through the state-file
  + IPC path. Close-out evidence lives in
  `crates/deslop-lsp/tests/state_file_and_ipc.rs`,
  `crates/deslop-mcp/tests/cli.rs`, and
  `crates/deslop-mcp/tests/lsp_integration.rs`.
- Deployment Toolkit migration: `shipwright.json` declares the Deslop
  executables, VSIX, JetBrains plugin, host activation checks, and release
  channels. Release gates now validate the manifest, binary version contracts,
  VSIX contents, and JetBrains package contents.
- Taxonomy content cleanup: every product-facing `Type-N` mention on the site
  and in `examples/**` leads with a canonical bucket label from
  [taxonomy.md [CLONE-BUCKETS-DUAL-LABEL]](../specs/taxonomy.md#clone-buckets-dual-label).
  Enforced by `scripts/taxonomy-gate.sh` (runs in `make lint`). Research pages
  (`research-background.md`, `ai-generated-code-duplicate-code.md`) are
  allowlisted because the taxonomy is their subject.

## Remaining Plan Files

- [Language roadmap](LANG-ROADMAP.md) — future parser/plugin rollout. P-LANG-0 is complete; TypeScript/TSX is the next planned language slice.
- [JetBrains native UX](jetbrains-ux-plan.md) — Tool Window and embedding picker over the existing LSP custom methods.
- [JetBrains E2E](jetbrains-e2e-plan.md) — real Rider / IntelliJ tests with the real `deslop-lsp` binary.
- [Autofix — Mechanical (zero-risk) deduplication](autofix-extract-method-plan.md) — LSP code actions + the `merge-plan` MCP tool over a shared `refactor` module: `[AUTOFIX-EXTRACT]` Type-1 verbatim extract, `[AUTOFIX-MERGE]` leaf-gap Type-2/3 call-site merge via anti-unification with default-valued params (the 50+-call-site case), `[AUTOFIX-CONSOLIDATE]` cross-file identical-definition consolidation. Safety underwritten by the static type checker (Dart/C#/Rust first; Python under strict typing). Tier 1 blocked on [#42](https://github.com/Nimblesite/Deslop/issues/42).
- [Autofix — AI-assisted Extract (fallback)](autofix-extract-ai-plan.md) — `extract-method-plan` + `extract-method-apply` MCP tools for the non-mechanical residue (structural drift, Type-4, readability naming). Blocked on the mechanical path landing.
- [Interactive TUI](interactive-tui-plan.md) — deferred. Revisit after real CLI operator feedback.

## TODO

### 🟡 Remaining features

- [ ] **Literal & constant duplication family** — magic literals, shadowed/duplicate/drifting/aliased
  constants as first-class findings on the category axis, plus the monorepo unused-public-constant
  marker. Phased plan: [literal-constant-plan.md](literal-constant-plan.md) Track A. Specs:
  [literals.md](../specs/literals.md), [taxonomy.md §CLONE-CATEGORY-REGISTRY](../specs/taxonomy.md#clone-category-registry),
  [pipeline.md §RANK-LITERAL-FAMILY](../specs/pipeline.md#rank-literal-family),
  [decisions.md §DECISION-LITERALS](../specs/decisions.md#decision-literals). Gated on the
  [LITERAL-CENSUS] noise calibration. Advances [#70](https://github.com/Nimblesite/Deslop/issues/70),
  [#79](https://github.com/Nimblesite/Deslop/issues/79), [#133](https://github.com/Nimblesite/Deslop/issues/133).
- [ ] **Facets + six-tool MCP surface** — bucket/category/language filtering and `type` grouping on
  every surface (tree, webviews, HTML, CLI, MCP); sorting via the MCP `sort` param and the existing
  tree sort axis (webview/HTML stay fixed worst-first by design); the MCP consolidated 12 → 6
  (`find-similar`, `duplicates`, `cluster-by-id`, `rescan`, `session`, `schema-doc`). Phased plan:
  [literal-constant-plan.md](literal-constant-plan.md) Track B. Specs: [facets.md](../specs/facets.md),
  [mcp.md §MCP-TOOLS](../specs/mcp.md#mcp-tools),
  [decisions.md §DECISION-MCP-SURFACE](../specs/decisions.md#decision-mcp-surface). Delivers the
  unbuilt asks of [#195](https://github.com/Nimblesite/Deslop/issues/195); fixes the Dart
  `language: "unknown"` defect behind [#164](https://github.com/Nimblesite/Deslop/issues/164);
  verify-closes [#170](https://github.com/Nimblesite/Deslop/issues/170)/[#198](https://github.com/Nimblesite/Deslop/issues/198).
- [ ] Continue [Language roadmap](LANG-ROADMAP.md) with TypeScript/TSX.
- [ ] **Cluster grouping + Duplication panel** — Top Offenders gains folder-tree grouping ([vsix.md §VSIX-TOP-OFFENDERS-FOLDER-MODE](../specs/vsix.md#vsix-top-offenders-folder-mode)), an impact/path sort axis ([§VSIX-TOP-OFFENDERS-SORT](../specs/vsix.md#vsix-top-offenders-sort)), a per-language split ([§VSIX-TOP-OFFENDERS-LANGUAGE-GROUP](../specs/vsix.md#vsix-top-offenders-language-group), [#162](https://github.com/Nimblesite/Deslop/issues/162)), and collapse/expand/refresh toolbar actions ([§VSIX-TOP-OFFENDERS-TOOLBAR](../specs/vsix.md#vsix-top-offenders-toolbar), [#60](https://github.com/Nimblesite/Deslop/issues/60)). The Focused File panel is replaced by the Duplication panel ([§VSIX-METRICS-PANEL](../specs/vsix.md#vsix-metrics-panel)) + report webview ([§VSIX-METRICS-REPORT](../specs/vsix.md#vsix-metrics-report), [#159](https://github.com/Nimblesite/Deslop/issues/159)), backed by new `RepoMetrics.per_file` ([pipeline.md §METRICS-REPO](../specs/pipeline.md#metrics-repo)). The HTML report gains optional per-language sections ([pipeline.md §OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS](../specs/pipeline.md#output-human-html-language-sections), [#163](https://github.com/Nimblesite/Deslop/issues/163)).
- [ ] Add `deslop.diagnostics.scope` (`"open-files"` | `"workspace"`) so Problems can mirror the Top Offenders tree even with no tabs open. Spec: [lsp.md §LSP-DIAGNOSTICS-SCOPE](../specs/lsp.md#lsp-diagnostics-scope) + [vsix.md §VSIX-SETTINGS](../specs/vsix.md#vsix-settings). Issue: [#129](https://github.com/Nimblesite/Deslop/issues/129).
- [ ] **Severity model + diagnostics-off-by-default** — split severity into two bucket-keyed maps: an always-on **colour** map ([severity.md §SEVERITY-DESLOP-MAP](../specs/severity.md#severity-deslop-map)) driving the bubble / tree / lens / gutter, and an opt-in **diagnostic** map ([§SEVERITY-DIAGNOSTICS](../specs/severity.md#severity-diagnostics)) behind a master `deslop.diagnostics.enabled` gate that **defaults off** ([§SEVERITY-DIAGNOSTICS-GATE](../specs/severity.md#severity-diagnostics-gate)). Make `crates/deslop-lsp/src/diagnostics.rs` resolve `gate → bucket → severity (≠none) → percentile` instead of the current hardcoded `Identical→Error` map. Surface the gate as a prominent one-click toggle at `navigation@0` of the Top Offenders title bar plus a status-bar segment, with an in-flow QuickPick severity editor ([vsix.md §VSIX-SEVERITY-CONTROL](../specs/vsix.md#vsix-severity-control)). Colour comes from `resolveSeverity(bucket, percentile)` in `clients/vscode/src/severity.ts`, with percentile kept as the orthogonal glyph channel ([§SEVERITY-COLOR](../specs/severity.md#severity-color)). E2E proof per [§SEVERITY-TESTING](../specs/severity.md#severity-testing). Issue: [#177](https://github.com/Nimblesite/Deslop/issues/177).
- [ ] **Selected-cluster synchronisation** — one `selectedClusterId` signal locks the editor caret, the Top Offenders tree, the cluster webview, and the bubble together: `deslop.openCluster` and an in-clone caret both `TreeView.reveal(..., { select: true, focus: false })` the matching row (requires `getParent` for every grouping mode), and the tree never steals the caret ([vsix.md §VSIX-CLUSTER-SYNC](../specs/vsix.md#vsix-cluster-sync)). E2E proof across cluster/file/folder modes and on retraction per [§VSIX-CLUSTER-SYNC-TESTS](../specs/vsix.md#vsix-cluster-sync-tests). Issue: [#178](https://github.com/Nimblesite/Deslop/issues/178).
- [ ] Finish [JetBrains native UX](jetbrains-ux-plan.md).
- [ ] Finish [JetBrains E2E](jetbrains-e2e-plan.md).
- [ ] Finish [Autofix — Mechanical (zero-risk) deduplication](autofix-extract-method-plan.md): Tier 1 `[AUTOFIX-EXTRACT]` (blocked on [#42](https://github.com/Nimblesite/Deslop/issues/42)), Tier A `[AUTOFIX-MERGE]` (anti-unification + default params + `merge-plan` tool + `refactor.rewrite` code action), Tier B `[AUTOFIX-CONSOLIDATE]` (cross-file consolidation).
- [ ] Finish [Autofix — AI-assisted Extract (fallback)](autofix-extract-ai-plan.md) for the non-mechanical residue (blocked on the mechanical path landing).
- [ ] Revisit [Interactive TUI](interactive-tui-plan.md) after operator feedback.

### ✅ Done

- **Windows MCP: cross-platform IPC transport** ([live.md §LIVE-IPC-TCP](../specs/live.md#live-ipc-tcp), [§MCP-IPC-DISCOVERY](../specs/live.md#mcp-ipc-discovery)): the MCP⇄LSP bridge no longer requires Unix domain sockets. `deslop_core::live::transport` carries the same line-delimited JSON-RPC over either a Unix socket (default on Unix, byte-for-byte unchanged) or TCP loopback (default on Windows, opt-in via `deslop-lsp --ipc-transport tcp`), discovered through the `IpcEndpointFile` record (`.deslop-cache/deslop.port`, typeDiagram model) and gated by a per-session token line. The MCP client and its `report/subscribe` reader are transport-generic; `LspNotRunning` errors name both candidate endpoints. E2E proof in `crates/deslop-mcp/tests/tcp_transport.rs` (full tool chain over TCP, token rejection, stale-record handling) — platform-neutral on purpose, and a new `windows` CI job runs `cargo check --workspace` plus that suite on `windows-latest` so `cfg(windows)` regressions can no longer ride to release unseen.
- **`StructuralOnly` bucket + `[ranking] structural_only` policy** ([pipeline.md §RANK-STRUCTURAL-ONLY](../specs/pipeline.md#rank-structural-only), [taxonomy.md §CLONE-BUCKETS](../specs/taxonomy.md#clone-buckets); closes the #134→#154→#169→#197 whack-a-mole): `structural_only` is now a first-class `ClusterKind` with one shared signal predicate (`is_structural_only_signals`) driving the wire label, the interpretation text, every renderer (CLI/HTML/LSP severity/VSIX), and a new weight policy — `[ranking] structural_only = "demote"` (default, × `structural_only_weight` = 0.15) / `"ignore"` / `"keep"`, validated like the data knobs. Byte-equivalent families still upgrade to `Identical`; the #134 cross-file and #197 single-file suppressions are unchanged. The editor channel `deslop.ranking.structuralOnly` → `deslop-lsp --ranking-structural-only` (recorded once in `deslop-core::state`) wins over `.deslop.toml`. The MCP `report-query` bucket enum is derived from `ClusterKind::all()`, so agents can filter `structural_only` (#195/#197). E2E proof in `crates/deslop/tests/rank_structural_only_policy.rs` (two-file Dart sibling-method family demoted below a genuine verbatim clone by default; keep/unit-weight restore the pre-fix order; ignore drops and counts hidden; invalid weights rejected).
- **Clone category + demote-not-drop ranking policy** ([#190](https://github.com/Nimblesite/Deslop/issues/190), [pipeline.md §RANK-CATEGORY](../specs/pipeline.md#rank-category), [exclusion.md §CLONE-NOISE-DART-DATA-TABLE-LITERAL](../specs/exclusion.md#clone-noise-dart-data-table-literal)): every cluster now carries a `CloneCategory` (`logic` / `data`) orthogonal to its similarity bucket, surfaced on `ReportCluster.category` in the JSON so text, HTML, and the VSIX tree label and order from one source. Dart top-level collection-literal data tables (`List`/`Set`/`Map` of near-identical constructor / record / map literals, no closure bodies) are detected in `crates/deslop-core/src/cluster_filters/dart_data_table.rs`, closing the #169 gap (that filter only covered `class_body` runs). The verbatim escape hatch (`raw_snippet_texts_differ`) keeps a byte-for-byte copied table at full `logic` weight. A new `[ranking]` config section drives a three-way policy — `data_clones = "demote"` (default, weight × `data_clone_weight` = 0.15) / `"ignore"` (dropped, counted in `clusters_hidden`) / `"keep"` (full weight) — validated with a `ConfigThreshold`-style error for non-finite / out-of-`(0.0, 1.0]` multipliers. Data clusters carry a "data table" chip and a builder/asset action hint instead of "extract the duplicate". E2E proof in `crates/deslop/tests/issue_190_data_table_demote.rs` (demote ordering, JSON label, ignore-drop, keep-restores, verbatim escape hatch, config rejection).
- **Remove stub provider from production VSIX**: the deterministic `blake3-stub` BLAKE3 shim is now `test-support`-feature-gated test infrastructure only (`crates/deslop-core/src/embedding/test_support.rs`); production registers `ollama` exclusively via `ProviderRegistry::production`. No production `src/` path, settings enum, picker row, MCP/LSP/CLI selection, or shipped VSIX asset exposes `stub`. A packaging acceptance gate (`assertNoStubProvider` in `clients/vscode/scripts/verify-vsix-package.mjs`) fails the `.vsix` build if any shipped `package.json` setting enum or `dist/*.{js,json,md}` asset carries `stub`/`blake3-stub`/`StubProvider` — proven against the real artifact plus three tamper cases. Stale `deslop.embedding.provider = "stub"` settings are ignored in memory without rewriting user config. Spec: [fusion.md §FUSION-EMBED-PROVIDER](../specs/fusion.md#fusion-embed-provider) + [vsix.md §VSIX-EMBED-PICKER](../specs/vsix.md#vsix-embed-picker); the gate is proven by `clients/vscode/scripts/stub-gate.test.mjs` (16 cases, run in `make lint`).
- **VSIX reactivity**: `@preact/signals-core` wired to `ReportStore`; tree providers, `DecorationManager`, `StatusBar`, `LiveBubble`, and `wirePanel` refresh from signal-driven effects. ESLint now guards the invariant against ad-hoc `reportGet`, timer-driven report refresh, and tree providers without signal subscriptions.
- **Deployment Toolkit migration**: binary version contracts, manifest-backed VS Code and JetBrains startup verification, VSIX package verification, JetBrains package verification, and release gate docs are implemented. `make jetbrains-package` builds the plugin, runs Gradle project/structure checks, and verifies the generated zip with `scripts/verify-jetbrains-package.mjs`.
- **JetBrains settings and packaging**: persistent project settings, validation, settings-derived LSP launch arguments, binary version checks, bundled binary staging, package verification, and local development docs are implemented.
- **LSP file watcher** ([LIVE-WATCHER], [LSP-PUSH]): `LspBackend` starts `LiveWatcher` + `Scheduler`; broadcasts `deslop/reportChanged` + `deslop/analysisState` to the editor. All file changes — agent, git, CI, formatter — trigger immediate re-analysis.
- **LSP editor surfaces** (additive only, per [LSP-NON-INTERFERENCE]): deterministic code-lens cycling via the Deslop-owned `deslop.jumpToNextOccurrence` command (never `textDocument/definition`), `deslop://` virtual documents, and `workspace/executeCommand` dispatch for `deslop.lsp.refreshReport`, `deslop.lsp.openCluster`, `deslop.lsp.openReport`, `deslop.lsp.pickEmbeddingModel`, and `deslop.lsp.toggleIncremental` are implemented with stdio E2E coverage. The LSP registers **no** `definitionProvider`/`hoverProvider`, so Go To Definition and Hover stay entirely the editor's own.
- **MCP push notifications infrastructure** ([MCP-NOTIFICATIONS]): `NotificationSender` (`Arc<Mutex<Box<dyn Write + Send>>>`) is wired through `McpBackend::set_notification_sender`; the state-file backend reloads the cache and pushes `resources/updated` + `deslop/reportChanged`.
- **Top Offenders tree grouping**: `cluster` and `file` grouping modes are implemented through `tree/nodes.ts`, `tree/grouping.ts`, `deslop.topOffenders.groupBy`, and the mutually exclusive view-title commands.
