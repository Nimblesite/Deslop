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
- Every cluster carries exact occurrence ranges, file paths, canonical extent, and duplicated mass. Pair evidence is requested separately for two explicit occurrences; no cluster record contains structural, token, embedding, or content scores.
- The report is strictly typed so agents can parse without heuristics. Persisted state that does not match the current shape is discarded and recreated.
- No ANSI colour codes, no unicode box-drawing, no paging — the agent needs a clean stream. The `text` format is ASCII-only and line-oriented.
- Per-cluster summaries describe occurrence membership and mass only. Pair-comparison summaries describe only the two named endpoints and their pair evidence.

See [OUTPUT-SCHEMA-JSON] for the JSON schema. The report format is a first-class interface — changes go through the same review bar as the ranking formula.

### [PRINCIPLES-LONG-RUNNING-DAEMON] Long-running mode (LSP) as a load-bearing constraint

Deslop v1 is a batch CLI, but the architecture must not foreclose a future daemon mode:

- **Library core.** `deslop-core` owns the pipeline. The CLI is a thin shell. The LSP is the long-running analysis process; the MCP is a thin IPC delegate over the LSP's live in-memory report. Adding a new surface means reading through the live API, not duplicating the analysis library.
- **Incremental first, batch second.** Every pipeline stage (parse, fingerprint, LSH, embedding) is keyed by `(file_content_hash, model_id, model_version)` and cached. A batch run is "incremental starting from empty cache." A watcher-driven update is "incremental starting from the previous cache." There is no separate batch code path.
- **Report is a materialized view over the cache, not a one-shot render.** Clusters are computed from the cached per-file fingerprints; re-rendering after a file change re-runs only cluster recomputation on the affected fingerprints.
- **File-watcher-driven incremental updates are a v2 feature — not v1.** v1 produces correct reports cheaply *because* the cache keys already support "this file didn't change, skip it." v2 wires a `notify`-based watcher to `deslop-core` and calls the existing incremental update path. v1 must ship with the cache keys and the incremental update function in place, even if the only caller is `main`.
- **Byte ranges, not line numbers, are the source of truth** everywhere in the core. Line numbers are derived at render time. LSPs need byte offsets; computing them retroactively would be a rewrite.
- **No process-global mutable state outside `src/state.rs`.** A daemon keeps multiple analyses live in one process — anything that assumes "one run, then exit" will bite later.

### [PRINCIPLES-REPORT-NOT-DICTATE] We report, we don't dictate

Deslop states what it measured and how it computed it. It never tells the reader what to do about a finding — not a human, not an agent.

**Banned on every surface** — pair evidence copy, severity copy, hover text, MCP responses, logs:

- Directives: *"Safe to extract"*, *"Verify before extracting"*, *"Review the locations"*, *"read both before merging"*, *"treat as a hint"*.
- Names that are directives: *act-now*, *action sentence*, *recommendation*.
- Safety and worth claims: *"safe to"*, *"worth extracting"*, *"you should"*. Whether extraction is safe depends on facts Deslop never measured.

**Required instead:** cluster surfaces state membership and mass. An explicit pair surface may state what those two endpoints measured, such as *"These two slices are byte-equivalent after whitespace folding."*

A threshold may describe where **Deslop's own** behaviour changes — what it admits, hides, or how it ranks. Never what the reader's should.

### [PRINCIPLES-ONE-CALCULATION] Every figure is computed once, in the engine

A *figure* is any number, label, verdict or ordering the product asserts about
duplication: a percentage, a confidence, a severity band, an occurrence count, a
rank, a threshold comparison, a classification, a plain-English reading of any of
those. Every figure is computed exactly once, in `deslop-core`, and carried on the
wire. No client — VS Code extension host, webview, JetBrains plugin, website, or
future editor integration — may derive one.

The reason is the accuracy contract, not tidiness. A client-side copy of a formula
is a second engine that ships on its own release cadence: it drifts, and when it
drifts the user is shown a figure the report never made. The failure is silent by
construction — nothing crashes, the number is simply wrong. Two shipped instances of
exactly this: the rank percentile, re-derived from array position so any filtered or
projected list silently rebanded every cluster in it; and the folder duplication
percentage, summed and divided in TypeScript beside an engine that had already
computed it.

What a client may do:

- **Render one wire value.** Choosing decimal places, truncating a percentage to a
  whole number for a narrow row, quantising one value to a glyph or a CSS width,
  thousands separators. One value in, one presentation out.
- **Look up a static label.** Language id to display name, severity band to glyph, or pair classification to a title inside an explicit pair view. A lookup table is not a calculation.
- **Run view mechanics.** Loop indices, spinner frames, path-segment splitting for a
  tree, byte offsets to editor coordinates, and comparators or aggregates over a
  *client-filtered* subset — the engine cannot see the user's active facet filter, so
  ordering that subset is the client's job. Such keys must be built from engine values
  and must never surface as a displayed figure: a displayed group figure is the
  engine's value on the group's worst member, selected by the engine's rank, never a
  maximum or a sum recomputed here.

One named exception, so it cannot grow quietly: the Duplication webview tints a
per-file / per-folder percentage on a three-step heat scale (`percentColor` in
`webview-ui/src/duplication/main.tsx`). It classifies a wire value against UI-owned
cut points, which the rule above otherwise forbids. It is allowed because it produces
a colour and no figure, the engine has no duplication heat band to carry, and the row
prints the exact percentage beside the tint. It is the only such site; a second one is
a defect, and if the engine ever gains a heat band this moves onto it.

What a client may never do: apply another threshold constant, classify a value into bands, combine two wire values into a third visible figure, or word its own verdict. Cluster identity, canonical extent, occurrence membership, `mass`, `rank`, and `rank_band` arrive stamped. Pair fields such as structural, Jaccard, embedding, content, admission result, and classification arrive only in an explicit two-endpoint response. Each is rendered in its own surface without crossing the boundary.
