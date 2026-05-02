# VSIX reactivity — Preact Signals across every surface

Tracks the work needed to make the VSIX live-react to `deslop/reportChanged`
across **every** surface, not just the cluster webview. Spec authority:
[vsix.md §[VSIX-REACTIVITY]](../specs/vsix.md#vsix-reactivity),
[vsix.md §[VSIX-REACTIVITY-TREE]](../specs/vsix.md#vsix-reactivity-tree),
[vsix.md §[VSIX-REACTIVITY-DECORATIONS]](../specs/vsix.md#vsix-reactivity-decorations),
[vsix.md §[VSIX-REACTIVITY-INVARIANT]](../specs/vsix.md#vsix-reactivity-invariant),
[live.md §[LIVE-NOTIFICATIONS]](../specs/live.md#live-notifications).

## Problem

After a real edit that removes duplicated code (verified against
`/Users/christianfindlay/Documents/Code/ClinicalCoding/ICD10/ICD10.Cli.Tests/CliE2ETests.cs`,
500 lines deleted), the tree view, in-editor decorations, and bubble continue
to display occurrences for clusters that no longer exist in the live report.
The user must restart the LSP / reopen the workspace to see the fresh state.
This violates [VSIX-PRINCIPLES] rules 1, 5, and 8 and [VSIX-REACTIVITY-INVARIANT].

The notification path exists — [`backend.rs`](../../crates/deslop-lsp/src/backend.rs)
sends `deslop/reportChanged`, [`extension.ts`](../../clients/vscode/src/extension.ts)
receives it and calls `applyDelta` on the [`ReportStore`](../../clients/vscode/src/reportStore.ts) —
so the bug is somewhere in (a) the LSP failing to fire the notification on
pure-removal deltas, (b) `applyDelta` not refreshing every consumer, or (c) at
least one consumer reading from a parallel cache instead of the store.

## Phases

### Phase 1 — root-cause the current staleness

Verify against the real reproducer (`CliE2ETests.cs` -500 lines):

1. Confirm with the new structured logging (issue [#45]) whether the LSP
   re-runs the pipeline after the save and whether `deslop/reportChanged`
   is sent. Log the `ChangeSummary` payload at info level.
2. Trace `extension.ts` → `refreshAfterChange` → `reportStore.applyDelta`
   end-to-end with `console.log` (or VS Code dev-tools breakpoints) to
   determine whether the store actually receives the new state.
3. For each consumer (`TopOffendersProvider`, `FocusedFileProvider`,
   `DecorationManager`, `LiveBubble`, `StatusBar`), confirm whether it is
   subscribed to `reportStore.onDidChange` and whether its render path
   reads from the store or from a parallel cache populated at startup.

The output of phase 1 is a numbered list of broken consumers + a
characterisation of the LSP-side notification gap, captured back in
the GH issue.

### Phase 2 — introduce `@preact/signals-core` to the extension host

Today only the webview uses signals. Phase 2 adopts
[`@preact/signals-core`](https://github.com/preactjs/signals) inside
`clients/vscode/src/**` so the extension host shares one signal graph
with the webview.

1. Add `@preact/signals-core` to `clients/vscode/package.json` (extension
   host bundle, not webview-only).
2. Refactor `ReportStore` so it exposes `signal<Report | null>`,
   `signal<AnalysisState>`, `computed<ReportCluster[]>` for top offenders,
   `computed<Map<string, Severity>>` for severity-by-id, etc. Keep the
   `EventEmitter`-based `onDidChange` API as a thin shim over
   `effect()` so the migration can land incrementally.
3. Document the rule in `clients/vscode/src/reportStore.ts` doc comment:
   "All consumers must `effect()` over signals or read `.value` from a
   computed. No consumer may keep a `Report` reference outside this store."

### Phase 3 — migrate every consumer to signals

Per consumer, swap the `EventEmitter` subscription for a `signals.effect()`
that recomputes its render output and triggers the appropriate VS Code
refresh API:

- `clients/vscode/src/tree/providers.ts` — `TopOffendersProvider`,
  `FocusedFileProvider`. Fire `_onDidChangeTreeData` from inside an
  `effect()` over the relevant computed signal. Delete any private
  cache fields.
- `clients/vscode/src/decorations/manager.ts` — `effect()` over
  `report` + `editorVisibleRanges`. Recompute decoration sets per
  visible editor; replace via `setDecorations`.
- `clients/vscode/src/bubble/live.ts` — `effect()` over `report` +
  `selectedClusterId`. Hide / show / repaint via the same pathway.
- `clients/vscode/src/statusBar.ts` (or equivalent) — same shape.
- `clients/vscode/src/tree/providers.ts` `SessionProvider` — same shape.

Each migration is one PR, each PR ships with an E2E test that asserts
the surface updates after a fixture-driven delete.

### Phase 4 — close the LSP-side notification gap (if Phase 1 finds one)

If Phase 1 shows the LSP suppresses `deslop/reportChanged` for pure
removal deltas (or for deltas where the worst cluster is unchanged),
fix the predicate in `crates/deslop-core/src/live/session.rs` /
`crates/deslop-lsp/src/backend.rs`. Spec invariant updated in
[live.md §[LIVE-NOTIFICATIONS]](../specs/live.md#live-notifications):
the notification fires for every observable change, removals included.
Cover with E2E in the live module (`crates/deslop-core/src/live/*`).

### Phase 5 — lint + invariant tests

1. ESLint rule (or equivalent) in `clients/vscode/eslint.config.mjs`
   that bans `setTimeout`-driven UI refresh, ad-hoc `reportGet` outside
   the bootstrap path, and `TreeDataProvider` implementations whose
   `_onDidChangeTreeData.fire` call site is not inside an `effect()`.
2. E2E test asserting [VSIX-REACTIVITY-INVARIANT] across every
   surface — open fixture with N clusters, delete duplicate, assert
   tree + decorations + bubble all show N-1 with no user-initiated
   refresh. Test fails if any surface still references the removed
   cluster id.
3. The fixture must be a real-world C# scenario derived from
   `CliE2ETests.cs` so the bug class that motivated this plan is
   explicitly covered.

## TODO

- [x] Phase 1 — root-cause analysis with structured logging from
      [#45](https://github.com/Nimblesite/Deslop/issues/45). The fixed path is
      captured in `crates/deslop-lsp/src/file_watch.rs`,
      `clients/vscode/src/extension.ts`, and
      `clients/vscode/src/test/suite/live-refresh.e2e.test.ts`.
- [x] Phase 2 — `@preact/signals-core` in the extension host;
      `ReportStore` exposes signals and a shimmed `onDidChange`.
- [x] Phase 3 — migrate tree, decorations, bubble, status bar, session
      panel to `effect()`; delete parallel startup-only caches.
- [x] Phase 4 — LSP notification predicate audited; fires on every
      observable change. Covered by live-refresh E2E and the
      `[VSIX-REACTIVITY-TREE]` watcher regression in
      `crates/deslop-core/tests/live.rs`.
- [ ] Phase 5 — lint rule + cross-surface E2E asserting
      [VSIX-REACTIVITY-INVARIANT] against the deduplication fixture. The
      E2E exists for tree/report generation refresh; the dedicated lint guard
      is still open.
