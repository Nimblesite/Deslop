# VSIX — the VS Code extension

The VSIX is the **polished reference client** for the Deslop daemon. Every other editor can wire up the LSP ([lsp.md](lsp.md)) and get a competent experience; the VSIX is where we prove what a genuinely beautiful duplication-surfacing UI looks like.

Distribution: one platform-specific `.vsix` per VS Code target attached to each GitHub Release — see [.github/workflows/release.yml](../../.github/workflows/release.yml). Extension id: `nimblesite.deslop-live`. Install via `code --install-extension deslop-live-X.Y.Z-<target>.vsix`, or from the Marketplace/OpenVSX once we set up publisher accounts.

### [VSIX-PRINCIPLES] UX principles

1. **In your face the moment you duplicate.** When the user types code that matches an existing cluster, the editor tells them **immediately** via the live-bubble ([VSIX-LIVE-BUBBLE]) — not on save, not on CI, not in a panel they have to open. This is the product's defining moment. Every other UX decision is subordinate to making it land cleanly.
2. **Silent when the code is clean.** If there are no clusters overlapping the current file, no UI elements appear on that file. The activity bar badge disappears. The editor is untouched. Loudness is reserved for real duplication.
3. **The worst offender is always one click away.** The activity bar icon always jumps to cluster `#1` of the live report. The user never navigates through menus to find duplication hotspots.
4. **Every surface speaks the same schema.** Tree view, hover, code lens, status bar, bubble, webview — all render the same `Report` the JSON file carries. Humans and agents read the same truth.
5. **Never block an edit.** The daemon is a sidecar; analysis runs asynchronously; UI updates ride notifications. A typing pause of 250 ms triggers re-analysis, not every keystroke.
6. **Legible, not decorative.** No animated icons, no gradient flourishes that obscure content. Density is high but scannable — the user is hunting for duplication, not admiring chrome. Severity is communicated by colour ramp + glyph, nothing else.
7. **Human-readable before machine-readable.** The VSIX is for developers working in an editor, so ordinary UI labels use friendly file names, line numbers, and columns. Byte offsets are valid in the JSON/AI report and wire schema, but the tree, webviews, bubbles, hovers, status bar, and command titles must not expose raw byte markers as the primary location text.
8. **Reactive end-to-end.** Every surface — tree, decorations, bubble, code lens, status bar, hovers, webviews, badges — derives from `@preact/signals` over the single [VSIX-STATE] store. `deslop/reportChanged` updates settle in one microtask across every surface. Stale UI is a correctness bug per [VSIX-REACTIVITY-INVARIANT], not a polish issue.

### [VSIX-LIVE-BUBBLE] Live duplication bubble — the flagship UX

This is the feature. The VSIX is the first tool that tells a developer **"you are duplicating code right now"** while the code is still under their cursor. Every other surface (tree view, webview, code lens, status bar) is supporting cast; the bubble is the lead.

**When it fires.**
After every coalesced buffer edit ([LIVE-WATCHER] debounce = 250 ms), the VSIX issues `duplicates/findSimilar` on the range the user most recently touched. If a cluster comes back with fused score ≥ `FUSED_THRESHOLD` (0.85, same as the offline report), the bubble appears anchored to the bottom-right of the duplicated range. If nothing matches, no bubble — silence is the signal that the code is novel.

**What it looks like.**
A compact floating widget (VS Code `InlayHint` + `Webview`-backed overlay, rendered by a single `DecorationType` whose `after.contentText` is an HTML-safe Unicode glyph, with a hover-triggered richer webview for detail). Anatomy, from left to right:

- **Severity dot** — colour mapped to the cluster's resolved severity per [LSP-SEVERITY-BUCKET]: red (`Error`), amber (`Warning`), blue (`Information`), grey (`Hint`). Defaults: `Identical` → red, all others → amber. Clusters whose bucket is configured to `"none"` are never shown as a bubble.
- **Short verdict** — one of: `DUPLICATE` (structural = 1.0), `NEAR-MISS` (token jaccard ≥ 0.90, structural < 1.0), `SEMANTIC MATCH` (embedding cos ≥ 0.90). One word, uppercase, so the user sees it without reading.
- **Count + location** — `× 4 • UserService.cs:230`. The canonical occurrence of the cluster, linkified to jump on click.
- **Signal strip** — three 8-pixel bars for structural / jaccard / embedding. Bright = high, dim = low. Lets the user distinguish "identical copy" from "semantic near-miss" at a glance.
- **Action chevron** — click expands the bubble into a webview-backed card with interpretation, all occurrences, action hints, and a `Compare` button that opens VS Code's diff view against the canonical occurrence.

**How it's rendered.**
VS Code doesn't give us a true floating tooltip over a specific range, so the bubble uses the layering documented in the VS Code extension cookbook:

- Primary: a `TextEditorDecorationType` with `after.contentText` attached to the end of the duplicated range's last line, carrying the severity dot + verdict + count. This is the always-visible indicator — shows up inline, like GitHub Copilot's ghost text but for duplication.
- Secondary: an `InlayHint` on the same range, carrying the signal strip. Inlay hints render in a different visual register than ghost text; the combination gives the user a two-part cue (verdict inline, signal bars on the hint line).
- Tertiary: hover over either surface opens the LSP hover ([LSP-HOVER]) for full detail.

No native floating bubble is possible in current VS Code APIs without a custom webview overlay — and a webview overlay would steal focus. The decoration + inlay combination is the closest legal approximation, reads as a single "bubble" to the user, and never steals the caret.

**Ghost-line mode (opt-in, `deslop.liveBubble.mode = "ghost"`).**
For users who want a tighter callout, ghost-line mode renders the bubble on a **phantom line inserted below the duplicated range**, using VS Code's `CodeLens` API with a custom-styled title. The phantom line is visually distinct from the real buffer (dimmed background, italic). It never modifies the buffer; scroll behaviour matches code lenses. This is the closest thing to "a speech bubble pointing at the duplicate" that VS Code natively supports.

**Cooldown + budget.**
- Bubbles don't flicker: once shown for a range, the same cluster on the same range stays bubbled until the user moves out, even if debounce re-fires. Cluster stability across re-analyses ([LIVE-DELTA]) makes this trivial — same id, same bubble.
- The live-bubble query has a 250 ms budget on the daemon side ([LIVE-PERF-BUDGETS]); if it misses, the bubble is skipped for that edit cycle and will try again on the next debounce. No stale bubbles.
- At most one bubble visible per editor at a time (the worst-weight cluster overlapping the most-recently-edited range). Users reading a report don't need N bubbles competing for attention; the tree view ([VSIX-ACTIVITY-BAR]) shows all of them.

**Dismissal.**
- `Escape` dismisses the bubble until the next edit re-triggers.
- Clicking a `Dismiss for this cluster` action in the expanded card suppresses that cluster id for the session. Session-scoped, never persisted — the next day, the duplication is real again and we say so.
- `deslop.liveBubble.enabled = false` turns the bubble off globally for users who want the rest of the VSIX without the in-your-face moment. Off-by-default is **not** a setting we ship — silence-when-clean already gives users a tolerable floor; the bubble is on from the first install.

**Why this is the headline.**
No competitor ([competitors.md](competitors.md)) tells a developer about duplication at typing time. PMD CPD runs on CI. jscpd runs on CI. SonarLint flags on save, after the thought is already committed. JetBrains' inspection flashes a Problems panel entry you have to look for. Deslop *shows the duplicate to the developer inside the IDE, inline with their cursor, as they type the thing*. First tool to do it. Called out on the Marketplace listing, the README, and every demo GIF.

### [VSIX-BUNDLE] Extension bundle

The VSIX ships:

- The extension TypeScript (`clients/vscode/src/extension.ts`, under 500 lines per CLAUDE.md; UI logic split into `webview/`, `tree/`, `decorations/`, `commands/`).
- A pre-built `deslop` CLI binary per platform, colocated with the server binaries for process-local PATH exposure after verification.
- A pre-built `deslop-lsp` binary per platform (darwin-arm64, darwin-x64, linux-x64, linux-arm64, win32-x64). Download-on-first-activate is **not** acceptable — the extension either works offline immediately or it doesn't install.
- A pre-built `deslop-mcp` binary per platform, colocated, registered with any MCP-aware VS Code host (Claude Code, Copilot Chat with MCP, etc.) via the extension's MCP contribution point.
- `shipwright.json` at the VSIX extension root. The manifest is the package authority for required executable components, expected versions, host startup checks, and allowed native files under `bin/<platform>/`.
- The shared `deslop-report-view` webview bundle (preact + no external CSS framework; see [VSIX-WEBVIEW]).
- The extension's own `schema_doc.md` pulled from `docs/specs/REPORTING-CONTEXT.md` at build time — the same `include_str!` content the report embeds. Drift is impossible.

### [VSIX-BINARY-VERSIONING] Binary versioning + PATH exposure

**One version, one zip.** The bundled `deslop`, `deslop-lsp`, and `deslop-mcp` binaries ship inside the VSIX and are versioned **lock-step** with the extension. Version `X.Y.Z` of the VSIX always contains version `X.Y.Z` of the binaries — no independent bumps, no "works with any binary ≥ …" fuzziness. The publish workflow ([VSIX-PUBLISH]) builds the Rust workspace and the TypeScript extension in the same job so the binaries that leave CI are the ones the Marketplace listing installs. No post-install downloads, no network dependency at activation time, no drift between the bundled binary and the wire contract the extension speaks.

**Manifest-backed activation.** On activation, the extension loads `shipwright.json` from the extension root and reads `hosts.vscode.activationVerifies`. Required components, currently `deslop-lsp` and `deslop-mcp`, must be resolved and version-checked before the LSP client, MCP integration, file watchers, workspace parsing, or live analysis starts. The manifest's `expectedVersion`, component id, binary name, platform map, and required flag are authoritative; `package.json` must not become a second source of truth for executable compatibility.

**Overrides are resolver inputs.** Resolution is manifest-driven: the resolver tries each source the component declares, in order, and verifies every candidate against the manifest before use. Deslop's shipped manifest declares exactly two sources for `deslop-lsp` and `deslop-mcp`:

1. `deslop.lspPath` / `deslop.mcpPath` — an absolute path the user sets deliberately (the override).
2. The binary bundled under `${extensionPath}/bin/${platform}/`.

Nothing else is consulted — no `PATH` search, no environment variable, no cargo-bin, package-manager, or download fallback. The extension runs the binary it shipped with, or the one the user explicitly pointed at, or activation fails loudly. An override that resolves to the wrong component, wrong version, or a non-executable file blocks activation with a visible error instead of silently falling through to the bundle. A bundled binary mismatch blocks activation because the package itself is corrupt. Missing required binaries block activation; optional components may degrade only when the manifest marks them optional. The resolver library additionally supports manifest-declared environment inputs (`env.pathVar` / `env.dirVar`) for other hosts, but Deslop's manifest does not enable them.

If the verified bundled directory is selected, the extension may prepend that directory to the current VS Code process's `PATH` so integrated terminals, task runners, and Run/Debug can invoke `deslop` directly. This change is process-local — the extension never modifies `~/.bashrc`, `~/.zshrc`, PowerShell profiles, or `launchctl` environment. A user who wants the CLI available outside VS Code should install via `brew install nimblesite/tap/deslop` (Homebrew) or `scoop install deslop` (Scoop, after adding the [Nimblesite bucket](https://github.com/Nimblesite/scoop-bucket)); the VSIX does not try to be a system package manager.

Activation binaries are resolved once per session; a `Deslop: Reveal Active Binary` command (under [VSIX-COMMANDS]) shows the path, source, component id, and version that were accepted so a user debugging a mismatch can see the resolver result without reading logs. Package verification is covered by [DEPLOY-VSIX-PACKAGE] and release gates by [DEPLOY-CI-GATES].

#### [VSIX-BUNDLED-BINARY-TESTS] Extension tests use the bundle

VSIX tests must exercise the same binary layout the installed extension uses:
`${extensionPath}/bin/${platform}/`. Test configuration must not point
`DESLOP_BINARY_DIR`, `DESLOP_LSP_PATH`, or `DESLOP_MCP_PATH` at
`target/release`, `~/.cargo/bin`, Homebrew, Scoop, PATH, or any other external
install. The Makefile stages the release binaries into the extension bundle
before `vsix-test`, `vsix-coverage`, and `vsix-test-ollama` run, then clears
the override environment variables in the VS Code test host. Runtime uses the
absolute bundled paths in `${extensionPath}/bin/${platform}/` and does not
prepend that directory to PATH.

Before test entry points run, installed `deslop`, `deslop-lsp`, and
`deslop-mcp` binaries are removed from the cargo install path and the build
fails if any of those commands still resolve on `PATH`. A passing extension
test must prove `resolvedLsp.source = "bundled"` and
`resolvedMcp.source = "bundled"` so a stale machine-level install cannot hide a
broken VSIX package.

### [VSIX-ACTIVATION] Activation

Activation events:

- `onLanguage:csharp`, `onLanguage:rust`, `onLanguage:python` — mirror the supported language set; extending requires a VSIX rebuild when `deslop-core` adds a language.
- `onCommand:deslop.openReport` — cold activation when the user explicitly asks for the report.
- `workspaceContains:**/*.cs`, `**/*.rs`, `**/*.py` — pre-warm the LSP on project open.

On activation: load `shipwright.json`, verify all required VS Code activation components from `hosts.vscode.activationVerifies`, then spawn the resolved `deslop-lsp` binary rooted at the first workspace folder and wire up the VSIX UI surfaces below. Multi-root workspaces get one LSP process per root, but binary verification is per extension activation session, not per workspace root.

### [VSIX-ACTIVITY-BAR] Activity bar + tree view

A dedicated activity bar icon (a stylised "dd" mark, the same one used in the Marketplace listing) opens the **Duplicate Clusters** view container. Inside:

- **Top Offenders** tree — see [VSIX-TOP-OFFENDERS-GROUPING] for the cluster / file / folder grouping modes, [VSIX-TOP-OFFENDERS-SORT] for the impact-vs-path sort axis, and [VSIX-TOP-OFFENDERS-LANGUAGE-GROUP] for the optional per-language split. In every mode, cluster rows show:
  - **Cluster slug** as the leading element of the bold label ([VSIX-TOP-OFFENDERS-CLUSTER-ID]) — the first 7 hex chars of `cluster.id`, identical to the slug used by the LSP hover bubble. The slug is stable across runs.
  - Severity dot ([LSP-SEVERITY]) and short interpretation (e.g. `Type-1 exact · 6 copies · 320 nodes`).
  - Grey description tail: `rank #N · N copies`. The literal word **rank** appears on every surface that shows `#N` (description, tooltip, accessibility label, copy-for-AI) so neither humans nor AI agents confuse the volatile rank for the stable id ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]).
  - Full 16-hex `cluster.id` is preserved in the tooltip (`cluster id: \`...\``) and in every command argument; only the visible label is shortened.
  - Children: one node per occurrence, shown as `path:line:column` for humans. Clicking opens the file at that occurrence's file, line, and column. Raw byte ranges remain available to AI/report consumers but are not rendered in the normal tree label.
- **Duplication** panel ([VSIX-METRICS-PANEL]) — the codebase duplication summary that replaces the former Focused File tree: a headline duplication score over the whole corpus, then a per-folder → per-file breakdown of how much of each is duplicated. The headline opens the full [VSIX-METRICS-REPORT] webview.
- **Session** panel — compact footer with: active embedding model (linkable, opens the picker), `cache_stats`, `files_analysed`, daemon state (`idle` / `running`).

Tree refresh is driven by `deslop/reportChanged`; the webview uses the same notification to bump its own state.

#### [VSIX-TOP-OFFENDERS-GROUPING] Cluster / File / Folder grouping modes

The Top Offenders tree exposes three grouping modes that change the tree shape and what counts as a root. Two orthogonal axes compose on top of every mode: the sort order ([VSIX-TOP-OFFENDERS-SORT]) and the per-language split ([VSIX-TOP-OFFENDERS-LANGUAGE-GROUP]).

The mode is persisted via the `deslop.topOffenders.groupBy` setting (`"cluster"` | `"file"` | `"folder"`, default `"cluster"`). VS Code's standard user→workspace precedence applies: a workspace value pinned in `.vscode/settings.json` overrides the user-level default, so a repo team can lock a lens for everyone working in that repo while individuals keep their own machine-wide default elsewhere. Unknown / missing values fall back to `"cluster"` — never panic.

A view-title toggle in the Top Offenders header cycles modes. The toggle writes to the workspace configuration target so the choice persists per-repo. Cold-start respects the persisted value — there is no flash-of-default render. The toolbar also carries collapse / expand / refresh actions ([VSIX-TOP-OFFENDERS-TOOLBAR]) because folder mode can nest deeply.

#### [VSIX-TOP-OFFENDERS-CLUSTER-MODE] Cluster mode (default)

Root rows are clusters in the report's worst-first order. No file-keyed reordering. Each root expands directly to its occurrence leaves. The row label keeps the form `<slug> <severity-dot> <plainTitle> · <file>` because the file is not implicit from any parent. The slug is the cluster's stable 7-hex prefix ([VSIX-TOP-OFFENDERS-CLUSTER-ID]); the row's volatile rank lives in the grey description as `rank #N · N copies`, never in the bold label.

#### [VSIX-TOP-OFFENDERS-FILE-MODE] File mode

Root rows are files. A file's child nodes are bucket groups, one per [CLONE-BUCKETS-DUAL-LABEL] bucket present in that file (no empty groups). Each bucket group expands to its clusters; each cluster expands to its occurrence leaves.

Files sort by max cluster weight desc (primary — "worst offender first" applied to the file's most-painful cluster), with sum-of-weights desc as the tiebreaker and `localeCompare` of the path as the final stable key. Bucket groups within a file sort by max cluster weight desc. Clusters within a bucket group sort by weight desc.

Cluster row labels in file mode drop the trailing `· <file>` suffix because the parent file row already shows it. The bold label still leads with the cluster slug ([VSIX-TOP-OFFENDERS-CLUSTER-ID]); the rank still lives in the grey description tail. The tooltip is mode-invariant — it always carries the full path so the AI-scrapable hover surface stays stable.

#### [VSIX-TOP-OFFENDERS-FOLDER-MODE] Folder mode

Root rows are the top-level folders of the workspace; each folder is a real tree that expands into its child folders and, at the leaves, the files that contain clusters. A file leaf behaves exactly like a file-mode root ([VSIX-TOP-OFFENDERS-FILE-MODE]): it expands to the bucket groups present in that file, then to clusters, then to occurrences. Single-child intermediate folders are path-compressed into their nearest branching ancestor so a deep `crates/deslop-core/src/...` chain renders as one row, not five.

Because each cluster is single-language ([CONFIG-CROSS-LANGUAGE]) and languages overwhelmingly live in separate directory trees, folder mode already separates most languages without an explicit language split. Each folder row's grey description carries its rolled-up worst weight and the count of files beneath it that contain duplication.

Folder rows, their child folders, and the files within them sort per [VSIX-TOP-OFFENDERS-SORT]. The default — impact — sorts by max cluster weight desc (a folder's worst cluster), sum-of-weights desc tiebreaker, then path `localeCompare`. Global rank ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]) is unchanged: rank #1 is still the repo's worst cluster, wherever it sits in the tree.

#### [VSIX-TOP-OFFENDERS-CLUSTER-ID] Cluster slug leads the row, rank never does

The bold label on every cluster row leads with the cluster slug — the first 7 hex chars of `cluster.id`. The slug is stable across runs, deltas, snapshots, and grouping modes; it is the single identifier humans and AI agents can quote between sessions. The same slug is used by the LSP hover bubble ([VSIX-HOVER-SHARED]), via the shared `clusterSlug()` helper, so the UI never shows two different short forms of the same id.

The volatile rank (`#N`) is never the leading element of the label. Rendering rank as if it were an id has shipped two incidents (the LSP hover regression tracked in `docs/plans/cluster-slug-vs-rank.md`, and the Top Offenders tree regression that produced this section). Both humans and — critically — AI agents reading the rendered tree treat the leading element as the row's identity; using rank there means the "identity" changes on every snapshot, which silently breaks cross-message references in agent transcripts.

Rules:

1. The bold label **must** start with `<slug> <severity-dot> <plainTitle>` (or `<slug> <severity-dot> <plainTitle> · <file>` in cluster mode). No `#N` prefix, anywhere.
2. The grey description **must** carry `rank #N · N copies`. The literal word **rank** must appear before `#N` — never bare.
3. Every other surface that mentions `#N` — tooltip, accessibility label, copy-for-AI payload, occurrence-tooltip parent reference — must use the literal word **rank**. AI consumers parse for the word; bare `#N` is forbidden.
4. The full 16-hex `cluster.id` is preserved in the tooltip (`cluster id: \`<id>\``), in every command argument (`deslop.openCluster`, `deslop.compareWithCanonical`, …), and in the AI copy payloads. Display truncation is presentation-only.

#### [VSIX-TOP-OFFENDERS-RANK-GLOBAL] Global rank #N

The rank #N attached to a cluster row is the cluster's position in the report's worst-first list. It does **not** change between modes, and it is **not** re-numbered within a file or within a bucket group. This keeps cross-file impact comparable at a glance — rank #1 is always the worst cluster in the repo, regardless of which lens the user picked.

Rank lives in the grey description, not the bold label. The bold label leads with the stable cluster slug ([VSIX-TOP-OFFENDERS-CLUSTER-ID]).

#### [VSIX-TOP-OFFENDERS-CATEGORY-COLORS] Top Offenders category metadata

Top Offenders root rows expose the clone bucket with stable theme-aware icon colour metadata. `Identical code`, `Nearly identical code`, `Loosely similar code`, and `Same behavior, different code` must not share the same colour token.

Colour is never the only signal. The category text remains in the visible label, the tooltip carries the hybrid taxonomy label, and the accessibility label includes the category and representative file.

#### [VSIX-TOP-OFFENDERS-SORT] Sort axis (impact vs path)

Sibling order is an axis orthogonal to the grouping mode, persisted via `deslop.topOffenders.sortBy` (`"impact"` | `"path"`, default `"impact"`). A view-title toggle flips it, writing to the workspace target like the grouping toggle; unknown / missing values fall back to `"impact"`.

- **impact** (default) — worst-offender first: clusters by weight desc, files and folders by max cluster weight desc (sum-of-weights desc, then path). This is the product's "worst first" promise ([VSIX-PRINCIPLES] principle 3). Within a cluster, occurrences keep the report's canonical order (canonical occurrence first).
- **path** — alphabetical by path (`localeCompare`), so a flat file list, a folder tree, or the occurrences inside a cluster read in filesystem order for navigation.

The sort axis reorders the **display order in every grouping mode** — cluster, file, and folder roots and their descendants, **plus the occurrences inside a cluster**. The global rank #N is read from the report's worst-first order and is never renumbered ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]), so a path-sorted cluster row still shows its true `rank #N`. Sorting is presentation-only — it never re-fetches or re-analyses ([VSIX-VIEW-STATE-UI-ONLY]).

#### [VSIX-TOP-OFFENDERS-LANGUAGE-GROUP] Per-language split

`deslop.topOffenders.splitByLanguage` (boolean, default `false`) adds an orthogonal outer grouping dimension. When on, top-level rows are one language group per language present, each containing the full cluster / file / folder subtree for that language; when off, languages interleave in one worst-first list (today's behaviour). Folder mode already separates most languages structurally, so the split is most useful with cluster or flat-file grouping in a polyglot tree where one directory mixes languages.

Language is derived from each cluster's representative occurrence path via the shared `languageForPath()` helper, which mirrors the core `language_for_path()` ([OUTPUT-HUMAN-HTML]). A single-language workspace renders exactly one group, so the split adds no noise. Global rank is preserved across and within groups ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]); a language group's description carries its worst weight and cluster count. The setting persists to the workspace target and exposes a view-title toggle like the other two axes.

#### [VSIX-VIEW-STATE-UI-ONLY] Grouping, sorting, and filtering are UI-only

Grouping ([VSIX-TOP-OFFENDERS-GROUPING]), sorting ([VSIX-TOP-OFFENDERS-SORT]), the per-language split ([VSIX-TOP-OFFENDERS-LANGUAGE-GROUP]), Expand All / Collapse All ([VSIX-TOP-OFFENDERS-TOOLBAR]), and the dirty-file projection ([VSIX-STATE-DIRTY]) are **pure presentation transforms over the report already held in the [VSIX-STATE] store**. This is non-negotiable:

- **They never reach the engine.** Flipping a sort axis, changing the grouping mode, expanding the tree, or toggling the language split does **not** send an LSP request, trigger a re-scan, or invalidate the on-disk cache. The provider re-reads the persisted view-state and rebuilds its rows from `store.current.visibleReport` on the next `onDidChangeTreeData` fire. No round-trip means no spinner and no latency — the reorder is instant and synchronous.
- **They never mutate the canonical report.** The only writers of the canonical report are `setSnapshot` / `applyDelta`, driven by `deslop/reportChanged` — i.e. an actual **file change** picked up by the file watcher, never a view toggle.

The single deliberate exception is the **Refresh** button (`deslop.refresh` → `deslop/refreshReport`): an explicit, user-initiated force of a full re-analysis. Everything else in the toolbar is local. A view toggle that triggers a re-analysis is a correctness bug, not a feature.

#### [VSIX-TOP-OFFENDERS-TOOLBAR] Collapse / expand / refresh actions

After the grouping/sort/split toggles (`navigation@1`–`@3`), the Top Offenders title bar carries three icon actions, **adjacent and in order**: **Expand All** (`$(expand-all)`, `deslop.topOffenders.expandAll`, `navigation@4`), **Collapse All** (`$(collapse-all)`, `deslop.topOffenders.collapseAll`, `navigation@5`), and **Refresh** (`$(refresh)`, `deslop.refresh`, `navigation@6`).

Expand All and Collapse All are **provider-driven** (`TopOffendersProvider.setBulkExpansion`): the provider rewrites the collapsible state it returns from `getTreeItem` and fires `onDidChangeTreeData`, so the whole tree expands or collapses **in one shot at every level** — reliable in cluster, file, and folder mode, not just the first level (which is why we do not use the one-level `TreeView.reveal({ expand: true })` or rely on the built-in `showCollapseAll` button, which would render a second, detached collapse action). The override is presentation-only ([VSIX-VIEW-STATE-UI-ONLY]) and is released on the next data change. **Refresh** is the one toolbar action that reaches the engine — it forces a full workspace re-scan. The Session and Duplication panels keep VS Code's built-in Collapse All for consistency.

#### [VSIX-METRICS-PANEL] Duplication panel

The **Duplication** tree (`deslop.metrics`) replaces the former Focused File panel and answers one question at a glance: *how duplicated is this codebase, and where?* It renders from the last analysed snapshot's repo metrics (`Report.metrics`, [METRICS-REPO]) and refreshes on `deslop/reportChanged`.

- **Headline row** — the overall duplication percentage (`duplication_percent`) as the bold label, with the grey description carrying `analysed_loc`, `duplicated_loc`, `clusters_total`, and `duplicated_files` in plain language. When `metrics.threshold.breached`, the row shows a warning glyph and names the gate it crossed. Activating the row opens the [VSIX-METRICS-REPORT] webview.
- **Per-folder → per-file breakdown** — below the headline, a tree of folders (rolled up from `metrics.per_file` by path prefix, summing numerator and denominator so each folder percentage is exact) expanding to the files within, each row showing its own duplication percentage in the grey description. Worst-first by percentage, path `localeCompare` tiebreaker; rows with zero duplication are omitted from display. Single-child folder chains are path-compressed, matching folder mode.
- **Clean / empty** — when there is no duplication, the panel shows a single "No duplication detected" row, honouring [VSIX-PRINCIPLES] principle 2.

#### [VSIX-METRICS-REPORT] Duplication report webview

Activating the headline opens a webview (`deslop.openDuplicationReport`) styled like the existing report webview ([VSIX-REPORT-WEBVIEW]). It presents the same data with more room: the headline totals and threshold verdict, then a sortable per-folder / per-file table of duplication percentages. It renders from the `report/snapshot` the panel host already pushes — now carrying `metrics.per_file` — so the webview stays dumb and the extension host owns all data shaping ([VSIX-PRINCIPLES] principle 4).

### [VSIX-CODE-LENS] Code lens

The LSP's code lens ([LSP-CODE-LENS]) is the content source. The VSIX styles it with the same severity colour ramp so inline clone markers match the tree view.

Each lens has three actions in its command array:

- **"Jump"** — cycles `textDocument/definition` through remaining occurrences.
- **"Compare"** — opens VS Code's diff view between this occurrence and the canonical occurrence of the cluster.
- **"Open cluster"** — opens the webview ([VSIX-WEBVIEW]) pinned to this cluster.

The lens is suppressed for clusters whose bucket is configured to `"none"` severity ([LSP-SEVERITY-BUCKET]) or that fall below the configured percentile floor ([LSP-SEVERITY-PERCENTILE]). Users can toggle via `deslop.showAllLenses` (off by default — this is the silent-when-clean principle in action).

### [VSIX-DECORATIONS] Editor decorations

Occurrences in the active editor get a subtle gutter decoration (a thin coloured bar, severity-mapped) and a 1-pixel underline on the clone range. Hover over the underline reveals the full cluster detail via the LSP hover provider.

No background highlighting, no border boxes, no emoji markers in the gutter. The decoration is visible at a glance but doesn't fight with any existing theme.

### [VSIX-STATE] Centralised state store

**All VSIX state lives in one place.** The extension owns a single in-process state container — the `ReportStore` — that every surface (tree, decorations, bubble, webviews, status bar, picker) reads from. Nothing renders from ad-hoc locals, nothing caches a parallel copy of the report, nothing keeps a "last-known" snapshot on the side. One truth, one listener tree, one invalidation path. This mirrors the `crates/deslop-core/src/state.rs` rule on the Rust side: centralised state is the contract, not an implementation detail.

Rules that fall out of that:

- The LSP's `deslop/reportChanged` notification is the **only** writer of the current report snapshot. The tree view does not call `reportGet` on its own, nor does the webview, nor the status bar — they all observe the store.
- Settings changes route through the LSP (`workspace/didChangeConfiguration`) and come back through the same store update path, so there's no "UI thinks the model is nomic-embed-text, LSP is actually using nomic-embed-code" drift window.
- When the LSP reconnects, the store is reset and every surface re-renders from empty — no stale colour on a tree node, no stale verdict on the bubble, no stale percentage on the status bar.
- Disposables are attached to the store, not scattered across provider objects, so extension shutdown tears everything down deterministically.

Centralisation is the enabler for [VSIX-REACTIVITY] below: one store + signal-backed derivations = no stale pixels.

#### [VSIX-STATE-DIRTY] Canonical report vs. visible projection — dirty tracking contract

When a user types into a file that participates in a cluster, two things must happen at once: stale byte ranges must stop driving decorations and tree rows ([#78], [#117]) **and** every command that resolves a cluster by id (`deslop.compareWithCanonical`, `deslop.openCluster`, `deslop.openOccurrence`, `deslop.openCanonicalFile`, `deslop.copyClusterLocations`, the cluster detail webview's row navigation) must continue to find that cluster until the LSP itself retracts it via `deslop/reportChanged`. Both requirements are non-negotiable.

The store therefore exposes **two views of the same report**:

- **Canonical report.** The exact snapshot the LSP last published. Only `deslop/reportChanged` (full snapshot or applied delta) writes it. Editor-side dirty tracking **never** mutates the canonical report. Every command that takes a cluster id, occurrence id, or file path resolves through the canonical report. Lookup by id never returns `undefined` for a cluster the LSP still considers live.
- **Visible projection.** A `computed()` derived from the canonical report and the per-file dirty set. For each file with unsaved edits, occurrences in that file are filtered out of the projection. Clusters whose visible occurrence count drops below two are elided from the projection (a one-copy "top offender" is a contradiction — see [#117]). Tree providers, decorations, hovers, code lenses, the live bubble, the status bar, the activity-bar badge, and the session panel **only ever read the visible projection**. Webviews receive the visible projection through `postMessage`.

`onDidChangeTextDocument` updates the dirty set, never the canonical report. On `didSaveTextDocument` (or external file watcher fire) the file leaves the dirty set; the LSP re-analyses and emits a fresh `deslop/reportChanged` which then updates the canonical report. The visible projection recomputes through the signal graph in the same microtask as either change.

This makes the two requirements compose: the visible projection drops the cluster from the tree the moment the user types (no stale "1 copies"), while the canonical report keeps the cluster id resolvable so `compareWithCanonical` can still diff the canonical (saved) bytes against itself or another peer. When the user saves, the LSP confirms the new shape and both views converge.

Tests must respect this contract too: any e2e test that injects a synthetic edit into a fixture file is responsible for restoring it before the suite ends, otherwise the dirty set leaks across suites and downstream tests see an unexpectedly empty visible projection.

### [VSIX-REACTIVITY] Preact Signals everywhere — every VSIX surface is reactive

**This is a top-level invariant, not a webview implementation detail.** Deslop Live is reactive end-to-end: the file watcher fires, the scheduler re-analyses, the LSP pushes [`deslop/reportChanged`](live.md#live-notifications), the extension applies the delta to the [VSIX-STATE] store — and **every surface that displays report data must update in the same microtask**. Tree providers, decorations, the live bubble, code lenses, the status bar, hovers, the cluster webview, the embedding picker, the activity-bar badge, the session panel: all of them read from `@preact/signals`-backed values derived from the single store. **No surface holds its own cached copy of the report. No surface schedules its own refresh independent of a signal change.**

`@preact/signals-core` is a workspace dependency available to **both** `clients/vscode/src/**` (extension host) and `clients/vscode/webview-ui/**` (webview) — it is *not* limited to webview UI. The extension host owns the canonical `signal<Report | null>`; webviews receive `postMessage` updates that mirror those signals locally.

Three hard guarantees, applied to every surface (tree included):

1. **Zero stale UI after `deslop/reportChanged`.** The moment the store applies a delta, every dependent signal updates in the same microtask. A cluster that disappeared from the report cannot remain on screen — not in the bubble, not in the tree, not in a hover, not in the gutter, not in a code lens. "Last-known-good" rendering is impossible by construction because there is no imperative render loop that could fall behind.
2. **Deterministic updates.** Signals settle transactionally — batches of updates during one delta application produce a single render across all surfaces. No intermediate flash of a partially-updated tree, no half-updated bubble showing yesterday's signals.
3. **Shared signal graph between extension-host and webview.** Tree providers and decoration managers `effect()` over the same signals the webviews mirror, so a user with the activity-bar tree and a cluster webview open at once sees them update in lock-step.

#### [VSIX-REACTIVITY-TREE] Tree providers are signal-driven

`TopOffendersProvider`, `FocusedFileProvider`, `SessionProvider` — and any future tree — derive their `getChildren` output from the store's signals via a `computed()` view. Their `onDidChangeTreeData` event fires from one place: a `signals.effect()` watching the relevant computed value. **The tree must not call `reportGet` directly, must not maintain a parallel `clusters` array, and must not be refreshed from outside the signal graph.** Removing 500 lines from a watched file fires `deslop/reportChanged` → store applies delta → computed `topOffenders.value` recomputes → `onDidChangeTreeData` fires → VS Code calls `getChildren` → tree shows the new state. Any surface still showing a cluster that no longer exists in `report.clusters` is a correctness bug.

**Re-analysis data retention.** When lifecycle transitions to `"analysing"` and the store already holds a report, every tree panel **keeps rendering the existing report** — it does not replace content with a spinner. The status bar carries the `(analysing…)` indicator; the tree panels are not cleared. A spinner placeholder is only shown when no report exists yet (`"starting"` with no data) or when the LSP has `"failed"`. Once `deslop/reportChanged` fires and the delta is applied, the tree updates atomically to the new state. Blanking the tree during re-analysis is a UX bug that breaks the "LIVE" brand promise.

#### [VSIX-REACTIVITY-DECORATIONS] Decorations and bubble are signal-driven

`DecorationManager` and `LiveBubble` `effect()` over `report` + `selectedClusterId` + `editorVisibleRanges`. When `deslop/reportChanged` removes a cluster, the corresponding decorations and bubbles disappear in the same microtask without an explicit `clear()` call from any handler — the effect re-runs, finds the cluster gone, and the diff drops the decoration set.

#### [VSIX-REACTIVITY-WEBVIEW] Webviews mirror the signal graph

**Webviews are built with Preact + `@preact/signals`, not plain React, not manual `useState` ceremony, not event emitters.** `clients/vscode/webview-ui/src/store.ts` exports the `signal<T>` collection: `report`, `selectedClusterId`, `analysisState`, `filters`, `severityByClusterId` (a `computed` over `report`). The extension process posts `postMessage` updates that the webview handler writes into signals; no other path mutates webview state. Components are function components using `@preact/signals` — `const cluster = selectedCluster.value` — not effects, not refs, not class lifecycle. No direct DOM manipulation, no untyped `any` escapes, no `setTimeout`-driven state. If a piece of UI feels like it needs imperative wiring, it's wrong — fold it into a signal or a computed.

#### [VSIX-REACTIVITY-INVARIANT] Staleness is a correctness bug

**Stale UI is a correctness bug, not a polish bug.** The whole product is "tell the developer they're duplicating right now" — if the tree is showing a cluster that was refuted 300 ms ago, we've broken the brand promise. Concrete acceptance test (E2E, against the real LSP binary): open a fixture workspace with N clusters; assert tree, decorations, and bubble all show N. Edit one of the duplicated files to delete a duplicate. After the [LIVE-WATCHER] debounce window plus one scheduler pass, assert tree, decorations, and bubble all show N − 1 **without any user-initiated refresh**. The test fails if any surface still references the removed cluster id. This invariant is enforced via that E2E and via lint rules in `clients/vscode/eslint.config.mjs` that ban `setTimeout`-driven UI refresh, ad-hoc `reportGet` calls outside the bootstrap path, and `TreeDataProvider` implementations that don't subscribe to a store signal.

### [VSIX-WEBVIEW] Cluster detail webview

Command `deslop.openCluster` opens a webview tab. The tab renders a single cluster with:

- Header: cluster id, rank, weight, size, severity badge, jump-to-next-cluster / jump-to-prev-cluster arrows.
- Interpretation and action hints (the same fields the JSON carries).
- Signal breakdown as four tiny bars: structural, token Jaccard, embedding cosine, fused. Each labelled with its numeric value to two decimals.
- One collapsible panel per occurrence, each containing:
  - File path plus human position (`line:column`), clickable to open the file at that exact editor position.
  - Line-numbered, syntax-highlighted source snippet (reusing the [OUTPUT-HUMAN-HTML] rendering path — the daemon returns the snippet as pre-highlighted HTML so the webview stays dumb).
  - "Open in editor" and "Reveal in Explorer" buttons.

Navigation is keyboard-first: `j/k` move occurrence focus, `n/p` move cluster focus, `Enter` opens the file at the focused occurrence, `?` shows the shortcut help. The webview is self-contained — no network fetches, no external CDNs, CSP locked to the extension origin.

#### [VSIX-WEBVIEW-ACTIONS-CONTEXT] Action wiring and hover context

Cluster detail controls must either execute a real command or not render. `Open` dispatches `deslop.openOccurrence` for the row's occurrence. `Compare` dispatches `deslop.compareWithCanonical` for the row's cluster and stays disabled on the canonical occurrence because comparing the canonical row to itself is meaningless. `Previous cluster` and `Next cluster` update the webview's selected cluster through the same signal path as the `p` and `n` keyboard shortcuts; the extension host must not keep a second copy of cluster selection state.

Every visible data item and action in the cluster detail webview carries a human-readable hover explanation. Signal labels explain what the score means and how to interpret high or low values. Occurrence rows explain the target file, line, column, hidden status, and whether the row is canonical. Rank, weight, size, occurrence count, bucket label, AI-match badge, and keyboard shortcut hints explain their purpose without exposing raw byte offsets as the primary user-facing location.

### [VSIX-REPORT-WEBVIEW] Full report webview

Command `deslop.openReport` opens a second webview with the full ranked list — essentially a live-refreshing version of the HTML renderer from [OUTPUT-SCHEMA-JSON], but wired to the daemon's notification stream so it stays current as the user types. Filters: by language, by severity, by file-path glob. Sort is fixed (worst-first) because the whole product premise is worst-first.

### [VSIX-EMBED-PICKER] Embedding model picker

A first-class VSIX surface because the user explicitly asked for it. Trigger:

- Clicking the embedding-model label in the Session panel.
- Running `deslop.pickEmbeddingModel` from the command palette.
- The status bar item (see [VSIX-STATUS-BAR]) when Ollama is detected on the host.

Flow:

1. Fresh installs keep `deslop.embedding.mode = "off"` and show `Select model to enable AI matches` in the Session panel. The VSIX must not let the LSP start the live embedding pass until the user opens this picker and selects a model.
2. VSIX calls `embedding/listModels` on the LSP. The daemon queries Ollama's `/api/tags` endpoint and returns every local model with:
   - `provider_id` (always `ollama` — the only production-registered provider; the deterministic stub is test-only infrastructure and is never listed).
   - `model_id` (e.g. `nomic-embed-code`, `nomic-embed-text`, `codet5p`, `unixcoder`, user-pulled models).
   - `model_version` (`digest` from Ollama).
   - `dimensions` (if known).
   - `size_bytes` (from `/api/tags`).
   - `is_embedding_model: bool` — derived by probing `/api/embed` once and caching; non-embedding models are still shown but tagged as "may not support embeddings."
3. VSIX renders a QuickPick with:
   - A disabled notice that selecting a model starts local embedding calculations, may be slow, and progress remains visible in Session.
   - Each installed model as a primary entry, with a short description of its suitability for code (from a bundled hint table: `nomic-embed-code` → "recommended for code clone detection," `unixcoder` → "alternative; strong on cross-language"), and a dimension/size badge.
   - A separator + "Pull a new model…" action that opens `https://ollama.com/library` in a browser and a second "Refresh list" action.
4. On selection, VSIX calls `embedding/setModel`, persists `deslop.embedding.mode = "auto"`, and keeps the model id visible as pending until a fresh report arrives. The daemon queues the provider refresh, invalidates the embedding cache layer only ([FUSION-EMBED-PROVIDER]), and re-runs the embedding pass in low-priority background batches. Structural + LSH results remain available while this happens.
5. MCP uses the same workspace settings contract. An agent-hosted MCP client must not change the model unless the user explicitly initiated that change. If it does switch the model, it must write the same `deslop.embedding.*` settings the VSIX/LSP reads before the switch is accepted. The VSIX treats those settings as authoritative so LSP and MCP model state does not drift.
6. The status bar updates to `embed: nomic-embed-code`; the Session panel updates; a toast confirms `Embedding model switched to nomic-embed-code`.

### [VSIX-SESSION-PROGRESS] Session embedding progress

The Session panel is the canonical place to check what Deslop is doing. It always includes the active or pending embedding model. When no model has been selected it shows `Select model to enable AI matches` and links to the picker.

During embedding work the panel shows an `Embedding` row with the current phase (`queued`, `starting`, `running`, `failed`), model id, and `done / total` counts. `complete` clears the progress row after the new report lands; `failed` stays visible with the provider message until the user picks another model or a fresh progress event replaces it.

Failure modes:

- Ollama not running / `/api/tags` unreachable → QuickPick shows no selectable models; a disabled info row reads `Ollama not detected — install from ollama.com to use local embedding models`, and a link opens the docs. There is no stub fallback — the deterministic BLAKE3 stub is test-only infrastructure and never reaches a production picker.
- Selected model fails to produce an embedding on probe → VSIX shows the daemon's `EmbeddingProbeError` verbatim, keeps the previous model active.

The picker is the flagship customisation of the VSIX. It's the single UI knob that meaningfully changes analysis quality; every other setting is `min-nodes` and exclusion patterns.

### [VSIX-STATUS-BAR] Status bar

Right-aligned status bar item reading `dedup · 2040 · #1=TradeService.cs:230 · embed=nomic-embed-code`. Sections:

- `dedup` — cluster count in current file (or total if no file open).
- `#1=…` — shortcut to the worst offender. Click jumps to cluster `#1`.
- `embed=<model>` — click opens the embedding picker.

When the daemon is re-analysing, the first section animates to `dedup (analysing…)`. Analysis never blocks the user; this is purely informational.

### [VSIX-COMMANDS] Command palette

Every interaction has a command palette entry:

- `Deslop: Open Report`
- `Deslop: Open Worst Cluster`
- `Deslop: Jump to Next Occurrence in Cluster`
- `Deslop: Compare With Canonical Occurrence`
- `Deslop: Pick Embedding Model`
- `Deslop: Refresh Report (force full re-analysis)`
- `Deslop: Toggle Show All Code Lenses`
- `Deslop: Show Schema Documentation`

VSIX command IDs stay in the `deslop.*` namespace for command palette, menus, and URI links. Any matching LSP `workspace/executeCommand` verb uses the `deslop.lsp.*` namespace so the language client does not double-register VSIX-owned commands during activation.

### [VSIX-SETTINGS] Settings

Exposed under `deslop.*` in VS Code settings:

| Setting | Default | Purpose |
|---|---|---|
| `deslop.minNodes` | `30` | Forwarded to the LSP at `initialize`. Matches CLI `--min-nodes`. |
| `deslop.embedding.provider` | `ollama` | `ollama` is the only production provider; the enum excludes the test-only stub. A stale `"stub"` value persisted by an older build is ignored in memory (treated as `ollama`, embeddings `off`) without rewriting user settings. |
| `deslop.embedding.model` | `nomic-embed-text` | Selected via picker; this is the persisted value. |
| `deslop.embedding.endpoint` | `http://127.0.0.1:11434` | Ollama endpoint. Loopback-only by default. |
| `deslop.embedding.mode` | `off` | Fresh live sessions do not run embeddings until the picker persists `auto` after model selection. |
| `deslop.incremental` | `true` | Mirrors `--incremental`. Always-on in the daemon shell; off for CLI compatibility. |
| `deslop.showAllLenses` | `false` | Show code lenses below the 50th-percentile threshold. |
| `deslop.diagnostics.scope` | `"open-files"` | `"open-files"` keeps LSP 3.17 pull behaviour (Problems only populated for tabs the editor has open); `"workspace"` makes the LSP push `publishDiagnostics` for every offender file so Problems mirrors the Top Offenders tree even with no tabs open. See [lsp.md §LSP-DIAGNOSTICS-SCOPE](lsp.md#lsp-diagnostics-scope). |
| `deslop.configPath` | `""` | Optional override for `.deslop.toml` — mirrors CLI `--config`. |

Settings changes hot-reload the LSP via `workspace/didChangeConfiguration` — no restart required.

### [VSIX-NOTIFICATIONS] User-facing toasts

The extension posts VS Code notifications sparingly:

- On daemon startup failure (missing binary, permission denied) → error toast with a `Reveal log` button.
- On embedding model switch → info toast confirming the new provenance.
- On first activation ever → info toast `Deslop is watching this workspace. Open the Duplicate Clusters view to see the report.` — one-time per workspace, dismissible forever.
- No toasts for ordinary re-analysis. That's what the status bar is for.

### [VSIX-MCP-INTEGRATION] MCP integration for in-VS-Code agents

VS Code's MCP-aware agent hosts (Claude Code, Copilot Chat with MCP) auto-discover the bundled `deslop-mcp` binary through the VSIX's `contributes.mcpServers` manifest entry. The VSIX registers a single server named `deslop` with the same workspace root the LSP uses. The contributed command path must resolve to one of the manifest-approved artifacts; package verification fails if the contribution and `shipwright.json` drift. Agents inside VS Code can call `find-similar` and friends against the same live daemon the UI is driving — one analysis, two consumers, no duplication of state.

Users who run an agent *outside* VS Code (e.g. Claude Code CLI in a terminal) can still wire the MCP up manually via the agent's own config. The VSIX bundling is convenience, not a lock-in.

### [VSIX-TESTING] Extension tests

`clients/vscode/test/` runs the VS Code extension test harness against fixture workspaces:

- Extension activates on `.cs` file open; daemon spawns; activity bar badge appears.
- Tree view populates with clusters ranked worst-first.
- Clicking a cluster node opens the occurrence.
- Editing a buffer updates the tree within 1 s.
- Embedding picker shows the `Ollama not detected` empty state — and never a stub row — when Ollama is unreachable.
- Embedding picker lists Ollama models when a mock Ollama HTTP server is running on `127.0.0.1:11434`.
- Packaged `.vsix` carries no `stub` / `blake3-stub` / `StubProvider` strings in its settings enum or shipped `dist/*.{js,json,md}` assets ([FUSION-EMBED-PROVIDER]); enforced by the `stub-gate` packaging check.
- Cluster webview renders interpretation, signals, and occurrences.
- Full-report webview refreshes on daemon notification.
- Manifest-backed activation tests cover configured paths, environment paths, `DESLOP_BINARY_DIR`, bundled success, `PATH` candidates ignored when the bundle is present, missing binary, component-name mismatch, and version mismatch.
- VSIX archive package tests prove `extension/shipwright.json` exists, `deslop`, `deslop-lsp`, and `deslop-mcp` are under the single target `extension/bin/<platform>/`, no other platform binary directory is present, no undeclared executable is present there, and every host-executable bundled binary reports the manifest `expectedVersion`.

Tests run in CI on every platform shipped in [VSIX-BUNDLE] via GitHub Actions `vscode-test` matrix. Per CLAUDE.md, these are coarse end-to-end tests, not unit tests.
