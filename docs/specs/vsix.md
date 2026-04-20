# VSIX — the VS Code extension

The VSIX is the **polished reference client** for the CodeDedup daemon. Every other editor can wire up the LSP ([lsp.md](lsp.md)) and get a competent experience; the VSIX is where we prove what a genuinely beautiful duplication-surfacing UI looks like.

Distribution: Marketplace + OpenVSX as a single `.vsix`. Extension id: `codededup.codededup-vscode`. Published from `clients/vscode/` in this repo.

### [VSIX-PRINCIPLES] UX principles

1. **Silent when the code is clean, loud only when it matters.** If there are no clusters overlapping the current file, no UI elements appear on that file. The activity bar badge disappears. The editor is untouched.
2. **The worst offender is always one click away.** The activity bar icon always jumps to cluster `#1` of the live report. The user never navigates through menus to find duplication hotspots.
3. **Every surface speaks the same schema.** Tree view, hover, code lens, status bar, webview — all render the same `Report` the JSON file carries. Humans and agents read the same truth.
4. **Never block an edit.** The daemon is a sidecar; analysis runs asynchronously; UI updates ride notifications. A typing pause of 250 ms triggers re-analysis, not every keystroke.
5. **Legible, not decorative.** No animated icons, no gradient flourishes that obscure content. Density is high but scannable — the user is hunting for duplication, not admiring chrome. Severity is communicated by colour ramp + glyph, nothing else.

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
