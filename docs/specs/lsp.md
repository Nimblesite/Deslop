# LSP shell

Thin Language Server Protocol shell over [LIVE-BINARY]. Makes the live Deslop report available to any LSP-compatible editor (VS Code via the VSIX, Neovim, Helix, Zed, Emacs `lsp-mode`, JetBrains via LSP4IJ). The VSIX is the polished reference client; the LSP is the open interface.

Crate: `crates/deslop-lsp`. Transport: JSON-RPC over stdio. Framework: `tower-lsp` (pure Rust, no C deps; already used by `rust-analyzer` and dozens of other servers).

### [LSP-TRANSPORT] Transport + framing

Stdio JSON-RPC 2.0 per the LSP base protocol. No TCP, no named pipes, no WebSockets. The editor spawns the binary; the binary speaks LSP on stdin/stdout; `tracing` goes to stderr (picked up as the "server log" in the client). One process per workspace root.

### [LSP-CAPABILITIES] Server capabilities

Declared in the `initialize` response:

| Capability | Purpose |
|---|---|
| `textDocumentSync = Incremental` | Track in-memory edits so the daemon can analyse buffer contents before save. |
| `codeLensProvider` | Inline "N copies of this block — click to see" badge at the head of every clone occurrence. |
| `documentLinkProvider` | Turn the "other occurrence" entries in a code lens into navigable `textDocument/documentLink` targets. |
| `hoverProvider` | On hover over a clone range: show cluster id, signal breakdown, interpretation, and a "jump to occurrence N" list. |
| `definitionProvider` (overloaded) | "Go to definition" from inside a clone range jumps to the canonical occurrence of that cluster. Users keep the muscle memory. |
| `executeCommandProvider` | Commands for: toggle daemon, refresh report, open full report, pick embedding model, extract-to-shared-function (future). |
| `diagnosticProvider` (pull-based, LSP 3.17) | Publish clone occurrences as diagnostics. **Off by default** ([severity.md §SEVERITY-DIAGNOSTICS-GATE](severity.md#severity-diagnostics-gate)); when enabled, severity is determined by clone bucket ([LSP-SEVERITY]) and is fully user-configurable. |
| `workspace/didChangeWatchedFiles` | Register for writes outside the editor (build output, generated files, `git checkout`). |
| Custom: `deslop/*` | Methods listed in [LSP-CUSTOM-METHODS]. |

### [LSP-SEVERITY] Diagnostic severity — the Problems-panel projection

The LSP is **one consumer** of the Deslop severity model defined in [severity.md §SEVERITY-MODEL](severity.md#severity-model). That file owns the bucket → severity maps; this section owns only how the **diagnostic** projection reaches the Problems panel. Diagnostics are resolved along two axes, applied in order, behind the master gate.

#### [LSP-SEVERITY-BUCKET] Primary axis: clone bucket — diagnostics default OFF

Diagnostic publication is **off by default** (`deslop.diagnostics.enabled = false`, the gate in [severity.md §SEVERITY-DIAGNOSTICS-GATE](severity.md#severity-diagnostics-gate)). When the user enables it, each bucket publishes at its `deslop.diagnostics.severity.*` level — defaults `Identical → Error`, the other three `→ Warning`, every one configurable to `"error" | "warning" | "information" | "hint" | "none"`. The full default table and the `none`-suppresses-this-bucket semantics live in [severity.md §SEVERITY-DIAGNOSTICS](severity.md#severity-diagnostics) and are not duplicated here.

A suppressed or gated-off cluster stays fully visible and **coloured** in the tree, code lens, hover, and live bubble — colour comes from the always-on Deslop-severity map ([severity.md §SEVERITY-COLOR](severity.md#severity-color)), not from this diagnostic projection. Suppression only removes the Problems-panel entry and the squiggle.

#### [LSP-SEVERITY-PERCENTILE] Secondary axis: weight-percentile thresholds

Within each bucket, a cluster is only published as a diagnostic if its weight percentile (across the whole live report) meets the configured floor. This prevents noise from trivial clusters of the same type drowning out the worst offenders.

| Percentile threshold setting | Default | Effect |
|---|---|---|
| `deslop.severity.errorPercentileFloor` | `0` (all) | Only clusters at or above this percentile floor publish as `Error`. |
| `deslop.severity.warningPercentileFloor` | `0` (all) | Only clusters at or above this floor publish as `Warning`. |
| `deslop.severity.informationPercentileFloor` | `0` (all) | Only clusters at or above this floor publish as `Information`. |
| `deslop.severity.hintPercentileFloor` | `0` (all) | Only clusters at or above this floor publish as `Hint`. |

**Percentile is computed across the whole report, not per file.** The defaults (`0`) publish every cluster in its bucket. Teams who want only the worst 10% of identical-code clusters to raise errors set `deslop.severity.errorPercentileFloor = 90`.

Because severity depends on the global weight set, the diagnostic provider declares `inter_file_dependencies: true` ([LSP-CAPABILITIES]); editing one file shifts every other file's percentile, and the client must refresh the corresponding diagnostics.

Clusters below their bucket's percentile floor remain visible via code lens, hover, and the VSIX tree — they are not published as diagnostics but are not hidden.

**Diagnostic resolution is stateless per cluster**: `gate_enabled → bucket → configured_severity (≠ none) → percentile_check → publish or suppress` ([severity.md §SEVERITY-DIAGNOSTICS-GATE](severity.md#severity-diagnostics-gate)). Resolution lives in `crates/deslop-lsp/src/diagnostics.rs` and is the single source of truth — every client (VSIX, Neovim, Helix, agents) consumes the published diagnostics rather than recomputing them from raw weights.

### [LSP-DIAGNOSTICS] Diagnostic content

Each published diagnostic carries:

- `range` — derived from `(start_byte, end_byte)` of the occurrence on this file, using the open buffer's line-index.
- `severity` — per [LSP-SEVERITY].
- `code` — the 16-char cluster id (same one used in text/HTML reports; stable across runs).
- `codeDescription.href` — `deslop://cluster/<id>` custom URI (see [LSP-VIRTUAL-DOC]).
- `message` — the cluster's `interpretation` string (already agent-readable per [PRINCIPLES-AUDIENCE-AGENT]).
- `source` — `"deslop"`.
- `tags` — never `Unnecessary` or `Deprecated`; duplication isn't dead code.
- `relatedInformation` — one entry per *other* occurrence of the cluster, with its `Location` and "occurrence N of M" label. This is what makes the Problems panel jumpable across occurrences.

Diagnostics refresh on every `report/changed` notification from the daemon. Pull-based (LSP 3.17) because push-based diagnostics can interleave badly with buffer edits; `tower-lsp` gives us pull for free.

### [LSP-DIAGNOSTICS-SCOPE] Open-file vs. workspace publication

Pull-based diagnostics only travel to the editor for files the client actively pulls — in practice, files the user has open. With no tab open the Problems panel is empty even when the workspace contains thousands of offenders, which contradicts the Top Offenders tree (always full) and surprises users who expect Problems to mirror the tree. The fix is a single workspace setting that selects the publication scope. Tracked by [#129](https://github.com/Nimblesite/Deslop/issues/129).

| Setting | Type | Default | Effect |
|---|---|---|---|
| `deslop.diagnostics.scope` | `"open-files" \| "workspace"` | `"open-files"` | `open-files` keeps the LSP 3.17 pull contract: only files the editor pulls for receive diagnostics. `workspace` actively pushes `textDocument/publishDiagnostics` for **every** file with at least one occurrence after each pipeline pass, so Problems stays populated regardless of which tabs are open. |

**`workspace` mode publication contract:**

- After every `report/changed`, the LSP iterates the cluster set, groups occurrences by file, and pushes one `publishDiagnostics` per offender file with the full per-file diagnostic list (same payload `diagnostic()` would have produced for a pull).
- Files that drop out of the offender set in a later pass receive an empty `publishDiagnostics` so the editor clears their stale entries — this is non-negotiable per [LSP-PUSH] reactivity.
- Severity, percentile gating, and message content are identical to pull mode — `[LSP-SEVERITY]` is the single source of truth.
- Pull (`textDocument/diagnostic`) keeps working in both modes; `workspace` mode is additive, never replacing the pull path.

`open-files` is the default because pushing thousands of diagnostics on first open of a large workspace is noisy and slows the editor's Problems panel; users who want full coverage opt in. The setting is workspace-scoped (`scope: "window"` in the VSIX) and forwarded to the LSP at `initialize` and on `workspace/didChangeConfiguration`.

### [LSP-CODE-LENS] Code lens

At the first line of every clone occurrence, a code lens reading:

```
●● 4 copies — structural 1.00 · jaccard 0.97 · embedding 0.91 — jump to next
```

The leading glyph (`●●`) is a two-dot severity badge whose colour matches the diagnostic severity. It's Unicode, not ANSI — LSP clients render their own. The text carries the same signal breakdown that appears in the JSON report so a user reading inline has parity with an agent reading the JSON.

Clicking the lens cycles `textDocument/definition` through the remaining occurrences, wrapping at the end. Shift-click runs `deslop.openCluster` (see [LSP-CUSTOM-METHODS]).

### [LSP-HOVER] Hover

Hovering inside a clone range returns a `MarkupContent` (`markdown`) body:

- **Header:** cluster id, weight, rank (`#12 of 2040`), severity badge.
- **Interpretation:** one-liner from `cluster.interpretation`.
- **Signals:** table of `structural / token_jaccard / embedding_cos / fused`.
- **Occurrences:** clickable list of all `path:start-end` locations (markdown links to `deslop://cluster/<id>?occurrence=<i>`).
- **Action hints:** matching entries from `report.action_hints` — the same playbook surfaced in the JSON header.

No snippets in the hover — snippets are in the virtual doc ([LSP-VIRTUAL-DOC]). Keeping the hover narrow keeps it usable in small editor windows.

### [LSP-VIRTUAL-DOC] Virtual document scheme

The shell registers a `deslop://` URI scheme via `textDocument/didOpen` for paths the client resolves through the custom scheme. Three document types:

| URI | Content |
|---|---|
| `deslop://cluster/<id>` | Full cluster detail: interpretation, signals, all occurrences with inlined source snippets + line numbers (same shape as the HTML `<details>` panels from [OUTPUT-HUMAN-HTML]). |
| `deslop://report` | The current report rendered as the canonical text format ([OUTPUT-SCHEMA-JSON] → text renderer). Refreshes on `report/changed`. |
| `deslop://schema` | The embedded `schema_doc` from the report. |

The daemon is the authority — virtual docs are regenerated per request, not stored. Editors that support syntax highlighting on virtual docs (VS Code, Neovim, Helix) get highlighted source snippets for free; others fall back to monospace.

### [LSP-CUSTOM-METHODS] Custom LSP methods

Standard LSP does not have a "give me the live dedup report" request, so the shell exposes a small custom namespace. These are the thin forwarding layer over [LIVE-QUERY-API]:

| LSP method | Forwards to |
|---|---|
| `deslop/reportGet` | `report/get` |
| `deslop/reportForFile` | `report/forFile` |
| `deslop/reportForRange` | `report/forRange` |
| `deslop/clusterById` | `cluster/byId` |
| `deslop/duplicatesFindSimilar` | `duplicates/findSimilar` |
| `deslop/embeddingListModels` | `embedding/listModels` |
| `deslop/embeddingSetModel` | `embedding/setModel` |
| `deslop/sessionConfig` | `session/config` |

Notifications (`deslop/reportChanged`, `deslop/analysisState`, `deslop/embeddingProgress`) mirror the daemon push methods. Namespacing (`deslop/*`) keeps us well clear of reserved LSP methods and any other server's custom namespace.

The MCP-facing local IPC endpoint exposes the same live service for agent-side calls that do not travel through a full LSP client. Unix hosts use `.deslop-cache/deslop.sock`; Windows uses token-gated TCP loopback discovered through `.deslop-cache/deslop.port`. The full method surface is documented in [LIVE-IPC-SOCKET]: five single-shot reads (`report/get`, `report/forFile`, `report/forRange`, `cluster/byId`, `session/config`), three single-shot computes (`duplicates/findSimilar`, `embedding/listModels`, `deslop.lsp.refreshReport`), and one long-lived subscription (`report/subscribe`) that fans `report/changed` notifications out to the MCP. `deslop.lsp.refreshReport` runs the same full-refresh command used by `workspace/executeCommand` so agent `rescan` calls force a re-analysis pass before the next `report/get`.

### [LSP-PUSH] Active push — the LSP never waits for the editor to ask

**This is the most critical correctness property of the live surface.** The LSP must push `deslop/reportChanged` (and `deslop/analysisState`) the moment re-analysis completes — unconditionally, regardless of which actor caused the file change.

**Three complementary triggers feed the same session:**

1. **`notify`-backed filesystem watcher** (`[LIVE-WATCHER]`) — started at LSP init against the full workspace root. Catches every mutation: terminal saves, `git pull`, AI coding agents editing files, CI pipelines, formatters, other editors. **This is the primary, non-negotiable trigger.** The LSP is not a VS Code extension; it cannot assume all mutations come from the editor.
2. **`textDocument/didChange` / `textDocument/didOpen`** — editor-side events for files the user has open. Belt-and-suspenders; slightly faster than waiting for the OS watcher on in-buffer edits.
3. **`workspace/didChangeWatchedFiles`** — the editor's own file-event relay. Belt-and-suspenders.

All three routes call `AnalysisSession::apply_changes` on the **same `Arc<Mutex<AnalysisSession>>`** — no duplicate state, just serialised access. The watcher-driven path goes through `Scheduler` (with a 250 ms debounce); the editor-driven path goes directly.

When the `Scheduler` finishes a pass it broadcasts `ReportChangedNotification` and `AnalysisState`. A background tokio task (`crates/deslop-lsp/src/file_watch.rs`) drains those broadcasts and pushes `deslop/reportChanged` + `deslop/analysisState` to the editor with no request from the editor.

**The VSIX must never rely on polling.** Stale UI after any external mutation — git, terminal, AI agent, CI — is a push-path correctness bug, not a refresh issue. Fix the push.

The `LspBackend` struct owns `_watcher: LiveWatcher` and `_scheduler: Scheduler` for the session lifetime; dropping either stops the watch loop. Watcher startup failures (`LiveError::WatcherInit`) are fatal — the editor surfaces them through the standard "server crashed" notification.

### [LSP-EMBEDDING-CONSENT] Startup embedding behaviour

The LSP starts with embeddings off unless its launch arguments carry a model that the user previously selected: `--embeddings auto|required`, `--embedding-provider`, `--embedding-model`, and `--embedding-endpoint`. A fresh VSIX install launches the LSP with `--embeddings off`, so the initial report is structural/token only and no local model work starts silently.

`deslop/embeddingSetModel` is the first-run consent boundary. The client prompts the user through [VSIX-EMBED-PICKER], calls this method with the selected model, and keeps rendering the last complete report while the LSP emits `deslop/embeddingProgress` updates. The embedding pass runs in low-priority batches with short yields between them.

The LSP and MCP must converge through the same workspace embedding settings. MCP must not choose, infer, rotate, or upgrade the embedding model on its own. When MCP changes the model after explicit user initiation, it persists `deslop.embedding.provider`, `deslop.embedding.model`, `deslop.embedding.endpoint`, and `deslop.embedding.mode` in the VSIX/LSP workspace settings contract before the change is considered accepted. The LSP must treat those settings as authoritative on startup and on configuration reload; neither live surface may keep a private model selection that silently diverges from the other.

### [LSP-COMMANDS] `workspace/executeCommand` verbs

`executeCommandProvider` advertises:

- `deslop.lsp.refreshReport` — force a full re-analysis (drop incremental state, re-run). Rarely needed; the scheduler is reliable. MCP `rescan` may call the same verb over the LSP IPC socket when an agent needs a synchronous post-edit refresh.
- `deslop.lsp.openCluster` — open `deslop://cluster/<id>` in the client.
- `deslop.lsp.openReport` — open `deslop://report`.
- `deslop.lsp.pickEmbeddingModel` — tell the client to prompt the user with the result of `embedding/listModels` and call `embedding/setModel` with the selection. The VSIX implements the prompt as a proper picker ([VSIX-EMBED-PICKER]); other clients fall back to a `showMessageRequest`.
- `deslop.lsp.toggleIncremental` — flip the daemon's incremental-cache behaviour (rare; mostly for debugging cache invalidation).

No `extract-to-function` command in v1 — that's an edit action that belongs downstream of a real refactor engine. Listed here as the eventual home for it so clients know where it will live.

### [LSP-AGENT-FRIENDLY] AI-agent-friendly behaviour

This LSP is used by AI agents (Claude Code, Cursor, Continue) the same way it's used by a human editor. Two implications:

- **Hover, code lens, and diagnostics all carry the cluster id.** An agent reading a diagnostic can call `deslop/clusterById` and get the full JSON cluster (with signals, interpretation, action hints) without re-parsing a hover string.
- **`deslop/duplicatesFindSimilar` is documented as the agent-facing entry point.** Before the agent commits new code, it can ask "is this block already present elsewhere?" and get back concrete clusters to refactor into. See [MCP-TOOL-FINDSIMILAR] for the MCP equivalent.

The LSP does not attempt to auto-surface clone warnings to the agent — the agent asks. This keeps the protocol predictable and the agent in control of its own context budget.

### [LSP-TESTING] E2E tests

Coarse E2E only, per CLAUDE.md. `crates/deslop-lsp/tests/cli.rs` spawns the real LSP binary, talks JSON-RPC over stdio, and asserts against:

- `initialize` + `initialized` handshake returning expected capabilities.
- Opening a fixture workspace produces diagnostics on the known-clone files.
- Editing a buffer triggers `deslop/reportChanged` with a non-empty delta.
- `deslop/reportForRange` returns the expected cluster for a known range.
- `deslop/duplicatesFindSimilar` returns the expected cluster for a hand-crafted snippet.
- `deslop/embeddingSetModel` queues the selected-model embedding refresh; the new provenance appears in `sessionConfig` after the background pass commits.

No mocking of the live session — the LSP binary links `deslop-core` with `features = ["live"]` and runs against `tests/fixtures/` workspaces.
