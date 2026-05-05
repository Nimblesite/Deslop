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
- Deployment Toolkit migration: `deployment-toolkit.json` declares the Deslop
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
- [Remove stub provider from production VSIX](remove-stub-provider-from-production-vsix.md) — move `blake3-stub` to test support and keep production provider surfaces Ollama-only.
- [JetBrains native UX](jetbrains-ux-plan.md) — Tool Window and embedding picker over the existing LSP custom methods.
- [JetBrains E2E](jetbrains-e2e-plan.md) — real Rider / IntelliJ tests with the real `deslop-lsp` binary.
- [Autofix — Extract Method for Type-1](autofix-extract-method-plan.md) — LSP `refactor.extract` code action. Blocked on [#42](https://github.com/Nimblesite/Deslop/issues/42) (Type-1 / Type-2 bucket split).
- [Autofix — AI-assisted Extract for Type-2 / Type-3](autofix-extract-ai-plan.md) — `extract-method-plan` + `extract-method-apply` MCP tools. Blocked on Type-1 path landing.
- [Interactive TUI](interactive-tui-plan.md) — deferred. Revisit after real CLI operator feedback.

## TODO

### 🟡 Remaining features

- [ ] Continue [Language roadmap](LANG-ROADMAP.md) with TypeScript/TSX.
- [ ] Add `deslop.diagnostics.scope` (`"open-files"` | `"workspace"`) so Problems can mirror the Top Offenders tree even with no tabs open. Spec: [lsp.md §LSP-DIAGNOSTICS-SCOPE](../specs/lsp.md#lsp-diagnostics-scope) + [vsix.md §VSIX-SETTINGS](../specs/vsix.md#vsix-settings). Issue: [#129](https://github.com/Nimblesite/Deslop/issues/129).
- [ ] Finish [Remove stub provider from production VSIX](remove-stub-provider-from-production-vsix.md).
- [ ] Finish [JetBrains native UX](jetbrains-ux-plan.md).
- [ ] Finish [JetBrains E2E](jetbrains-e2e-plan.md).
- [ ] Finish [Autofix — Extract Method for Type-1](autofix-extract-method-plan.md) (blocked on [#42](https://github.com/Nimblesite/Deslop/issues/42)).
- [ ] Finish [Autofix — AI-assisted Extract](autofix-extract-ai-plan.md) (blocked on Type-1 landing).
- [ ] Revisit [Interactive TUI](interactive-tui-plan.md) after operator feedback.

### ✅ Done

- **VSIX reactivity**: `@preact/signals-core` wired to `ReportStore`; tree providers, `DecorationManager`, `StatusBar`, `LiveBubble`, and `wirePanel` refresh from signal-driven effects. ESLint now guards the invariant against ad-hoc `reportGet`, timer-driven report refresh, and tree providers without signal subscriptions.
- **Deployment Toolkit migration**: binary version contracts, manifest-backed VS Code and JetBrains startup verification, VSIX package verification, JetBrains package verification, and release gate docs are implemented. `make jetbrains-package` builds the plugin, runs Gradle project/structure checks, and verifies the generated zip with `scripts/verify-jetbrains-package.mjs`.
- **JetBrains settings and packaging**: persistent project settings, validation, settings-derived LSP launch arguments, binary version checks, bundled binary staging, package verification, and local development docs are implemented.
- **LSP file watcher** ([LIVE-WATCHER], [LSP-PUSH]): `LspBackend` starts `LiveWatcher` + `Scheduler`; broadcasts `deslop/reportChanged` + `deslop/analysisState` to the editor. All file changes — agent, git, CI, formatter — trigger immediate re-analysis.
- **LSP editor surfaces**: `textDocument/definition`, deterministic code-lens cycling, `deslop://` virtual documents, and `workspace/executeCommand` dispatch for `deslop.lsp.refreshReport`, `deslop.lsp.openCluster`, `deslop.lsp.openReport`, `deslop.lsp.pickEmbeddingModel`, and `deslop.lsp.toggleIncremental` are implemented with stdio E2E coverage.
- **MCP push notifications infrastructure** ([MCP-NOTIFICATIONS]): `NotificationSender` (`Arc<Mutex<Box<dyn Write + Send>>>`) is wired through `McpBackend::set_notification_sender`; the state-file backend reloads the cache and pushes `resources/updated` + `deslop/reportChanged`.
- **Top Offenders tree grouping**: `cluster` and `file` grouping modes are implemented through `tree/nodes.ts`, `tree/grouping.ts`, `deslop.topOffenders.groupBy`, and the mutually exclusive view-title commands.
