# Principles

### [PRINCIPLES-LIVE-IS-REACTIVE] Live = Reactive — non-negotiable

Every Deslop surface except the CLI applies each new report in the same microtask that publishes the change.

The reactive contract spans the whole product:

- **Watcher → scheduler → pipeline.** The `notify` watcher feeds [LIVE-WATCHER] / [LIVE-SCHEDULER], which calls `update_files` and publishes a report generation.
- **LSP.** The session fires `deslop/reportChanged` ([LIVE-NOTIFICATIONS]) for every observable change, including removals; clients do not poll.
- **MCP.** The MCP subscribes over local IPC ([MCP-NOTIFICATIONS]) and serves the LSP's current in-memory snapshot.
- **VSIX.** Tree, decorations, bubble, status bar, code lenses, hovers, and webviews derive from the single [VSIX-STATE] store. [VSIX-REACTIVITY-INVARIANT] enforces transactional updates and forbids per-surface caches.
- **Future clients.** Every editor integration implements the same notification → store → signal → render path.

The CLI is the only non-reactive surface because it exits after a one-shot CI or audit run.

Stale UI is a correctness bug and has the same priority as wrong analysis output. Lint rules and cross-surface E2E tests enforce [VSIX-REACTIVITY-INVARIANT].

### [PRINCIPLES-AUDIENCE-AGENT] Audience for the report: AI coding agents

AI coding agents are the primary report consumer, while human renderers use the same data:

- Structured output is the product. JSON is the canonical format; the text renderer is a pretty-printer over the same data. Never emit information in text that isn't also in JSON.
- Every cluster carries enough context for an agent to act without re-reading the whole repo: exact byte ranges, file paths, a canonical representative snippet, the reason signals fired (structural / LSH / embedding with scores), and a suggested refactor hint where one is reliably inferrable (e.g. "extract as shared function," "move to module X," "both call sites are in the same crate").
- The report is strictly typed so agents can parse without heuristics. Persisted state that does not match the current shape is discarded and recreated.
- No ANSI colour codes, no unicode box-drawing, no paging — the agent needs a clean stream. The `text` format is ASCII-only and line-oriented.
- Per-cluster entries include an agent-readable `summary` computed from the same locations and signals as the structured fields.

See [OUTPUT-SCHEMA-JSON] for the JSON schema. The report format is a first-class interface — changes go through the same review bar as the ranking formula.

### [PRINCIPLES-LONG-RUNNING-DAEMON] Long-running mode (LSP) as a load-bearing constraint

Deslop v1 is a batch CLI, but the architecture must not foreclose a future daemon mode:

- **Library core.** `deslop-core` owns the pipeline. The CLI is a thin shell. The LSP is the long-running analysis process; the MCP is a thin IPC delegate over the LSP's live in-memory report. Adding a new surface means reading through the live API, not duplicating the analysis library.
- **Incremental first, batch second.** Every pipeline stage (parse, fingerprint, LSH, embedding) is keyed by `(file_content_hash, model_id, model_version)` and cached. A batch run is "incremental starting from empty cache." A watcher-driven update is "incremental starting from the previous cache." There is no separate batch code path.
- **Report is a materialized view over the cache, not a one-shot render.** Clusters are computed from the cached per-file fingerprints; re-rendering after a file change re-runs only cluster recomputation on the affected fingerprints.
- **File-watcher-driven incremental updates are a v2 feature — not v1.** v1 produces correct reports cheaply *because* the cache keys already support "this file didn't change, skip it." v2 wires a `notify`-based watcher to `deslop-core` and calls the existing incremental update path. v1 must ship with the cache keys and the incremental update function in place, even if the only caller is `main`.
- **Byte ranges, not line numbers, are the source of truth** everywhere in the core. Line numbers are derived at render time. LSPs need byte offsets; computing them retroactively would be a rewrite.
- **No process-global mutable state outside `src/state.rs`.** A daemon keeps multiple analyses live in one process — anything that assumes "one run, then exit" will bite later.
