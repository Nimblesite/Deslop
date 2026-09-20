# VSIX UX Plan

Remaining VS Code extension feature work. The specs are the source of truth; each item below is independently shippable.

## TODO

<!-- Removed 17 Aug: every part of the "Cluster grouping + Duplication panel" item shipped —
     folder mode, sort axis, language group and toolbar all live in clients/vscode/src/tree/,
     the Duplication panel and report webview in webview/panels.ts (pinned by
     duplication-panel.e2e.test.ts), RepoMetrics.per_file is on the wire in live-ipc.td, and the
     HTML per-language sections are in render/html.rs behind `split_by_language`. -->
- [ ] **Diagnostics scope** — add `deslop.diagnostics.scope` (`"open-files"` | `"workspace"`) so Problems can mirror the Top Offenders tree even with no tabs open. Spec: `[LSP-DIAGNOSTICS-SCOPE]` + [vsix.md §VSIX-SETTINGS](../specs/vsix.md#vsix-settings). Issue: [#129](https://github.com/Nimblesite/Deslop/issues/129).
- [ ] **Diagnostic settings** — implement [SEVERITY-CONFIG](../specs/severity.md#severity-config-configuration) and verify [SEVERITY-TESTING](../specs/severity.md#severity-testing-required-checks) through the installed extension. Category names and order come from [CLONE-KIND-LABELS](../specs/taxonomy.md#clone-kind-labels-use-the-same-names-everywhere).
- [ ] **Selected-cluster synchronisation** — one `selectedClusterId` signal locks the editor caret, the Top Offenders tree, the cluster webview, and the bubble together: `deslop.openCluster` and an in-clone caret both `TreeView.reveal(..., { select: true, focus: false })` the matching row (requires `getParent` for every grouping mode), and the tree never steals the caret ([VSIX-CLUSTER-SYNC]). E2E proof across cluster/file/folder modes and on retraction per [VSIX-CLUSTER-SYNC-TESTS]. Issue: [#178](https://github.com/Nimblesite/Deslop/issues/178).
- [ ] **Server-side occurrence columns** (deferred from the VSIX perf pass) — add `start_col`/`end_col` to `ReportOccurrence` in [live-ipc.td](../models/live-ipc.td) and compute them in the LSP so the webview and `locations.ts` need **zero** file reads. The wire `ReportOccurrence` is shared by the CLI/HTML/Markdown/JSON renderers, so the new fields ripple into E2E report snapshots — a self-contained change, not a hot-fix rider.
