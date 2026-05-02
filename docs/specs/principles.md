# Principles

### [PRINCIPLES-LIVE-IS-REACTIVE] Live = Reactive — non-negotiable

**Every Deslop surface except the CLI is reactive end-to-end.** *Live* is not a marketing label — it is a hard invariant on every running surface: when an analysis pass produces a new report, every reader updates **immediately**, in the same microtask the change fires. Not on the next save. Not when the editor next refreshes. Not on a polling timer. Not when the user runs the CLI again. **Immediately.**

The reactive contract spans the whole product:

- **The watcher → scheduler → pipeline path is reactive.** The `notify`-backed file watcher in `deslop-lsp` feeds the [LIVE-WATCHER] / [LIVE-SCHEDULER] debouncer, which calls `update_files`, produces a new report generation, and atomically writes `.deslop-cache/live-report.json`. No surface polls — the pipeline pushes.
- **The LSP wire is reactive.** The session fires `deslop/reportChanged` ([LIVE-NOTIFICATIONS]) for every observable change, removals included. There is no `pollReport` request.
- **The MCP wire is reactive.** The MCP watches `.deslop-cache/live-report.json` for modification ([MCP-NOTIFICATIONS]) and pushes change notifications to the agent immediately. An agent that calls `find-similar` one tick after a notification is reading a snapshot consistent with the file system.
- **The VSIX is reactive.** Tree, decorations, bubble, status bar, code lenses, hovers, webviews — every surface is `@preact/signals`-driven over the single [VSIX-STATE] store. Updates settle transactionally. No surface holds its own cache. No surface schedules a refresh independent of a signal change. Enforced by [VSIX-REACTIVITY] and its acceptance test in [VSIX-REACTIVITY-INVARIANT].
- **Future editor surfaces inherit the rule.** The JetBrains plugin, any Zed / Neovim / web-dashboard integration — every new client implements the same notification → store → signal → render path. There is no second-class "polls every N seconds" client.

The **CLI is the only non-reactive surface in the product**, and that is by design: it is the cold-cache fallback for CI gates and one-shot audits, where reactivity has no meaning because the process exits before the caller reads the output. Anything that runs as a long-lived process is reactive. No exceptions.

**Stale UI is a correctness bug.** "The tree still shows clusters that were just deleted from the source" is not a polish issue — it is a failure of the brand promise ("tell the developer they're duplicating right now"). Bugs of this class are fixed at the same priority as wrong analysis output. Lint rules and cross-surface E2E tests enforce the invariant; see [VSIX-REACTIVITY-INVARIANT].

### [PRINCIPLES-AUDIENCE-AGENT] Audience for the report: AI coding agents

The report is not just for humans scanning a terminal — **the primary consumer is an AI coding agent using Deslop as a tool**. Design choices follow from that:

- Structured output is the product. JSON is the canonical format; the text renderer is a pretty-printer over the same data. Never emit information in text that isn't also in JSON.
- Every cluster carries enough context for an agent to act without re-reading the whole repo: exact byte ranges, file paths, a canonical representative snippet, the reason signals fired (structural / LSH / embedding with scores), and a suggested refactor hint where one is reliably inferrable (e.g. "extract as shared function," "move to module X," "both call sites are in the same crate").
- The schema is stable, versioned (`report_schema_version`), and strictly-typed so agents can parse without heuristics. Breaking changes bump the version; additive changes don't.
- No ANSI colour codes, no unicode box-drawing, no paging — the agent needs a clean stream. The `text` format is ASCII-only and line-oriented.
- Per-cluster entries include a short natural-language `summary` field written for an agent reader ("3 near-identical copies of a 42-node `switch` block across `Foo.cs:120-180`, `Bar.cs:55-112`, and `Baz.cs:200-260`; structural=1.0, token_jaccard=0.97, embedding_cos=0.91 — safe to extract"). This is a synthesised description, not a log, and it's computed from the same signals the score uses.

See [OUTPUT-SCHEMA-JSON] for the JSON schema. The report format is a first-class interface — changes go through the same review bar as the ranking formula.

### [PRINCIPLES-LONG-RUNNING-DAEMON] Long-running mode (LSP) as a load-bearing constraint

Deslop v1 is a batch CLI, but the architecture must not foreclose a future daemon mode:

- **Library core.** `deslop-core` owns the pipeline. The CLI is a thin shell. The LSP is the long-running analysis process; the MCP is a thin state reader over the LSP's output file. Adding a new surface means reading the state file, not duplicating the analysis library.
- **Incremental first, batch second.** Every pipeline stage (parse, fingerprint, LSH, embedding) is keyed by `(file_content_hash, model_id, model_version)` and cached. A batch run is "incremental starting from empty cache." A watcher-driven update is "incremental starting from the previous cache." There is no separate batch code path.
- **Report is a materialized view over the cache, not a one-shot render.** Clusters are computed from the cached per-file fingerprints; re-rendering after a file change re-runs only cluster recomputation on the affected fingerprints.
- **File-watcher-driven incremental updates are a v2 feature — not v1.** v1 produces correct reports cheaply *because* the cache keys already support "this file didn't change, skip it." v2 wires a `notify`-based watcher to `deslop-core` and calls the existing incremental update path. v1 must ship with the cache keys and the incremental update function in place, even if the only caller is `main`.
- **Byte ranges, not line numbers, are the source of truth** everywhere in the core. Line numbers are derived at render time. LSPs need byte offsets; computing them retroactively would be a rewrite.
- **No process-global mutable state outside `src/state.rs`.** A daemon keeps multiple analyses live in one process — anything that assumes "one run, then exit" will bite later.
