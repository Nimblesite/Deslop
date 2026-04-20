# Principles

### [PRINCIPLES-AUDIENCE-AGENT] Audience for the report: AI coding agents

The report is not just for humans scanning a terminal — **the primary consumer is an AI coding agent using Deslop as a tool**. Design choices follow from that:

- Structured output is the product. JSON is the canonical format; the text renderer is a pretty-printer over the same data. Never emit information in text that isn't also in JSON.
- Every cluster carries enough context for an agent to act without re-reading the whole repo: exact byte ranges, file paths, a canonical representative snippet, the reason signals fired (structural / LSH / embedding with scores), and a suggested refactor hint where one is reliably inferrable (e.g. "extract as shared function," "move to module X," "both call sites are in the same crate").
- The schema is stable, versioned (`report_schema_version`), and strictly-typed so agents can parse without heuristics. Breaking changes bump the version; additive changes don't.
- No ANSI colour codes, no unicode box-drawing, no paging — the agent needs a clean stream. The `text` format is ASCII-only and line-oriented.
- Per-cluster entries include a short natural-language `summary` field written for an agent reader ("3 near-identical copies of a 42-node `switch` block across `Foo.cs:120-180`, `Bar.cs:55-112`, and `Baz.cs:200-260`; structural=1.0, token_jaccard=0.97, embedding_cos=0.91 — safe to extract"). This is a synthesised description, not a log, and it's computed from the same signals the score uses.

See [OUTPUT-SCHEMA-JSON] for the JSON schema. The report format is a first-class interface — changes go through the same review bar as the ranking formula.

### [PRINCIPLES-LONG-RUNNING-DAEMON] Long-running mode (MCP/LSP) as a load-bearing constraint

Deslop v1 is a batch CLI, but the architecture must not foreclose a future daemon mode:

- **Library core.** `codededup-core` owns the pipeline. The CLI is a thin shell. An MCP/LSP binary is just a second shell over the same crate.
- **Incremental first, batch second.** Every pipeline stage (parse, fingerprint, LSH, embedding) is keyed by `(file_content_hash, model_id, model_version)` and cached. A batch run is "incremental starting from empty cache." A watcher-driven update is "incremental starting from the previous cache." There is no separate batch code path.
- **Report is a materialized view over the cache, not a one-shot render.** Clusters are computed from the cached per-file fingerprints; re-rendering after a file change re-runs only cluster recomputation on the affected fingerprints.
- **File-watcher-driven incremental updates are a v2 feature — not v1.** v1 produces correct reports cheaply *because* the cache keys already support "this file didn't change, skip it." v2 wires a `notify`-based watcher to `codededup-core` and calls the existing incremental update path. v1 must ship with the cache keys and the incremental update function in place, even if the only caller is `main`.
- **Byte ranges, not line numbers, are the source of truth** everywhere in the core. Line numbers are derived at render time. LSPs need byte offsets; computing them retroactively would be a rewrite.
- **No process-global mutable state outside `src/state.rs`.** A daemon keeps multiple analyses live in one process — anything that assumes "one run, then exit" will bite later.
