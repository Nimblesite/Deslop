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
- Canonical clone buckets in `deslop-core::buckets`, `cluster.bucket` on the
  report, HTML / CLI / VSIX bucket rendering, and AI-match badging.
- JetBrains plugin scaffold: Gradle project, LSP support provider, descriptor,
  binary resolver, and Make targets.

## Remaining Plan Files

- [LSP editor surfaces](lsp-editor-surfaces-plan.md) - remaining standard LSP
  UX beyond diagnostics, hover, code lens, and custom report methods.
- [VS Code schema docs](vscode-schema-docs-plan.md) - optional build-time
  offline `schema_doc.md` copy for the VSIX.
- [Taxonomy content cleanup](taxonomy-content-cleanup-plan.md) - update
  public site and examples to use bucket-first language.
- [JetBrains settings and packaging](jetbrains-settings-packaging-plan.md) -
  settings page, version checks, and bundled binary packaging.
- [JetBrains native UX](jetbrains-ux-plan.md) - Tool Window and embedding
  picker over the existing LSP custom methods.
- [JetBrains E2E](jetbrains-e2e-plan.md) - real Rider / IntelliJ tests with the
  real `deslop-lsp` binary.
- [Interactive TUI](interactive-tui-plan.md) - deliberately deferred terminal
  UI work.

## TODO

- [ ] Finish [LSP editor surfaces](lsp-editor-surfaces-plan.md).
- [ ] Finish [VS Code schema docs](vscode-schema-docs-plan.md).
- [ ] Finish [taxonomy content cleanup](taxonomy-content-cleanup-plan.md).
- [ ] Finish [JetBrains settings and packaging](jetbrains-settings-packaging-plan.md).
- [ ] Finish [JetBrains native UX](jetbrains-ux-plan.md).
- [ ] Finish [JetBrains E2E](jetbrains-e2e-plan.md).
- [ ] Revisit [Interactive TUI](interactive-tui-plan.md) after more real CLI
      operator feedback.
