# Top Offenders — Cluster vs File grouping toggle (implementation plan)

> **Spec is the source of truth.** Behavioural shape, sort order, label rules, and persistence semantics live in [vsix.md](../specs/vsix.md) under [VSIX-TOP-OFFENDERS-GROUPING], [VSIX-TOP-OFFENDERS-CLUSTER-MODE], [VSIX-TOP-OFFENDERS-FILE-MODE], and [VSIX-TOP-OFFENDERS-RANK-GLOBAL]. This file describes only **how** to build it.

## Context

The current implementation at [providers.ts:250-258](../../clients/vscode/src/tree/providers.ts#L250-L258) sorts cluster rows alphabetically by representative file then by impact rank within file — the legacy `[VSIX-TOP-OFFENDERS-FILE-GROUPS]` behaviour. We're replacing that with the two-mode shape spec'd in [VSIX-TOP-OFFENDERS-GROUPING]. No back-compat, no preserved old behaviour — legacy is deleted.

## File splits required first

[CLAUDE.md hard rule: files < 500 lines.] These additions push two files past the limit, so split before adding behaviour:

- [clients/vscode/src/tree/providers.ts](../../clients/vscode/src/tree/providers.ts) (356 lines) → split into:
  - `clients/vscode/src/tree/nodes.ts` — **new**. `ClusterNode`, `OccurrenceNode`, `FileNode`, `BucketGroupNode`, `StatusNode`, `SessionFieldNode`. Promote `representativePath`, `displayPath`, `categoryIcon`, `CATEGORY_STYLE` to exports so `BucketGroupNode` / `FileNode` reuse them.
  - `clients/vscode/src/tree/grouping.ts` — **new**. Pure functions `buildClusterMode(clusters, severities)` and `buildFileMode(clusters, severities)` returning `Node[]`.
  - `clients/vscode/src/tree/providers.ts` — keeps `LifecycleAwareProvider`, the three providers, `StatusTicker`. **Re-exports** node classes from `./nodes` so existing imports in [register.ts](../../clients/vscode/src/commands/register.ts) and [treeMenus.ts](../../clients/vscode/src/commands/treeMenus.ts) keep working unchanged.

- [clients/vscode/src/test/unit/tree.unit.test.ts](../../clients/vscode/src/test/unit/tree.unit.test.ts) (506 lines, **already over the limit**) → split:
  - `clients/vscode/src/test/unit/tree.helpers.ts` — `cluster()`, `report()`, `bucketSignals()`, `labelText()`, `iconColorId()`, `tooltipText()` factories. Non-`.test.ts` so the Mocha glob doesn't load it as a suite.
  - `tree.topOffenders.unit.test.ts`, `tree.focusedFile.unit.test.ts`, `tree.session.unit.test.ts`.

## Setting + toggle wiring

`package.json` contributions:

- `deslop.topOffenders.groupBy` under `contributes.configuration.properties`: `enum: ["cluster","file"]`, `default: "cluster"`, `scope: "window"`. Description references the spec.
- New command `deslop.toggleTopOffendersGrouping` with an `icon` field on the *command* (so the title-bar button renders).
- Two `view/title` menu entries on `deslop.topOffenders` view, gated by mutually exclusive `when` clauses on `deslop.topOffendersGroupBy == 'cluster'` vs `'file'`. Each entry uses a different codicon to visualise the *next* state.

Toggle command body — copy the [`deslop.toggleShowAllLenses`](../../clients/vscode/src/commands/register.ts#L68-L72) pattern verbatim, including `ConfigurationTarget.Workspace`. Read setting defensively with a `cluster` fallback so an unknown / missing value never panics.

Activation bridge in [extension.ts](../../clients/vscode/src/extension.ts) `activate()`:

1. **Synchronously, before any tree provider is registered**, read the setting and call `vscode.commands.executeCommand("setContext", "deslop.topOffendersGroupBy", value)`. Otherwise both `when` clauses are false on cold start and neither button renders.
2. Subscribe to `vscode.workspace.onDidChangeConfiguration`. When `deslop.topOffenders.groupBy` changes, refresh the context key and fire the `TopOffendersProvider` change emitter so the tree rebuilds.

Multi-root caveat: `ConfigurationTarget.Workspace` writes to `.code-workspace` in a multi-root layout; single-root writes to `.vscode/settings.json` as expected. Don't use `WorkspaceFolder` — it forces folder selection.

## Tree provider dispatch

`TopOffendersProvider.getChildren()`:

- Compute `indexedSeverity(report.clusters)` once at the top.
- Dispatch on the cached mode value (read once per `getChildren()` call) into `buildClusterMode` or `buildFileMode` from `tree/grouping.ts`.
- Cluster mode iterates `report.clusters` directly (the LSP guarantees weight-desc order).
- File mode keys by [`representativePath()`](../../clients/vscode/src/tree/providers.ts#L76-L78), groups buckets via [`resolveBucket(cluster)`](../../clients/vscode/src/types/report.ts#L302-L307) (never touch `cluster.bucket` directly — optional on v3 reports), and applies the sort tuple from [VSIX-TOP-OFFENDERS-FILE-MODE].

`ClusterNode` label — extract a free function `clusterRowLabel({ rank, severity, bucket, file? })`. The constructor calls it. File mode passes `file: undefined`, cluster mode passes the display path. Tooltip stays mode-invariant ([VSIX-TOP-OFFENDERS-RANK-GLOBAL] / [VSIX-TOP-OFFENDERS-FILE-MODE] both require this).

`FileNode` and `BucketGroupNode` are display-only — `contextValue` left off the existing `deslop.cluster` / `deslop.occurrence` keys so no context-menu entry needs to be added or scoped.

## Existing helpers — reuse, do not duplicate

- [`bucketLabels(bucket).plainTitle`](../../clients/vscode/src/types/report.ts#L275-L277) — bucket group title text.
- [`resolveBucket(cluster)`](../../clients/vscode/src/types/report.ts#L302-L307).
- [`indexedSeverity()`, `SEVERITY_DOT`](../../clients/vscode/src/severity.ts).
- [`occurrenceDisplayLocation()`](../../clients/vscode/src/locations.ts#L14-L27).
- `representativePath`, `displayPath`, `categoryIcon`, `CATEGORY_STYLE` — promote from [providers.ts:27-87](../../clients/vscode/src/tree/providers.ts#L27-L87) into the new `tree/nodes.ts`.

## Tests to add (under `src/test/unit/`)

Drive `TopOffendersProvider.getChildren()` directly against a seeded `ReportStore` — existing pattern, no fake LSP.

1. Cluster mode (default): clusters appear weight-desc as roots; no file grouping.
2. File mode top-level rows are files, sorted by max cluster weight desc, sum-weight tiebreaker, path `localeCompare` final.
3. File mode children of a file are bucket groups; only buckets present appear.
4. File mode clusters under a bucket sorted by weight desc.
5. File mode occurrence leaves byte-identical (label, description, command) to cluster mode.
6. `#N` rank stays the global worst-first rank in both modes ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]).
7. Cluster row label drops the `· file.cs` suffix in file mode but keeps it in cluster mode; tooltip preserves the file path in both modes.
8. Setting flip via `cfg.update(...)` triggers a tree change event and rebuilds; cold-start respects the persisted value.
9. Unknown / missing `topOffenders.groupBy` value falls back to `"cluster"` (no panic).

Each split test file stays under 500 lines.

## Verification

1. `cd clients/vscode && npm run typecheck && npm run lint` — must pass.
2. `cd clients/vscode && npm test` — full unit + e2e suite. The e2e suite under `src/test/suite/` builds and installs the real LSP/MCP binaries (CLAUDE.md: no fake LSP).
3. Manual end-to-end against `examples/csharp/` per the cold-start, toggle, persist, and user-vs-workspace cases the spec calls out.
4. `make ci` from the repo root.

---

## TODO

- [x] Update [vsix.md](../specs/vsix.md): replace `[VSIX-TOP-OFFENDERS-FILE-GROUPS]` with `[VSIX-TOP-OFFENDERS-GROUPING]`, `[VSIX-TOP-OFFENDERS-CLUSTER-MODE]`, `[VSIX-TOP-OFFENDERS-FILE-MODE]`, `[VSIX-TOP-OFFENDERS-RANK-GLOBAL]`.
- [x] Split [providers.ts](../../clients/vscode/src/tree/providers.ts) → `tree/nodes.ts` + `tree/grouping.ts` + slim `providers.ts` with re-exports. Verify `register.ts` and `treeMenus.ts` imports still resolve.
- [x] Add `FileNode` and `BucketGroupNode` to `tree/nodes.ts`, promoting `representativePath` / `displayPath` / `categoryIcon` / `CATEGORY_STYLE` to exports.
- [x] Extract `clusterRowLabel({ rank, severity, bucket, file? })` free function; update `ClusterNode` to call it; verify tooltip stays mode-invariant.
- [x] Implement `buildClusterMode(clusters, severities)` in `tree/grouping.ts`. Spec ID comment: `[VSIX-TOP-OFFENDERS-CLUSTER-MODE]`.
- [x] Implement `buildFileMode(clusters, severities)` in `tree/grouping.ts` with the max-weight / sum-weight / `localeCompare` sort tuple. Spec ID comment: `[VSIX-TOP-OFFENDERS-FILE-MODE]`.
- [x] Wire `TopOffendersProvider.getChildren()` to dispatch on the `deslop.topOffenders.groupBy` setting with a `cluster` fallback. Spec ID comment: `[VSIX-TOP-OFFENDERS-GROUPING]`.
- [x] Add `deslop.topOffenders.groupBy` configuration property in `package.json`.
- [x] Add grouping commands with codicons and two `view/title` menu entries gated by mutually exclusive `when` clauses on `deslop.topOffendersGroupBy`. The implementation uses explicit `showByCluster` / `showByFile` commands instead of one ambiguous toggle.
- [x] Register the grouping commands in [register.ts](../../clients/vscode/src/commands/register.ts).
- [x] Add the synchronous `setContext` bridge in [extension.ts](../../clients/vscode/src/extension.ts) `activate()`, before tree registration. Add the `onDidChangeConfiguration` listener that updates the context key and fires the `TopOffendersProvider` change emitter.
- [x] Split [tree.unit.test.ts](../../clients/vscode/src/test/unit/tree.unit.test.ts) into `tree.helpers.ts` + `tree.topOffenders.unit.test.ts` + `tree.focusedFile.unit.test.ts` + `tree.session.unit.test.ts`. Confirm Mocha glob still loads them.
- [x] Add tests 1–9 listed above. Each test references the spec ID it covers.
- [x] `cd clients/vscode && npm run typecheck && npm run lint && npm test` — green. Verified locally: typecheck ✅, lint ✅, `npm test` 290 passing ✅.
- [x] `make ci` — green.
- [ ] Manual e2e in a real VS Code window: cold start in cluster mode, toggle to file mode, reload, confirm `.vscode/settings.json` contains the override, confirm user-level fallback when no workspace value is set.
