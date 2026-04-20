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
| `diagnosticProvider` (pull-based, LSP 3.17) | Publish clone occurrences as `Information` / `Hint` diagnostics. Severity scales with cluster weight ([LSP-SEVERITY]). |
| `workspace/didChangeWatchedFiles` | Register for writes outside the editor (build output, generated files, `git checkout`). |
| Custom: `deslop/*` | Methods listed in [LSP-CUSTOM-METHODS]. |

### [LSP-SEVERITY] Mapping cluster weight → diagnostic severity

Cluster weights (`count × (size−1) × log2(1 + spanned_loc)`) are unbounded, so they're bucketed:

| Weight percentile in current report | Severity | UX |
|---|---|---|
| Top 1% (the worst offenders) | `Warning` | Yellow squiggle, surfaces in Problems panel. |
| 1 – 10% | `Information` | Info dot, Problems panel. |
| 10 – 50% | `Hint` | Faded underline, Problems panel only if filter allows. |
| Bottom 50% | Not published as a diagnostic | Still visible via code lens + hover. |

**Percentile is computed across the whole report, not per file.** A cluster's severity is its weight's percentile against the weights of every cluster in the live report. A cluster that is the worst offender in a sleepy file but mid-tier overall must rank mid-tier in the Problems panel — otherwise a quiet file with three trivial near-misses would publish a `Warning` while the actual hot files compete for the same bucket. This matches the "worst offenders first" rank order surfaced everywhere else (CLI text report, VSIX top-offenders tree, HTML report).

Because severity depends on the global weight set, the diagnostic provider declares `inter_file_dependencies: true` ([LSP-CAPABILITIES]); editing one file shifts every other file's percentile, and the client must refresh the corresponding diagnostics.

Severity is **never** `Error` — duplication isn't a compile error, and polluting the error stream breaks existing developer workflows (CI red on clone count is a future opt-in, not the default). Percentile thresholds are fixed; the user doesn't tune severity, they tune `min-nodes` and exclusion patterns instead.

Severity bucketing lives in `crates/deslop-lsp/src/diagnostics.rs` and is the single source of truth — every client (VSIX, Neovim, Helix, agents) consumes the published diagnostics rather than recomputing severity from raw weights.

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

Notifications (`deslop/reportChanged`, `deslop/analysisState`) mirror the daemon push methods. Namespacing (`deslop/*`) keeps us well clear of reserved LSP methods and any other server's custom namespace.

### [LSP-COMMANDS] `workspace/executeCommand` verbs

`executeCommandProvider` advertises:

- `deslop.refreshReport` — force a full re-analysis (drop incremental state, re-run). Rarely needed; the scheduler is reliable.
- `deslop.openCluster` — open `deslop://cluster/<id>` in the client.
- `deslop.openReport` — open `deslop://report`.
- `deslop.pickEmbeddingModel` — tell the client to prompt the user with the result of `embedding/listModels` and call `embedding/setModel` with the selection. The VSIX implements the prompt as a proper picker ([VSIX-EMBED-PICKER]); other clients fall back to a `showMessageRequest`.
- `deslop.toggleIncremental` — flip the daemon's incremental-cache behaviour (rare; mostly for debugging cache invalidation).

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
- `deslop/embeddingSetModel` swaps the embedding provenance in the next `sessionConfig`.

No mocking of the live session — the LSP binary links `deslop-core` with `features = ["live"]` and runs against `tests/fixtures/` workspaces.
