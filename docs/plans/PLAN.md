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
  `LanguageFn` grammar pins and Rust / Python AST-golden coverage
  ([TS-UPGRADE.md](TS-UPGRADE.md)).
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
- Deployment Toolkit manifest scaffold: `deployment-toolkit.json` declares the
  Deslop executables, VSIX, JetBrains plugin, host activation checks, and
  release channels. Remaining migration work is tracked separately because it
  changes release and startup guarantees, not editor UI behavior.
- Taxonomy content cleanup: every product-facing `Type-N` mention on the site
  and in `examples/**` leads with a canonical bucket label from
  [taxonomy.md [CLONE-BUCKETS-DUAL-LABEL]](../specs/taxonomy.md#clone-buckets-dual-label).
  Enforced by `scripts/taxonomy-gate.sh` (runs in `make lint`). Research pages
  (`research-background.md`, `ai-generated-code-duplicate-code.md`) are
  allowlisted because the taxonomy is their subject.

## Remaining Plan Files

- [MCP architecture fix](mcp-architecture-fix-plan.md) — **START HERE.** Rip out `PipelineSessionBackend` from `deslop-mcp`. LSP writes `.deslop-cache/live-report.json`; MCP reads it. LSP exposes `.deslop-cache/deslop.sock` for compute delegation. MCP becomes a dumb state wrapper.
- [LSP editor surfaces](lsp-editor-surfaces-plan.md) — remaining standard LSP UX beyond diagnostics, hover, code lens, and custom report methods.
- [Deployment Toolkit migration](deployment-toolkit-migration-plan.md) — binary version contract, manifest-backed VS Code and JetBrains startup verification, VSIX / plugin package verification, CI release gates. (Issues #37–#41.)
- [JetBrains settings and packaging](jetbrains-settings-packaging-plan.md) — settings page, version checks, bundled binary packaging.
- [JetBrains native UX](jetbrains-ux-plan.md) — Tool Window and embedding picker over the existing LSP custom methods.
- [JetBrains E2E](jetbrains-e2e-plan.md) — real Rider / IntelliJ tests with the real `deslop-lsp` binary.
- [Autofix — Extract Method for Type-1](autofix-extract-method-plan.md) — LSP `refactor.extract` code action. Blocked on [#42](https://github.com/Nimblesite/Deslop/issues/42) (Type-1 / Type-2 bucket split).
- [Autofix — AI-assisted Extract for Type-2 / Type-3](autofix-extract-ai-plan.md) — `extract-method-plan` + `extract-method-apply` MCP tools. Blocked on Type-1 path landing.
- [Interactive TUI](interactive-tui-plan.md) — deferred. Revisit after real CLI operator feedback.

## TODO

### 🔴 Critical — MCP architecture fix ([mcp-architecture-fix-plan.md](mcp-architecture-fix-plan.md))

The MCP currently runs its own analysis pipeline. This is wrong. Execute in order:

- [ ] **Phase 1 — LSP writes state file [LIVE-STATE-FILE]**: after every scheduler pass, atomically write `{root}/.deslop-cache/live-report.json`. Write on initial `ready` too. Add E2E test.
- [ ] **Phase 2 — LSP IPC socket [LIVE-IPC-SOCKET]**: on `initialize`, create `.deslop-cache/deslop.sock`. Accept JSON-RPC for `duplicates/findSimilar`, `embedding/listModels`, `session/config`. Remove on shutdown. Add E2E test.
- [ ] **Phase 3 — MCP refactor [MCP-STATE-FILE]**: delete `PipelineSessionBackend`, `SessionState`, `refresh.rs`, all embedding provider usage. Replace with `StateFileReader` (reads + caches state file) + single-file `notify` watcher + IPC client. Remove CLI args `--min-nodes`, `--incremental`, `--embeddings`, `--embedding-provider`, `--embedding-model`, `--embedding-endpoint`. Keep only `--root` and `--config`.
- [ ] **Phase 4 — Wire tools to new backend**: snapshot tools read from cache; `find-similar` and `list-embedding-models` delegate via IPC; `set-embedding-model` writes settings file + IPC notify.
- [ ] **Phase 5 — MCP push notifications rewired [MCP-NOTIFICATIONS]**: notifications now fire from the single-file watcher detecting a new state file, not from internal `mark_changed`. Rewrite `files_changed_pushes_resources_updated_and_report_changed_notifications` to exercise the file-watch path.
- [ ] **Phase 6 — MCP E2E tests updated [MCP-TESTING]**: snapshot-tool tests pre-write a fixture `live-report.json`; compute-tool tests spawn LSP + MCP side-by-side. Coverage threshold does not regress.

### 🟡 Remaining features

- [ ] Finish [LSP editor surfaces](lsp-editor-surfaces-plan.md).
- [ ] Finish [Deployment Toolkit migration](deployment-toolkit-migration-plan.md).
- [ ] Finish [JetBrains settings and packaging](jetbrains-settings-packaging-plan.md).
- [ ] Finish [JetBrains native UX](jetbrains-ux-plan.md).
- [ ] Finish [JetBrains E2E](jetbrains-e2e-plan.md).
- [ ] Finish [Autofix — Extract Method for Type-1](autofix-extract-method-plan.md) (blocked on [#42](https://github.com/Nimblesite/Deslop/issues/42)).
- [ ] Finish [Autofix — AI-assisted Extract](autofix-extract-ai-plan.md) (blocked on Type-1 landing).
- [ ] Revisit [Interactive TUI](interactive-tui-plan.md) after operator feedback.

### ✅ Done

- **VSIX reactivity** ([vsix-reactivity-plan.md](vsix-reactivity-plan.md)): `@preact/signals-core` wired to `ReportStore`; all surfaces (`DecorationManager`, `StatusBar`, `LiveBubble`, `wirePanel`) use `effect()` — zero `onDidChange` callbacks.
- **LSP file watcher** ([LIVE-WATCHER], [LSP-PUSH]): `LspBackend` starts `LiveWatcher` + `Scheduler`; broadcasts `deslop/reportChanged` + `deslop/analysisState` to the editor. All file changes — agent, git, CI, formatter — trigger immediate re-analysis.
- **MCP push notifications infrastructure** ([MCP-NOTIFICATIONS]): `NotificationSender` (`Arc<Mutex<Box<dyn Write + Send>>>`) wired through `McpBackend::set_notification_sender`. ⚠️ The trigger mechanism (internal `mark_changed`) is being replaced in Phase 5 above — this item is partially reopened.
