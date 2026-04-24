# LSP Editor Surfaces Plan

## Scope

Finish the remaining standard LSP UX described in
[lsp.md](../specs/lsp.md). Diagnostics, hover, code lens, watched-file updates,
embedding progress, `deslop/reportSchemaDoc`, and the core `deslop/*` report
methods already exist and are not tracked here.

This plan is for editor-neutral LSP features. VS Code-specific UI stays in the
VSIX, and JetBrains-specific native UI stays in the JetBrains plans.

## Implementation Notes

- Keep `crates/deslop-lsp/src/main.rs` as transport glue only.
- Put protocol-specific logic in small modules under `crates/deslop-lsp/src/`.
- All handlers must forward to `LiveApi` or existing report renderers. Do not
  add clone detection, ranking, or bucket routing in the LSP crate.
- Tests must spawn the real `deslop-lsp` binary over stdio and drive JSON-RPC
  frames, matching the existing LSP E2E style.
- No fake LSP, no mocked live service, and no parser shortcuts.

## TODO

- [ ] Add `definitionProvider` capability and handler.
- [ ] Implement "go to definition from inside a clone range" as a jump to the
      canonical occurrence for that cluster.
- [ ] Add deterministic cycling semantics for code-lens "jump to next" if the
      current command path still depends on client-only behavior.
- [ ] Add `documentLinkProvider` capability if the hover or virtual document
      content exposes navigable occurrence links.
- [x] Implement `deslop://cluster/<id>` virtual document rendering from
      `LiveApi::cluster_by_id`, including snippets and line numbers.
- [x] Implement `deslop://report` virtual document rendering from the canonical
      text renderer.
- [x] Implement `deslop://schema` virtual document rendering from
      `report.schema_doc`.
- [ ] Advertise `executeCommandProvider` with `deslop.refreshReport`,
      `deslop.openCluster`, `deslop.openReport`, `deslop.pickEmbeddingModel`,
      and `deslop.toggleIncremental`.
- [ ] Implement command handling without adding edit-producing refactor actions.
- [ ] Add E2E coverage for initialize capabilities, definition lookup, virtual
      document content, command dispatch, and malformed parameter errors.
