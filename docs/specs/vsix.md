# VSIX — the VS Code extension

The VSIX is the **polished reference client** for the CodeDedup daemon. Every other editor can wire up the LSP ([lsp.md](lsp.md)) and get a competent experience; the VSIX is where we prove what a genuinely beautiful duplication-surfacing UI looks like.

Distribution: Marketplace + OpenVSX as a single `.vsix`. Extension id: `codededup.codededup-vscode`. Published from `clients/vscode/` in this repo.

### [VSIX-PRINCIPLES] UX principles

1. **In your face the moment you duplicate.** When the user types code that matches an existing cluster, the editor tells them **immediately** via the live-bubble ([VSIX-LIVE-BUBBLE]) — not on save, not on CI, not in a panel they have to open. This is the product's defining moment. Every other UX decision is subordinate to making it land cleanly.
2. **Silent when the code is clean.** If there are no clusters overlapping the current file, no UI elements appear on that file. The activity bar badge disappears. The editor is untouched. Loudness is reserved for real duplication.
3. **The worst offender is always one click away.** The activity bar icon always jumps to cluster `#1` of the live report. The user never navigates through menus to find duplication hotspots.
4. **Every surface speaks the same schema.** Tree view, hover, code lens, status bar, bubble, webview — all render the same `Report` the JSON file carries. Humans and agents read the same truth.
5. **Never block an edit.** The daemon is a sidecar; analysis runs asynchronously; UI updates ride notifications. A typing pause of 250 ms triggers re-analysis, not every keystroke.
6. **Legible, not decorative.** No animated icons, no gradient flourishes that obscure content. Density is high but scannable — the user is hunting for duplication, not admiring chrome. Severity is communicated by colour ramp + glyph, nothing else.

### [VSIX-LIVE-BUBBLE] Live duplication bubble — the flagship UX

This is the feature. The VSIX is the first tool that tells a developer **"you are duplicating code right now"** while the code is still under their cursor. Every other surface (tree view, webview, code lens, status bar) is supporting cast; the bubble is the lead.

**When it fires.**
After every coalesced buffer edit ([LIVE-WATCHER] debounce = 250 ms), the VSIX issues `duplicates/findSimilar` on the range the user most recently touched. If a cluster comes back with fused score ≥ `FUSED_THRESHOLD` (0.85, same as the offline report), the bubble appears anchored to the bottom-right of the duplicated range. If nothing matches, no bubble — silence is the signal that the code is novel.

**What it looks like.**
A compact floating widget (VS Code `InlayHint` + `Webview`-backed overlay, rendered by a single `DecorationType` whose `after.contentText` is an HTML-safe Unicode glyph, with a hover-triggered richer webview for detail). Anatomy, from left to right:

- **Severity dot** — the same colour ramp as [LSP-SEVERITY] (red for top 1% weight, amber for 1–10%, blue for 10–50%, faint grey never shown as a bubble).
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

**Ghost-line mode (opt-in, `codededup.liveBubble.mode = "ghost"`).**
For users who want a tighter callout, ghost-line mode renders the bubble on a **phantom line inserted below the duplicated range**, using VS Code's `CodeLens` API with a custom-styled title. The phantom line is visually distinct from the real buffer (dimmed background, italic). It never modifies the buffer; scroll behaviour matches code lenses. This is the closest thing to "a speech bubble pointing at the duplicate" that VS Code natively supports.

**Cooldown + budget.**
- Bubbles don't flicker: once shown for a range, the same cluster on the same range stays bubbled until the user moves out, even if debounce re-fires. Cluster stability across re-analyses ([LIVE-DELTA]) makes this trivial — same id, same bubble.
- The live-bubble query has a 250 ms budget on the daemon side ([LIVE-PERF-BUDGETS]); if it misses, the bubble is skipped for that edit cycle and will try again on the next debounce. No stale bubbles.
- At most one bubble visible per editor at a time (the worst-weight cluster overlapping the most-recently-edited range). Users reading a report don't need N bubbles competing for attention; the tree view ([VSIX-ACTIVITY-BAR]) shows all of them.

**Dismissal.**
- `Escape` dismisses the bubble until the next edit re-triggers.
- Clicking a `Dismiss for this cluster` action in the expanded card suppresses that cluster id for the session. Session-scoped, never persisted — the next day, the duplication is real again and we say so.
- `codededup.liveBubble.enabled = false` turns the bubble off globally for users who want the rest of the VSIX without the in-your-face moment. Off-by-default is **not** a setting we ship — silence-when-clean already gives users a tolerable floor; the bubble is on from the first install.

**Why this is the headline.**
No competitor ([competitors.md](competitors.md)) tells a developer about duplication at typing time. PMD CPD runs on CI. jscpd runs on CI. SonarLint flags on save, after the thought is already committed. JetBrains' inspection flashes a Problems panel entry you have to look for. CodeDedup *shows the duplicate to the developer inside the IDE, inline with their cursor, as they type the thing*. First tool to do it. Called out on the Marketplace listing, the README, and every demo GIF.

### [VSIX-BUNDLE] Extension bundle

The VSIX ships:

- The extension TypeScript (`clients/vscode/src/extension.ts`, under 500 lines per CLAUDE.md; UI logic split into `webview/`, `tree/`, `decorations/`, `commands/`).
- A pre-built `codededup-lsp` binary per platform (darwin-arm64, darwin-x64, linux-x64, linux-arm64, win32-x64). Download-on-first-activate is **not** acceptable — the extension either works offline immediately or it doesn't install.
- A pre-built `codededup-mcp` binary per platform, colocated, registered with any MCP-aware VS Code host (Claude Code, Copilot Chat with MCP, etc.) via the extension's MCP contribution point.
- The shared `codededup-report-view` webview bundle (preact + no external CSS framework; see [VSIX-WEBVIEW]).
- The extension's own `schema_doc.md` pulled from `docs/specs/REPORTING-CONTEXT.md` at build time — the same `include_str!` content the report embeds. Drift is impossible.

### [VSIX-ACTIVATION] Activation

Activation events:

- `onLanguage:csharp`, `onLanguage:rust`, `onLanguage:python` — mirror the supported language set; extending requires a VSIX rebuild when `codededup-core` adds a language.
- `onCommand:codededup.openReport` — cold activation when the user explicitly asks for the report.
- `workspaceContains:**/*.cs`, `**/*.rs`, `**/*.py` — pre-warm the LSP on project open.

On activation: spawn the bundled `codededup-lsp` binary rooted at the first workspace folder, start the LSP client, wire up the VSIX UI surfaces below. Multi-root workspaces get one LSP process per root.

### [VSIX-ACTIVITY-BAR] Activity bar + tree view

A dedicated activity bar icon (a stylised "dd" mark, the same one used in the Marketplace listing) opens the **Duplicate Clusters** view container. Inside:

- **Top Offenders** tree — one node per cluster, ranked worst-first. Each node shows:
  - Rank badge (`#1`, `#2`, …) coloured by severity ([LSP-SEVERITY]).
  - Short interpretation (e.g. `Type-1 exact · 6 copies · 320 nodes`).
  - Cluster id in a subdued monospace suffix.
  - Children: one node per occurrence, `path` + byte range rendered as `line:col` for humans. Clicking opens the file at the occurrence.
- **Focused File** tree — the cluster subset overlapping the currently open editor. Collapses when no clusters apply.
- **Session** panel — compact footer with: active embedding model (linkable, opens the picker), `cache_stats`, `files_analysed`, daemon state (`idle` / `running`).

Tree refresh is driven by `codededup/reportChanged`; the webview uses the same notification to bump its own state.

### [VSIX-CODE-LENS] Code lens

The LSP's code lens ([LSP-CODE-LENS]) is the content source. The VSIX styles it with the same severity colour ramp so inline clone markers match the tree view.

Each lens has three actions in its command array:

- **"Jump"** — cycles `textDocument/definition` through remaining occurrences.
- **"Compare"** — opens VS Code's diff view between this occurrence and the canonical occurrence of the cluster.
- **"Open cluster"** — opens the webview ([VSIX-WEBVIEW]) pinned to this cluster.

The lens is suppressed for clusters below the 50th weight percentile (consistent with [LSP-SEVERITY]). Users can toggle via `codededup.showAllLenses` (off by default — this is the silent-when-clean principle in action).

### [VSIX-DECORATIONS] Editor decorations

Occurrences in the active editor get a subtle gutter decoration (a thin coloured bar, severity-mapped) and a 1-pixel underline on the clone range. Hover over the underline reveals the full cluster detail via the LSP hover provider.

No background highlighting, no border boxes, no emoji markers in the gutter. The decoration is visible at a glance but doesn't fight with any existing theme.

### [VSIX-WEBVIEW] Cluster detail webview

Command `codededup.openCluster` opens a webview tab. The tab renders a single cluster with:

- Header: cluster id, rank, weight, size, severity badge, jump-to-next-cluster / jump-to-prev-cluster arrows.
- Interpretation and action hints (the same fields the JSON carries).
- Signal breakdown as four tiny bars: structural, token Jaccard, embedding cosine, fused. Each labelled with its numeric value to two decimals.
- One collapsible panel per occurrence, each containing:
  - File path (clickable — opens the file at the byte range).
  - Line-numbered, syntax-highlighted source snippet (reusing the [OUTPUT-HUMAN-HTML] rendering path — the daemon returns the snippet as pre-highlighted HTML so the webview stays dumb).
  - "Open in editor" and "Reveal in Explorer" buttons.

Navigation is keyboard-first: `j/k` move occurrence focus, `n/p` move cluster focus, `Enter` opens the file at the focused occurrence, `?` shows the shortcut help. The webview is self-contained — no network fetches, no external CDNs, CSP locked to the extension origin.

### [VSIX-REPORT-WEBVIEW] Full report webview

Command `codededup.openReport` opens a second webview with the full ranked list — essentially a live-refreshing version of the HTML renderer from [OUTPUT-SCHEMA-JSON], but wired to the daemon's notification stream so it stays current as the user types. Filters: by language, by severity, by file-path glob. Sort is fixed (worst-first) because the whole product premise is worst-first.

### [VSIX-EMBED-PICKER] Embedding model picker

A first-class VSIX surface because the user explicitly asked for it. Trigger:

- Clicking the embedding-model label in the Session panel.
- Running `codededup.pickEmbeddingModel` from the command palette.
- The status bar item (see [VSIX-STATUS-BAR]) when Ollama is detected on the host.

Flow:

1. VSIX calls `embedding/listModels` on the LSP. The daemon queries Ollama's `/api/tags` endpoint and returns every local model with:
   - `provider_id` (`ollama` / `stub`).
   - `model_id` (e.g. `nomic-embed-code`, `nomic-embed-text`, `codet5p`, `unixcoder`, user-pulled models).
   - `model_version` (`digest` from Ollama).
   - `dimensions` (if known).
   - `size_bytes` (from `/api/tags`).
   - `is_embedding_model: bool` — derived by probing `/api/embeddings` once and caching; non-embedding models are still shown but tagged as "may not support embeddings."
2. VSIX renders a QuickPick with:
   - Each installed model as a primary entry, with a short description of its suitability for code (from a bundled hint table: `nomic-embed-code` → "recommended for code clone detection," `unixcoder` → "alternative; strong on cross-language"), and a dimension/size badge.
   - The built-in `stub` provider as the last entry, for users who want deterministic CI-style behaviour without Ollama.
   - A separator + "Pull a new model…" action that opens `https://ollama.com/library` in a browser and a second "Refresh list" action.
3. On selection, VSIX calls `embedding/setModel`. The daemon swaps providers atomically, invalidates the embedding cache layer only ([FUSION-EMBED-PROVIDER]), and re-runs the embedding pass on existing subtrees. Structural + LSH results are unaffected.
4. The status bar updates to `embed: nomic-embed-code`; the Session panel updates; a toast confirms `Embedding model switched to nomic-embed-code`.

Failure modes:

- Ollama not running / `/api/tags` unreachable → QuickPick shows `stub` only, a disabled info row reads `Ollama not detected — install from ollama.com to use local embedding models`, and a link opens the docs.
- Selected model fails to produce an embedding on probe → VSIX shows the daemon's `EmbeddingProbeError` verbatim, keeps the previous model active.
- User selects `stub` → confirmation dialog explains `stub` is deterministic but not semantically meaningful, so Type-4 recall is effectively disabled. Honours user choice.

The picker is the flagship customisation of the VSIX. It's the single UI knob that meaningfully changes analysis quality; every other setting is `min-nodes` and exclusion patterns.

### [VSIX-STATUS-BAR] Status bar

Right-aligned status bar item reading `dedup · 2040 · #1=TradeService.cs:230 · embed=nomic-embed-code`. Sections:

- `dedup` — cluster count in current file (or total if no file open).
- `#1=…` — shortcut to the worst offender. Click jumps to cluster `#1`.
- `embed=<model>` — click opens the embedding picker.

When the daemon is re-analysing, the first section animates to `dedup (analysing…)`. Analysis never blocks the user; this is purely informational.

### [VSIX-COMMANDS] Command palette

Every interaction has a command palette entry:

- `CodeDedup: Open Report`
- `CodeDedup: Open Worst Cluster`
- `CodeDedup: Jump to Next Occurrence in Cluster`
- `CodeDedup: Compare With Canonical Occurrence`
- `CodeDedup: Pick Embedding Model`
- `CodeDedup: Refresh Report (force full re-analysis)`
- `CodeDedup: Toggle Show All Code Lenses`
- `CodeDedup: Show Schema Documentation`

Each entry maps 1:1 to an LSP `workspace/executeCommand` or virtual-document open. Nothing UI-only — keeps the VSIX a thin client.

### [VSIX-SETTINGS] Settings

Exposed under `codededup.*` in VS Code settings:

| Setting | Default | Purpose |
|---|---|---|
| `codededup.minNodes` | `30` | Forwarded to the LSP at `initialize`. Matches CLI `--min-nodes`. |
| `codededup.embedding.provider` | `ollama` | `ollama` or `stub`. |
| `codededup.embedding.model` | `nomic-embed-text` | Selected via picker; this is the persisted value. |
| `codededup.embedding.endpoint` | `http://127.0.0.1:11434` | Ollama endpoint. Loopback-only by default. |
| `codededup.embedding.mode` | `auto` | Mirrors `--embeddings={auto,required,off}`. |
| `codededup.incremental` | `true` | Mirrors `--incremental`. Always-on in the daemon shell; off for CLI compatibility. |
| `codededup.showAllLenses` | `false` | Show code lenses below the 50th-percentile threshold. |
| `codededup.configPath` | `""` | Optional override for `.codededup.toml` — mirrors CLI `--config`. |

Settings changes hot-reload the LSP via `workspace/didChangeConfiguration` — no restart required.

### [VSIX-NOTIFICATIONS] User-facing toasts

The extension posts VS Code notifications sparingly:

- On daemon startup failure (missing binary, permission denied) → error toast with a `Reveal log` button.
- On embedding model switch → info toast confirming the new provenance.
- On first activation ever → info toast `CodeDedup is watching this workspace. Open the Duplicate Clusters view to see the report.` — one-time per workspace, dismissible forever.
- No toasts for ordinary re-analysis. That's what the status bar is for.

### [VSIX-MCP-INTEGRATION] MCP integration for in-VS-Code agents

VS Code's MCP-aware agent hosts (Claude Code, Copilot Chat with MCP) auto-discover the bundled `codededup-mcp` binary through the VSIX's `contributes.mcpServers` manifest entry. The VSIX registers a single server named `codededup` with the same workspace root the LSP uses. Agents inside VS Code can call `find-similar` and friends against the same live daemon the UI is driving — one analysis, two consumers, no duplication of state.

Users who run an agent *outside* VS Code (e.g. Claude Code CLI in a terminal) can still wire the MCP up manually via the agent's own config. The VSIX bundling is convenience, not a lock-in.

### [VSIX-TESTING] Extension tests

`clients/vscode/test/` runs the VS Code extension test harness against fixture workspaces:

- Extension activates on `.cs` file open; daemon spawns; activity bar badge appears.
- Tree view populates with clusters ranked worst-first.
- Clicking a cluster node opens the occurrence.
- Editing a buffer updates the tree within 1 s.
- Embedding picker lists `stub` when Ollama is unreachable.
- Embedding picker lists Ollama models when a mock Ollama HTTP server is running on `127.0.0.1:11434`.
- Cluster webview renders interpretation, signals, and occurrences.
- Full-report webview refreshes on daemon notification.

Tests run in CI on every platform shipped in [VSIX-BUNDLE] via GitHub Actions `vscode-test` matrix. Per CLAUDE.md, these are coarse end-to-end tests, not unit tests.
