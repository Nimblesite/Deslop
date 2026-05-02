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

- [LSP editor surfaces](lsp-editor-surfaces-plan.md) - remaining standard LSP
  UX beyond diagnostics, hover, code lens, and custom report methods.
- [JetBrains settings and packaging](jetbrains-settings-packaging-plan.md) -
  settings page, version checks, and bundled binary packaging.
- [Deployment Toolkit migration](deployment-toolkit-migration-plan.md) -
  GitHub issues #37-#41: binary version contract, manifest-backed VS Code and
  JetBrains startup verification, VSIX / plugin package verification, and CI
  release gates.
- [JetBrains native UX](jetbrains-ux-plan.md) - Tool Window and embedding
  picker over the existing LSP custom methods.
- [JetBrains E2E](jetbrains-e2e-plan.md) - real Rider / IntelliJ tests with the
  real `deslop-lsp` binary.
- [Interactive TUI](interactive-tui-plan.md) - deliberately deferred terminal
  UI work.
- [Autofix — Extract Method for Type-1](autofix-extract-method-plan.md) - LSP
  `refactor.extract` code action for true Type-1 clusters. Blocked on
  [#42](https://github.com/Nimblesite/Deslop/issues/42) splitting Type-1 from
  Type-2 in the bucket.
- [Autofix — AI-assisted Extract for Type-2 / Type-3](autofix-extract-ai-plan.md)
  \- two new MCP tools (`extract-method-plan`, `extract-method-apply`) that
  combine a mechanical AST-derived scaffold with an AI-filled name slot. AI
  never writes code; it picks one method name and one canonical name per
  parameter slot. Blocked on the Type-1 path landing.
- [VSIX reactivity](vsix-reactivity-plan.md) - Preact Signals across every
  VSIX surface so `deslop/reportChanged` updates the tree, decorations, and
  bubble in lock-step. Closes the staleness bug where deleted duplicates
  remain visible in the tree until the LSP is restarted.

## TODO

- [ ] Finish [LSP editor surfaces](lsp-editor-surfaces-plan.md).
- [ ] Finish [JetBrains settings and packaging](jetbrains-settings-packaging-plan.md).
- [ ] Finish [Deployment Toolkit migration](deployment-toolkit-migration-plan.md).
- [ ] Finish [JetBrains native UX](jetbrains-ux-plan.md).
- [ ] Finish [JetBrains E2E](jetbrains-e2e-plan.md).
- [ ] Revisit [Interactive TUI](interactive-tui-plan.md) after more real CLI
      operator feedback.
- [ ] Finish [Autofix — Extract Method for Type-1](autofix-extract-method-plan.md)
      (blocked on [#42](https://github.com/Nimblesite/Deslop/issues/42)).
- [ ] Finish [Autofix — AI-assisted Extract for Type-2 / Type-3](autofix-extract-ai-plan.md)
      (blocked on the Type-1 path landing).
- [x] **DONE** — [VSIX reactivity](vsix-reactivity-plan.md): `@preact/signals-core` added to
      extension host; `ReportStore` now uses `signal<T>` internally with `batch()` on
      multi-field updates; `DecorationManager`, `StatusBar`, `LifecycleAwareProvider`,
      `LiveBubble`, and `wirePanel` all use `effect()` directly — zero `onDidChange`
      callbacks on reactive surfaces. `StatusBar._analysing` is now a signal so the
      "analysing…" suffix also tracks reactively.
- [x] **DONE** — Live filesystem watching ([LIVE-WATCHER], [LSP-PUSH]): `LspBackend` now starts
      `LiveWatcher` + `Scheduler` at construction; a background tokio task forwards scheduler
      broadcasts (`ReportChangedNotification`, `AnalysisState`) to the editor as
      `deslop/reportChanged` + `deslop/analysisState`. Changes from AI agents, git, CI, terminals,
      other editors all trigger immediate re-analysis and push — no polling. `file_watch.rs` is
      the single home for all watcher/scheduler wiring.
- [x] **DONE** — MCP push notifications ([MCP-NOTIFICATIONS]): `NotificationSender`
      (`Arc<Mutex<Box<dyn Write + Send>>>`) wired through `McpBackend::set_notification_sender`;
      `mark_changed` pushes `notifications/resources/updated` + `notifications/deslop/reportChanged`
      synchronously; embedding refresh thread pushes the same pair on completion. E2E test
      `files_changed_pushes_resources_updated_and_report_changed_notifications` covers both frames.
