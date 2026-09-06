# Webview runtime — the VSIX Preact webviews

This file specifies the three Preact webviews (cluster detail, full report, and duplication report), their signal store, host↔webview protocol, cross-surface contracts, and coverage gate.

This file owns webview-specific requirements; cross-surface invariants remain in [vsix.md](vsix.md):

- [vsix.md §VSIX-STATE](vsix.md#vsix-state) — the single in-process `ReportStore`. Webviews are readers of that store, never holders of a parallel copy.
- [vsix.md §VSIX-STATE-DIRTY](vsix.md#vsix-state-dirty) — canonical report vs. visible projection. Webviews receive the **visible projection** through `postMessage`, while commands that resolve a cluster by id still go through the canonical report.
- [vsix.md §VSIX-REACTIVITY](vsix.md#vsix-reactivity) — Preact Signals everywhere; every surface settles in one microtask after `deslop/reportChanged`.
- [vsix.md §VSIX-CLUSTER-SYNC](vsix.md#vsix-cluster-sync) — one `selectedClusterId` notion across tree, decorations, bubble, and webview.

All webviews follow [PRINCIPLES-LIVE-IS-REACTIVE](principles.md#principles-live-is-reactive).

## [VSIX-WEBVIEW-PROTOCOL] Host↔webview message protocol

The extension host owns data shaping ([VSIX-PRINCIPLES](vsix.md#vsix-principles) principle 4); webview state changes only through host `postMessage` calls. The contract joins `clients/vscode/src/webview/panels.ts` and `clients/vscode/webview-ui/src/store.ts`.

**Handshake.** When a webview mounts it posts `{ kind: "ready" }` to the host (`wireMessagePump()` in `store.ts`). The host delays the first feed for the cluster panel until that `ready` arrives (`wireClusterFeed` in `panels.ts`), so the opened cluster is never pushed into a webview that has not yet wired its message listener.

**Host → webview messages.** The `HostMessage` union in `store.ts` is the authoritative set the webview accepts; any payload without a string `kind` is ignored. `applyHostMessage` is the **sole** batched writer of webview signals — it folds each message into the signal graph inside a single `batch()` so a delta produces one render:

| `kind` | Payload | Effect on webview signals |
|---|---|---|
| `report/snapshot` | `report` | Replaces `report`, bumps `lastUpdatedAt`. The full current snapshot. |
| `report/delta` | `report` | Same writer as snapshot — replaces `report`, bumps `lastUpdatedAt`. (The host always sends a whole report; the webview never reassembles a delta.) |
| `analysis/state` | `state` | Sets `analysisState` (`idle` / `analysing` / …). Lifecycle ticks alone do not re-push the report ([vsix.md §VSIX-PERF](vsix.md#vsix-perf)). |
| `select/cluster` | `id` (`string \| null`) | Sets `selectedClusterId`. The cluster panel pushes this after each feed so the opened cluster stays resolved across id churn. |
| `filter/set` | `filters` | Sets the `filters` signal (language / severity / pathGlob) used by the report webview. |

**Webview → host messages.** The webview posts intents the host turns into real VS Code commands (`handleMessage` in `panels.ts`); an unknown `kind` is a no-op, never a throw:

- `ready` — the mount handshake above.
- `open/cluster` `{ id }` → `deslop.openCluster`.
- `open/occurrence` `{ occurrence }` → `deslop.openOccurrence`.
- `compare/canonical` `{ clusterId }` → `deslop.compareWithCanonical`.
- `refresh` → `deslop.refreshReport`.

The host pushes the **visible projection** for the report and duplication webviews (`store.visibleReport`), and a per-anchor resolved feed for the cluster webview (`clusterPanelFeed`), so an unsaved edit hides occurrences in lock-step with the tree ([vsix.md §VSIX-STATE-DIRTY](vsix.md#vsix-state-dirty)). Each webview's locations are rendered for humans via `reportWithDisplayLocations` before they leave the host — byte offsets never become the primary location text ([vsix.md §VSIX-PRINCIPLES](vsix.md#vsix-principles) principle 7).

## [VSIX-REACTIVITY-WEBVIEW] Webviews mirror the signal graph

**Webviews are built with Preact + `@preact/signals`, not plain React, not manual `useState` ceremony, not event emitters.** `clients/vscode/webview-ui/src/store.ts` exports the `signal<T>` collection: `report`, `selectedClusterId`, `analysisState`, `filters`, `severityByClusterId` (a `computed` over `report`). The extension process posts `postMessage` updates that the webview handler writes into signals; no other path mutates webview state. Components are function components using `@preact/signals` — `const cluster = selectedCluster.value` — not effects, not refs, not class lifecycle. No direct DOM manipulation, no untyped `any` escapes, no `setTimeout`-driven state. If a piece of UI feels like it needs imperative wiring, it's wrong — fold it into a signal or a computed. (The webview-side store is also referenced in code as `[VSIX-WEBVIEW-REACTIVITY]`; the two ids name the same contract.)

## [VSIX-WEBVIEW] Cluster detail webview

Command `deslop.openCluster` opens a webview tab. The tab renders a single cluster with:

- Header: cluster id, rank, weight, size, severity badge, jump-to-next-cluster / jump-to-prev-cluster arrows.
- Interpretation and action hints (the same fields the JSON carries).
- Signal breakdown as four tiny bars: structural, token Jaccard, embedding cosine, fused. Each labelled with its numeric value to two decimals.
- One collapsible panel per occurrence, each containing:
  - File path plus human position (`line:column`), clickable to open the file at that exact editor position.
  - Line-numbered, syntax-highlighted source snippet (reusing the [OUTPUT-HUMAN-HTML](pipeline.md#output-human-html) rendering path — the daemon returns the snippet as pre-highlighted HTML so the webview stays dumb).
  - "Open in editor" and "Reveal in Explorer" buttons.

Navigation is keyboard-first: `j/k` move occurrence focus, `n/p` move cluster focus, `Enter` opens the file at the focused occurrence, `?` shows the shortcut help. The webview is self-contained — no network fetches, no external CDNs, CSP locked to the extension origin.

### [VSIX-WEBVIEW-ACTIONS-CONTEXT] Action wiring and hover context

Cluster detail controls must either execute a real command or not render. `Open` dispatches `deslop.openOccurrence` for the row's occurrence. `Compare` dispatches `deslop.compareWithCanonical` for the row's cluster and stays disabled on the canonical occurrence because comparing the canonical row to itself is meaningless. `Previous cluster` and `Next cluster` update the webview's selected cluster through the same signal path as the `p` and `n` keyboard shortcuts; the extension host must not keep a second copy of cluster selection state.

Every visible data item and action in the cluster detail webview carries a human-readable hover explanation. Signal labels explain what the score means and how to interpret high or low values. Occurrence rows explain the target file, line, column, hidden status, and whether the row is canonical. Rank, weight, size, occurrence count, bucket label, AI-match badge, and keyboard shortcut hints explain their purpose without exposing raw byte offsets as the primary user-facing location.

### [VSIX-CLUSTER-DOCUMENT] Cluster link documents

Cluster references rendered anywhere (copy-for-AI payloads, hovers, report text) are emitted as `deslop://cluster/<id>` URIs. The extension registers a read-only `TextDocumentContentProvider` for the `deslop` scheme so clicking such a link opens a virtual document summarising that cluster — occurrence list, weight, and the structural/jaccard/embedding signals — drawn from the store's visible projection ([vsix.md §VSIX-STATE-DIRTY](vsix.md#vsix-state-dirty)); an unknown or unparseable id renders an explicit placeholder document rather than throwing.

### [VSIX-CLUSTER-ID-CONSISTENCY] One short identity across every surface

Every cluster surface (Top Offenders tree, hover bubble, cluster webview, report webview) and the copy-for-AI payload identify a cluster by the same stable 7-hex slug from the single `clusterSlug()` helper — never two short forms, never the volatile `#N` rank as identity. The slug leads each rendered row and leads the AI payload ahead of the `rank:` line; the full 16-hex `cluster.id` is preserved separately for tooling round-trip. It is the cross-surface twin of [vsix.md §VSIX-TOP-OFFENDERS-CLUSTER-ID](vsix.md#vsix-top-offenders-cluster-id), which governs slug-vs-rank inside the tree.

## [VSIX-REPORT-WEBVIEW] Full report webview

Command `deslop.openReport` opens a second webview with the full ranked list — essentially a live-refreshing version of the HTML renderer from [OUTPUT-SCHEMA-JSON](pipeline.md#output-schema-json), but wired to the daemon's notification stream so it stays current as the user types. Filters: by language, by severity, by file-path glob. Sort is fixed (worst-first) because the whole product premise is worst-first.

## [VSIX-METRICS-REPORT] Duplication report webview

Activating the headline opens a webview (`deslop.openDuplicationReport`) styled like the existing report webview ([VSIX-REPORT-WEBVIEW]). It presents the same data with more room: the headline totals and threshold verdict, then a sortable per-folder / per-file table of duplication percentages. It renders from the `report/snapshot` the panel host already pushes — now carrying `metrics.per_file` — so the webview stays dumb and the extension host owns all data shaping ([vsix.md §VSIX-PRINCIPLES](vsix.md#vsix-principles) principle 4).

## [VSIX-WEBVIEW-COVERAGE] Webview coverage gate

The webview bundle (`webview-ui/src` → esbuild → `media/webview/*.js`) is **invisible to the `vscode-test --coverage` c8 pass**, which only measures the extension host under `out/**`. That blind spot is real: bug #254 — a runtime value erased by `import type`, which froze every cluster panel on "No cluster selected." — shipped straight through it because the host-side coverage number never touched the bundle.

This gate closes the blind spot by exercising the real bundle in a real browser and measuring it directly:

- **Coverage build.** `clients/vscode/webview-ui/build.mjs --coverage` emits **unminified** bundles with **inline** sourcemaps. Minified single-line output collapses every statement onto one line, which destroys the line mapping the V8 coverage converter needs; the inline sourcemap travels with the bundle so the converter can map executed ranges back to `webview-ui/src`.
- **Real-browser smoke run.** The Playwright smoke suite (`clients/vscode/scripts/playwright-webview-smoke.spec.ts`) is the same suite that proves the webviews render — one set of interactions, no duplicate rendering harness. It runs through the coverage fixture (`clients/vscode/scripts/webview-coverage-fixture.ts`), which imports `test`/`expect` and wraps the `page` fixture to call `page.coverage.startJSCoverage({ resetOnNavigation: false, reportAnonymousScripts: true })` when `WEBVIEW_COVERAGE=1`. `reportAnonymousScripts` is required because the bundle loads as **inline ESM** via `setContent`, which V8 treats as an anonymous script that Playwright otherwise drops. The fixture keeps only the entry whose source carries a `sourceMappingURL` (the bundle) and writes the raw V8 dumps under `coverage/webview/raw/`.
- **Convert + merge.** `clients/vscode/scripts/webview-coverage.mjs` reads every raw V8 dump, converts each with `v8-to-istanbul` (anchored at the bundle's own directory so relative sourcemap sources resolve), keeps only files under `webview-ui/src`, and merges them with `istanbul-lib-coverage` into one map — yielding per-file and total line coverage written to `coverage/webview/coverage-summary.json`.
- **The floor.** The threshold lives in the repo-root `coverage-thresholds.json` at `.vsix.webview_threshold` (currently **95**), enforced with the same **+1% rounding slack** as `check-coverage.mjs` and the Rust `_coverage_check`: the run fails if `measured + 1.0 < threshold`. It also fails hard if **zero** `webview-ui/src` coverage was mapped, so a broken harness can never pass by default. Like every other floor in `coverage-thresholds.json`, it ratchets upward only.
- **Wiring.** `make ci` runs the gate via the `_vsix-webview-coverage` target (after `_vsix-coverage`); `npm run coverage:webview` runs it locally. After measuring, the run rebuilds the **production (minified, external-sourcemap)** bundle so a coverage run never leaves the unminified inline-sourcemap output staged for packaging.

Acceptance is the gate itself: a webview-only regression like #254 lands a coverage drop (or a smoke-suite failure) instead of shipping silently.
