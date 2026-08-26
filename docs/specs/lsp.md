# LSP shell

Thin Language Server Protocol shell over [LIVE-BINARY]. Makes the live Deslop report available to any LSP-compatible editor (VS Code via the VSIX, Neovim, Helix, Zed, Emacs `lsp-mode`, JetBrains via LSP4IJ). The VSIX is the polished reference client; the LSP is the open interface.

Crate: `crates/deslop-lsp`. Transport: JSON-RPC over stdio. Framework: `tower-lsp` (pure Rust, no C deps; already used by `rust-analyzer` and dozens of other servers).

### [LSP-TRANSPORT] Transport + framing

Stdio JSON-RPC 2.0 per the LSP base protocol. No TCP, no named pipes, no WebSockets. The editor spawns the binary; the binary speaks LSP on stdin/stdout; `tracing` goes to stderr (picked up as the "server log" in the client). One process per workspace root.

### [LSP-LIFECYCLE] How the server ends

The base protocol gives an editor three ways to end a session, and Deslop honours all three.

| What the client does | What the server does | Process exit code |
|---|---|---|
| `shutdown` | Answers, stops accepting new work, **keeps running** — the client still has to send `exit` | — |
| `exit` after `shutdown` | Ends | `0` |
| `exit` with no `shutdown` before it | Ends | `1` — the session was torn down out of order |
| Closes stdin (crashed, force-quit) | Ends | `0` — a vanished client is not a server fault |

A server that ignores `exit` outlives every editor window that opened it, and each abandoned process keeps its file watcher, its IPC socket, and its analysis threads. The machine then accumulates one live analyser per window the user has ever opened.

Two things had to be true for this to work, and neither is automatic:

- **The read loop has to stop on `exit`.** `tower-lsp` only notices that the server has exited when the *next* message arrives — and after `exit` the protocol tells the client there is nothing left to send. `server.rs` watches the wire for the two lifecycle methods and ends the loop on that signal instead of on a message that never comes.
- **The process must not wait for stdin.** The stdio reader is a Tokio blocking task parked in a read that only returns at EOF, and dropping a runtime joins its blocking tasks. A client that says `exit` never closes stdin, so `app.rs` detaches the runtime rather than waiting on it.

Pinned by `crates/deslop-lsp/tests/lifecycle.rs`, which drives the real binary and asserts every row of the table above, exit code included.

**This is also what makes the crate measurable.** A signalled process never runs libc's `atexit` handlers, so LLVM's profile runtime never writes its `.profraw` and every line the server executed is discarded. Measured against this repository's own instrumented binary: `SIGKILL` and `SIGTERM` each produce zero profile files, a closed stdin produces one. The E2E harness therefore reaps servers by closing stdin and waiting (`deslop_test_support::reap`), never by signalling them — which moved `deslop-lsp` from 92.5% to 94.0% line coverage without a single new assertion, because that 1.5% was being deleted at teardown rather than never executed.

### [LSP-CAPABILITIES] Server capabilities

#### [LSP-NON-INTERFERENCE] Deslop is additive — it never touches the editor's standard features

> **Hard invariant.** Deslop MUST NOT register, intercept, override, suppress, or slow standard language-intelligence requests, including definition, hover, references, implementation/type definition, completion, rename, signature help, formatting, highlights, and symbols. Those requests belong to the editor's language server.

Deslop's only `initialize` capabilities are therefore **purely additive** — each one contributes a Deslop-owned surface that stacks on top of the editor's features without replacing any of them:

| Capability | Purpose |
|---|---|
| `textDocumentSync = Incremental` | Track in-memory edits so the daemon can analyse buffer contents before save. Affects no editor command. |
| `codeLensProvider` | Inline "N copies of this block — jump to next" badge at the head of every clone occurrence. A Deslop-owned lens, never a standard feature. |
| `executeCommandProvider` | Deslop's own `deslop.*` verbs ([LSP-COMMANDS]): refresh report, open full report, pick embedding model, etc. |
| `diagnosticProvider` (pull-based, LSP 3.17) | Publish clone occurrences as diagnostics. **Off by default** ([severity.md §SEVERITY-DIAGNOSTICS-GATE](severity.md#severity-diagnostics-gate)); when enabled, severity is determined by clone bucket ([LSP-SEVERITY]) and is fully user-configurable. Additive Problems-panel entries — never replaces another tool's diagnostics. |
| `workspace/didChangeWatchedFiles` | Register for writes outside the editor (build output, generated files, `git checkout`). |
| Custom: `deslop/*` | Methods listed in [LSP-CUSTOM-METHODS] — all namespaced; never a standard request. |

**Deslop deliberately does NOT advertise** `hoverProvider`, `definitionProvider`, `documentLinkProvider`, `referencesProvider`, `completionProvider`, `renameProvider`, or any other standard provider.

LSP clients merge responses from multiple providers and wait for the slowest. Registering a standard provider would therefore add clone locations to unrelated results and could delay editor commands during analysis. Deslop avoids both failures by not registering those providers.

Canonical-occurrence navigation uses Deslop-owned surfaces: the `deslop.jumpToNextOccurrence` code lens, `deslop.compareWithCanonical`, and the Top Offenders tree.

##### [LSP-NON-INTERFERENCE-NONBLOCKING] Even Deslop's own additive reads never block the editor

Every Deslop request handler that reads the report (`diagnostic`, `codeLens`) answers from a lock-free `RwLock<Arc<Report>>` snapshot published by the analysis session — it **never awaits the session mutex**, which an in-flight `apply_changes` pass can hold for seconds on a churning monorepo. So even Deslop's additive surfaces cannot freeze the editor. The freshness-checked read path that re-stats files on demand is reserved for the agent-facing MCP/IPC queries, where a synchronous edit must be reflected immediately ([LIVE-CLUSTER-OFFSET-FRESHNESS]); it never runs on an editor request.

### [LSP-SEVERITY] Diagnostic severity — the Problems-panel projection

The LSP is **one consumer** of the Deslop severity model defined in [severity.md §SEVERITY-MODEL](severity.md#severity-model). That file owns the bucket → severity maps; this section owns only how the **diagnostic** projection reaches the Problems panel. Diagnostics are resolved along two axes, applied in order, behind the master gate.

#### [LSP-SEVERITY-BUCKET] Primary axis: clone bucket → severity

Every cluster publishes a diagnostic whose severity is resolved from its bucket: `Identical → Error`, and `NearlyIdentical` / `LooselySimilar` / `SameBehavior` → `Warning`. Resolution lives in `crates/deslop-lsp/src/diagnostics.rs::severity_for` and is the single source of truth — every client (VSIX, Neovim, Helix, agents) consumes the published diagnostics rather than recomputing them from raw weights. A cluster stays fully visible and **coloured** in the tree, code lens, hover, and live bubble regardless of this diagnostic projection; colour is an always-on VSIX concern, independent of whether a diagnostic is published.

**Status: ⏳ The configurable model is planned (#177).** The `deslop.diagnostics.enabled` master gate (diagnostics off by default), the per-bucket `deslop.diagnostics.severity.*` levels with `none`-suppression, the weight-percentile floors ([LSP-SEVERITY-PERCENTILE]), and the bucket-colour map ([severity.md §SEVERITY-COLOR](severity.md#severity-color)) are specified in [severity.md §SEVERITY-MODEL](severity.md#severity-model) but not yet shipped; today every cluster publishes at its bucket default with no gate and no percentile floor.

#### [LSP-SEVERITY-PERCENTILE] Secondary axis: weight-percentile thresholds

**Status: ⏳ Planned (#177).** Percentile floors are not yet implemented; today every cluster in its bucket publishes a diagnostic. The model below is the target.

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
- `data` — `{ "cluster_id": <16-char cluster id>, "taxonomy": <academic label> }`. The cluster id (stable across runs, same one used in text/HTML reports) rides the machine-facing `data` so an agent can call `deslop/clusterById` without parsing the message ([LSP-AGENT-FRIENDLY]); `code` and `codeDescription` are left unset, and the cluster's `deslop://cluster/<id>` view is reached through [LSP-VIRTUAL-DOC].
- `message` — `"<bucket title> × <count> — <action sentence> — <confidence explanation>"`, the same human-readable, agent-readable line the other surfaces show ([PRINCIPLES-AUDIENCE-AGENT]). The confidence explanation is `deslop-core::render::signals::plain_explanation` — `structural`, `jaccard`, `embedding`, `fused`, then the measured content evidence `agreement`, `rename`, `literal`, each to two decimal places ([FUSION-CONTENT-GATE]). It is the reason the bucket title is falsifiable: a corroborated Type-2 rename and an anchor-poor scaffolding family both render `structural 1.00`, and only the evidence tells them apart. Every surface renders it through that one function, so the Problems panel, the code lens, the Markdown report and the HTML footer can never describe the same numbers differently.
- `source` — `"deslop"`.
- `tags` — never `Unnecessary` or `Deprecated`; duplication isn't dead code.
- `relatedInformation` — one entry per *other* occurrence of the cluster, with its `Location` and "occurrence N of M" label. This is what makes the Problems panel jumpable across occurrences.

Diagnostics refresh on every `report/changed` notification from the daemon. Pull-based (LSP 3.17) because push-based diagnostics can interleave badly with buffer edits; `tower-lsp` gives us pull for free.

### [LSP-DIAGNOSTICS-SCOPE] Open-file vs. workspace publication

**Status: ⏳ `workspace` mode is planned (#129).** Today only the default `open-files` pull behaviour ships; the `deslop.diagnostics.scope` setting and the `workspace`-mode push below are the target.

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
●● 4 copies — structural 1.00 · jaccard 0.97 · embedding 0.91 · fused 0.88 · agreement 0.62 · rename 0.94 · literal 0.11 — jump to next
```

The leading glyph (`●●`) is a two-dot severity badge whose colour matches the diagnostic severity. It's Unicode, not ANSI — LSP clients render their own. The signal breakdown is the same `render::signals::plain_explanation` the diagnostic message carries ([LSP-DIAGNOSTICS], [FUSION-CONTENT-GATE]) — all seven fields of the JSON report's `signals`, so a user reading inline has parity with an agent reading the JSON, and the lens and the Problems panel can never disagree. Plain text, never Markdown: a client renders a lens title verbatim, so a code span would show as a literal backtick.

Clicking the lens runs the Deslop-owned `deslop.jumpToNextOccurrence` command, which cycles through the remaining occurrences, wrapping at the end. It deliberately does **not** route through `textDocument/definition` ([LSP-NON-INTERFERENCE]) — the code lens carries its own command so canonical navigation never touches the editor's Go To Definition. Shift-click runs `deslop.openCluster` (see [LSP-CUSTOM-METHODS]).

### [LSP-HOVER] No LSP hover — the clone card is an additive client surface

**The LSP does not register `hoverProvider`** ([LSP-NON-INTERFERENCE]). Deslop must never answer `textDocument/hover`, because hover results from every provider stack into one popup and the editor waits on the slowest — a duplication linter has no business sitting in that path.

The clone card a user sees on hover is rendered **entirely client-side** by the VSIX's `ClusterHoverProvider` ([VSIX-HOVER-PROVIDER]), registered as an ordinary `vscode.languages.registerHoverProvider`. VS Code *stacks* it alongside the language server's own hover, so it **adds** Deslop's clone information (cluster id, signal breakdown, "Compare with canonical" link) without ever replacing or suppressing the editor's hover. It reads the in-memory report the LSP already pushes via `deslop/reportChanged`, so it needs no LSP hover round-trip.

Editors without a Deslop client lose nothing essential: the same clone information is always available through diagnostics ([LSP-DIAGNOSTICS]) — whose `relatedInformation` links to the canonical occurrence — and the code-lens badges ([LSP-CODE-LENS]).

### [LSP-EDITOR-SURFACES] Editor-neutral rendered surfaces

Every Deslop surface that returns rendered content (cluster detail, report, schema
doc) is produced server-side as editor-neutral markdown/text, so any LSP client —
VS Code, Neovim, Helix, Zed — gets identical output without re-deriving it from
the report structure. The `deslop/virtualDocument` method ([LSP-VIRTUAL-DOC]) is
the canonical example: it resolves each `deslop://` URI into a string the client
opens in a read-only buffer. Rendering is pure and lives in `deslop-core::render`
(e.g. `render_cluster_markdown`, `render_text`); the LSP only supplies a
filesystem source lookup. No surface ships a client-specific shape — the client
chooses presentation (syntax highlighting, theming), never content.

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

The MCP-facing local IPC endpoint exposes the same live service for agent-side calls that do not travel through a full LSP client. Unix hosts use `.deslop/cache/deslop.sock`; Windows uses token-gated TCP loopback discovered through `.deslop/cache/deslop.port`. The full method surface is documented in [LIVE-IPC-SOCKET]: five single-shot reads (`report/get`, `report/forFile`, `report/forRange`, `cluster/byId`, `session/config`), three single-shot computes (`duplicates/findSimilar`, `embedding/listModels`, `deslop.lsp.refreshReport`), and one long-lived subscription (`report/subscribe`) that fans `report/changed` notifications out to the MCP. `deslop.lsp.refreshReport` runs the same full-refresh command used by `workspace/executeCommand` so agent `rescan` calls force a re-analysis pass before the next `report/get`.

### [LSP-WIRE-BUDGET] Keeping the live wire small

Live responses must stay small enough to ship over JSON-RPC on every analysis
pass, so the wire projection differs from the full in-memory report. Each
cluster's `occurrences` is capped at `LIVE_WIRE_OCCURRENCE_CAP` (100) with the
pre-cap total preserved as `occurrences_total`, and the fat derivable strings —
`schema_doc`, per-cluster `summary` and `interpretation` — are blanked because
clients regenerate them. The `schema_doc` is served lazily through its own
`deslop/reportSchemaDoc` method rather than riding every `deslop/reportGet`.
Clients that need the omitted detail page it back via `deslop/clusterById`; this
keeps a pathological multi-thousand-occurrence cluster from blowing the wire from
kilobytes into megabytes.

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

### [LSP-PUSH-NOTIFICATIONS] Server-initiated notifications

Beyond responding to requests, the LSP pushes unsolicited JSON-RPC notifications
the moment server-side state changes: `deslop/reportChanged` (a new analysis
generation is ready) and `deslop/analysisState` (running/idle lifecycle). A
background task drains the scheduler's broadcast channels and forwards each event
to the client ([LSP-PUSH]), so editor surfaces stay live regardless of which actor
mutated a file. Notifications are namespaced `deslop/*` ([LSP-CUSTOM-METHODS]);
clients must never poll in their place, and a stale UI after an external mutation
is a push-path bug, not a refresh issue ([vsix.md §VSIX-REACTIVITY-INVARIANT](vsix.md#vsix-reactivity-invariant)).

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

- **Code lens and diagnostics carry the cluster id** (as does the VSIX's additive clone-hover card). An agent reading a diagnostic can call `deslop/clusterById` and get the full JSON cluster (with signals, interpretation, action hints) without re-parsing a rendered string.
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
