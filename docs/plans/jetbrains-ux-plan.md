# JetBrains Native UX Plan — VSIX feature parity

## Scope

The JetBrains plugin ships as **one LSP4IJ artifact** (`deslop-lsp4ij`) covering
Android Studio, IntelliJ Community, and — with LSP4IJ installed — Rider / Ultimate.
It must reach **full feature parity with the VS Code extension** (`clients/vscode`).
Today it does not: it is a thin LSP bridge plus an HTML-report tool window. This
file is the authoritative gap list so the next pass has the complete checklist.

The plugin must stay thin. Kotlin owns editor integration, settings UI, tool windows, and context-menu actions only. Clone detection, mass ranking, report schema, mass severity, exact pair comparison, and embedding-model discovery stay in Rust behind the LSP custom methods (`deslop/reportGet`, `deslop/comparePair`, `deslop/embeddingListModels`, …). Kotlin never infers or selects pair evidence for a cluster. Do not port the VSIX webviews and do not parse hover markdown to recover structured data.

## Landed

- Single LSP4IJ artifact; the native-LSP (`deslop-ultimate`) build was removed.
- `since-build = 243` (IntelliJ 2024.3 / Android Studio Meerkat) so the plugin
  actually loads in shipping Android Studio. Compiled against IDEA Community 2024.3.
- Editor diagnostics + **Problems** entries (`source = "deslop"`) and the LSP4IJ
  **Language Servers** status surface, via the LSP — parity with the VSIX
  diagnostics channel.
- **Deslop** tool window (right stripe) hosting the engine HTML report in a JCEF
  browser, with a toolbar **Refresh** and the `Tools → Deslop: Open HTML Report`
  action, both behind the shared `DeslopReportRenderer` seam.
- `DeslopPluginDescriptorTest` pins the tool window / service / action / server
  registrations so the visible surfaces can't silently disappear.

## Feature parity matrix (VSIX → IntelliJ)

`deslop-lsp` already serves cluster membership and mass plus endpoint-keyed pair comparisons over LSP; the gaps below are all **client-side Kotlin UX** the VSIX has and the plugin does not.

| VSIX feature | VSIX surface | IntelliJ status | Notes / source of truth |
|---|---|---|---|
| Clone diagnostics (underline + Problems) | LSP diagnostics | **Done** | via LSP4IJ |
| HTML / Duplication report | `deslop.openHtmlReport` webview | **Done** (tool window) | JCEF; consumes `renderHtmlReport` |
| Code lens cycling (`jumpToNextOccurrence`) | LSP code lens | **Partial — verify** | LSP4IJ renders code lens; confirm the command round-trips |
| **Copy Context For AI** + copy commands (`copyContextForAI`, `copyHumanLocation`, `copyClusterLocations`, `copySourceSnippet`) | editor/tree context menu | **Missing** | **Hard rule** (CLAUDE.md: every context menu must offer "Copy Context For AI"). Highest-priority gap. |
| Top Offenders tree + grouping (cluster/file/folder), impact/path sort, split-by-language, expand/collapse/refresh toolbar | `deslop.topOffenders` view | **Missing** | consume `deslop/reportGet`; [vsix.md §VSIX-TOP-OFFENDERS-*](../specs/vsix.md) |
| Duplication metrics panel | `deslop.metrics` view | **Missing** | `RepoMetrics.per_file`, [vsix.md §VSIX-METRICS-PANEL](../specs/vsix.md#vsix-metrics-panel) |
| Session panel (active model, cache stats, files analysed, state) | `deslop.session` view | **Missing** | `deslop/analysisState` |
| Rich cluster hover (id, mass, rank, occurrences) | client `clusterHoverProvider` | **Missing** | consume membership and mass only; never infer pair evidence |
| Explicit occurrence-pair compare | `deslop.comparePair` | **Missing** | require two selected endpoints, use the IDE diff viewer, render the endpoint-keyed evidence compactly |
| Go to occurrence / open canonical / reveal in explorer / open all occurrences | commands | **Missing** | navigation actions over report data |
| Open worst cluster / open cluster / cluster details | commands | **Missing** | |
| Embedding model picker | `deslop.pickEmbeddingModel` QuickPick | **Missing** | native popup over `deslop/embeddingListModels` + `deslop/embeddingSetModel` |
| Settings UI (the ~18 `deslop.*` settings) | VS Code settings | **Missing** | `DeslopSettings` persists the contract but there is no IDE settings page; launch is `.deslop.toml`-driven, embeddings forced off |
| Live bubble | `deslop.liveBubble.*` | **Missing** | |
| Severity colours (mass rank band) | client `severity.ts` | **Missing** | gutter/lens/tree colour channel |
| Selected-cluster synchronisation | client signal | **Missing** | lock editor caret ↔ tree ↔ detail |
| Toggle all code lenses / schema doc / reveal CPU report / reveal active binary | commands | **Missing** | low priority |

## TODO (priority order)

- [ ] **Copy Context For AI** context-menu action (+ the copy-location/snippet
      family) on the editor and any tree rows — closes the CLAUDE.md hard-rule gap.
- [ ] `Duplicate Clusters` tool window (or extra tabs on **Deslop**): Top Offenders
      tab consuming `deslop/reportGet` order, with grouping / sort / split-by-language
      / collapse-expand-refresh toolbar parity.
- [ ] Duplication (metrics) tab and Session tab.
- [ ] Navigation from rows to source occurrences; neutral cluster detail view containing identity, occurrence membership, canonical extent, mass, and rank only.
- [ ] Native rich hover fed by a custom LSP method (no markdown parsing).
- [ ] Explicit two-occurrence compare via the IDE diff viewer; never infer comparison endpoints from a cluster.
- [ ] Native embedding model picker backed by `deslop/embeddingListModels`;
      persist via the shared settings contract + `deslop/embeddingSetModel`;
      surface refresh progress without blocking typing.
- [ ] IDE settings page exposing the `deslop.*` contract (parity with VSIX config).
- [ ] Severity colour channel + selected-cluster synchronisation.
- [ ] Bump the Gradle `jvmToolchain` to 21 to match the 2024.3 platform's preferred
      Java (currently 17 — builds and runs, but `verifyPluginProjectConfiguration`
      warns). Requires a JDK 21 on dev machines / CI (CI already uses 21).
