# CodeDedup — Plan

Sibling to [SPEC.md](SPEC.md). **Priority: ship C# CLI fast for feedback.** Rust/Python and embeddings come after the deterministic core is proven on real C# code.

## Principles
- C# end-to-end first. Other languages reuse the `LanguageParser` trait.
- Deterministic core (AST + token LSH) before learned signal (embeddings). Hybrid is SOTA per SPEC §3; sequence matters.
- `codededup-core` lib + thin `codededup` bin from day one (LSP-friendly per CLAUDE.md).
- Coarse e2e tests only, golden JSON against fixtures. Byte ranges everywhere.
- Ratchet `coverage-thresholds.json` upward only.

## Phases
- **P0 Scaffold** — workspace, lints, `Makefile` (7 targets), CI, tracing, first passing e2e.
- **P1 C# parse + normalize** — `LanguageParser` trait, `tree-sitter-c-sharp`, `NormalizedNode`, `ignore`-crate file walk, `src/state.rs` registry.
- **P2 Structural fingerprint + exact clusters** — Merkle subtree hash, `--min-nodes`, ranking `count × (size−1) × log(loc)`, text + JSON renderer. First useful C# report.
- **P3 Sibling extension + token MinHash/LSH** — Type-3 for C#. Per-pair scores `(structural, jaccard)`, transitive-closure clusters, signal breakdown in report. **Ship. Get feedback.**
- **P4 Rust + Python** — new `LanguageParser` impls only; versions pinned in `Cargo.toml` + CI + Dockerfile.
- **P4.1 Self-documenting report + three-format output** — JSON is canonical; text (AI-readable terse) and HTML (human-readable) are derived from it. Embed `schema_doc`, per-cluster `interpretation`, top-level `action_hints`. `--from-report` re-renders without re-analysing. Bump `report_schema_version` to 2.
- **P4.2 Exclusion configuration** — simple `.codededup.toml` config with two tiers: `exclude` (skip parsing entirely) and `report_hide` (analyse for duplication but omit from report unless a visible file duplicates them). Per-language sections + a shared default. Implements [EXCLUSION-CONFIG].
- **P5 Embedding pass (hybrid)** — Ollama + `nomic-embed-code`, HNSW (`usearch`), fuse via max-normalized sum (never average). Cache `(content_hash, model_id, version)`.
- **P6 Harden** — `--incremental`, perf pass (<30s/100K LOC C#, no embeddings), coverage ratchet.

## Non-goals
No LSP/daemon, no remote APIs, no execution validation (HyClone), no cross-language detection, no auto-fix, no unit tests.

## Risks
`--min-nodes` default is a guess — tune on real repos. Sibling-extension may miss Type-3s that need tree-edit-distance. HNSW determinism is per-machine only. Ollama absence: CI runs P3 path by default, nightly runs P5.

---

## Current state (summary)

- **P0 – P5 complete.** C#, Rust, and Python Type-1 / Type-2 / Type-3 / Type-4 clone detection works end-to-end through the CLI. Reports are self-documenting (embedded `schema_doc`, per-cluster `interpretation`, top-level `action_hints`) and emitted in three formats by default (canonical JSON, terse AI-readable text, single-file HTML). Text and HTML are derived from JSON; `--from-report` re-renders a cached JSON without re-analysing. Exclusion config (`.codededup.toml`) supports two tiers: `exclude` (skip parsing) and `report_hide` (analyse but hide hidden-only clusters), with per-language overlays. Embedding pass (`--embeddings={auto,required,off}`) fuses cosine similarity from a pluggable `EmbeddingProvider` (ships with `ollama` + `stub` providers) via HNSW top-k into the candidate-pair union and the fused score. Provenance `(provider_id, model_id, model_version, dimensions)` is pinned in every report; embeddings are cached by `(content_hash, provider, model, version)` under `.codededup-cache/`.
- `make ci` green: 29/29 e2e tests (Ollama-gated test runs under `make ci-ollama` via the `ollama_` name prefix), clippy clean (pedantic + nursery), rustfmt clean, coverage **94.0% ≥ 94% threshold** (held steady through P5 by adding a `stub` provider that exercises the trait / cache / HNSW / pipeline path without needing a live daemon; `embedding/ollama.rs` is excluded from coverage because it's an HTTP client exercised only by `make ci-ollama`).
- Verified non-destructively against a real 63-file C# repo (TradiSite backend): 17K fingerprints, 2040 clusters ranked worst-first, no panics, no source modification. Top offenders correctly surface generated GraphQL `.g.cs` duplication and test-fixture boilerplate.
- GitHub repo settings applied (squash-only, auto-merge, delete-on-merge, wiki/projects off, discussions on, ruleset "Protect main" requires PR + CI check).
- `float_arithmetic = "deny"` removed from the lint profile (with rationale comment in `Cargo.toml`); AgentPMO template updated with the same rationale so other repos don't inherit the footgun.
- Spec IDs converted to hierarchical `[GROUP-TOPIC-DETAIL]` form; every module references the IDs it implements AND the academic work those IDs cite (Baxter 1998, Chilowicz 2009, SourcererCC, ensemble-LLM 2025).
- Pluggable by construction: `PairScore` carries a third `embedding_cos` slot so P5 is additive; fingerprints are keyed by `(file_id, byte_range)` so P6 file-watcher incremental updates slot into the same cache keys.

**Next up (P6):** hardening pass — `--incremental` keyed by content hash, perf target <30 s on 100 K-LOC C# (no embeddings), fixture-per-bug workflow from CLAUDE.md, and continued coverage ratchet. P5 groundwork (cache keyed by `(content_hash, provider_id, model_id, model_version)`, fingerprints keyed by `(file_id, byte_range)`) is already in place, so the incremental pipeline slots into the same cache surface.

## TODO

### P0 Scaffold — COMPLETE
- [x] Workspace: `crates/codededup-core` (lib), `crates/codededup` (bin)
- [x] Strict `[workspace.lints]`: clippy pedantic + nursery, `unsafe_code = "deny"`, `expect_used = "deny"`, `arithmetic_side_effects = "deny"`
- [x] `Makefile` with exactly 7 targets: build, test, lint, fmt, clean, ci, setup (cross-platform, AgentPMO-stamped)
- [x] `coverage-thresholds.json` at repo root (currently **87** — ratcheted from 0 → 37 → 87 as tests landed)
- [x] `.github/workflows/ci.yml` runs `make ci`; pinned dep versions; 10-min timeout
- [x] `.devcontainer/devcontainer.json` mirrors CI versions
- [x] `tracing` + `tracing-subscriber` wired; workspace lints forbid `print_stdout`/`print_stderr`
- [x] AgentPMO remediation: CLAUDE.md canonical, pointer files (AGENTS.md, .clinerules, .cursorrules, .windsurfrules, copilot-instructions, opencode.json), `.claude/skills/{ci-prep,code-dedup,submit-pr}`, PR template
- [x] E2E: `--version`, `--help` mentions `--min-nodes`, empty-path-no-panic — 3/3 green
- [x] `make ci` green (lint + fmt + test + build)

### P1 C# parse + normalize — COMPLETE
- [x] `src/state.rs` — `FileId ↔ path` registry (only global state)
- [x] `LanguageParser` trait in core (implements [PIPELINE-LANG-TRAIT])
- [x] `tree-sitter-c-sharp` impl (pinned `=0.21.3`; version pinned in Cargo.toml)
- [x] `NormalizedNode { kind, children, byte_range, file_id }`
- [x] Normalization collapses identifiers, literals, comments, whitespace (`__ident__` / `__literal__` / dropped trivia)
- [x] `ignore`-crate file walk (implements [PIPELINE-DISCOVER-FILES])
- [x] Fixture `tests/fixtures/csharp-small/` with Alpha.cs + Beta.cs (Type-2 clone pair)
- [x] E2E: detects Type-2 clone between Alpha.cs and Beta.cs in JSON report
- [ ] `--debug-ast` dump (deferred — not blocking ship)
- [ ] Pin `tree-sitter-c-sharp` version in `.github/workflows/ci.yml` too (currently only in Cargo.toml)
- [ ] Golden AST dump test (deferred — e2e JSON assertion covers the contract)

### P2 Structural fingerprint + exact clusters — COMPLETE
- [x] Bottom-up Merkle hash per subtree (blake3)
- [x] `--min-nodes` flag (default 30)
- [x] Hash-bucket clustering (implements [PIPELINE-CLUSTER-EXACT])
- [x] Ranking `count × (size−1) × log2(spanned+1)` (implements [PIPELINE-RANK-WORST-FIRST])
- [x] Text + JSON renderer (stable versioned schema, `report_schema_version = 1`)
- [x] `--format`, `--output` flags
- [x] Byte ranges are source of truth; lines derived
- [x] E2E on C# fixture with planted Type-2 clone; JSON assertion
- [ ] Tune `--min-nodes` default on real C# repo (needs real corpus)

### P3 Sibling extension + token LSH (Type-3 for C#) — COMPLETE
- [x] Sibling-extension over exact clusters (`crates/codededup-core/src/sibling.rs`, Chilowicz 2009)
- [x] Normalized token stream per file (`crates/codededup-core/src/tokens.rs`)
- [x] k-gram → MinHash → LSH buckets (k=5, 128-wide signature, 32 bands × 4 rows) (`crates/codededup-core/src/lsh.rs`)
- [x] Candidate union: exact ∪ sibling ∪ LSH (`crates/codededup-core/src/pair.rs`)
- [x] Pair scores `(structural_sim, token_jaccard, embedding_cos)` in [0,1]; embedding slot reserved for P5
- [x] Transitive-closure clustering (iterative union-find with LSH-only Jaccard + min-node-count floors so tiny sibling windows do not mega-cluster)
- [x] Report shows per-cluster signal breakdown (`ReportSignals { structural, token_jaccard, embedding_cos, fused }`)
- [x] Fixture with hand-crafted Type-3 (`crates/codededup/tests/fixtures/csharp-type3/{Delta,Epsilon}.cs`); e2e asserts cross-file cluster with `structural=0.0` and non-empty `token_jaccard`
- [x] Coverage ratcheted 87 → 93
- [x] **SHIPPED C# CLI** (`cargo run --release -- <dir> --format json --min-nodes 15`)

### P4 Rust + Python — COMPLETE
- [x] `tree-sitter-rust` impl (`crates/codededup-core/src/lang/rust_lang.rs`, grammar pinned `=0.21.2`) + Type-2 fixture + e2e
- [x] `tree-sitter-python` impl (`crates/codededup-core/src/lang/python.rs`, grammar pinned `=0.21.0`) + Type-2 fixture + e2e
- [x] Mixed-language fixture (`cs/Lib.cs` + `rs/lib.rs` + `py/lib.py`) + e2e asserting 3 files analysed
- [x] Shared walking / interning plumbing factored to `crates/codededup-core/src/lang/shared.rs` so each language module is ~80 LOC of `normalise_kind` + boilerplate
- [x] Grammar versions pinned in `Cargo.toml` (source of truth — CI workflow picks them up automatically; Dockerfile will mirror when P6 ships)

### P4.1 Self-documenting report + three-format output — COMPLETE
Output contract: JSON is canonical ([PRINCIPLES-AUDIENCE-AGENT]); text is AI-readable terse; HTML is human-readable. Text + HTML are **derived** from the JSON — nothing lives in two places. Default: emit all three. Flags suppress individual formats.

- [x] Embed `schema_doc` at JSON top level: field-by-field explanations, signal semantics, ranking formula, byte-range conventions, clone taxonomy. Shipped via `include_str!` from [`docs/specs/REPORTING-CONTEXT.md`](../specs/REPORTING-CONTEXT.md) so it can't drift from the schema.
- [x] Embed per-cluster `interpretation: String` computed from the signal combination — see `codededup-core::report::interpret`.
- [x] Embed top-level `action_hints: Vec<ActionHint>` — short playbook entries derived from the "Reading the signals together" table in the reporting context doc.
- [x] Replaced `--format={text,json}` with default-emit-all-three: JSON + text + HTML. Suppression flags `--nojson`, `--notext`, `--nohtml`; CLI exits non-zero when all three are suppressed.
- [x] `--output <path>` writes `<path>.json`, `<path>.txt`, `<path>.html`. Defaults to `codededup-report.{json,txt,html}` in CWD. Nothing written to stdout.
- [x] `--from-report <file.json>` skips analysis and re-renders text + HTML from a canonical JSON input.
- [x] HTML renderer (`codededup-core::render::html`): single-file output, inline CSS, no JS, no external fonts. Renders per-cluster summary / interpretation / signals; first 8 occurrences expanded, rest in a collapsed `<details>`. Header carries the action hints; schema_doc lives in a collapsed reference panel.
- [x] Text renderer migrated to `codededup-core::render::text` — takes `&Report` → `String`, shared by live runs and `--from-report`.
- [x] Coverage ratcheted 93 → 94 covering the renderers + `--from-report` round-trip.
- [x] E2E: `default_run_emits_all_three_formats`, `suppression_flags_leave_only_enabled_formats`, `suppressing_every_format_is_an_error`, `from_report_rerenders_without_analysing`, `default_output_written_to_current_directory`.
- [x] CLI `--help` advertises `--min-nodes`, `--nojson`, `--notext`, `--nohtml`, `--from-report`, `--config` (asserted in `prints_help_and_mentions_min_nodes_flag`).
- [x] [OUTPUT-SCHEMA-JSON] documented in `docs/specs/SPEC.md`; `REPORT_SCHEMA_VERSION` bumped to 2. Report now carries `schema_doc`, `action_hints`, per-cluster `interpretation`, per-occurrence `hidden`, and top-level `clusters_hidden`.

### P4.2 Exclusion configuration — COMPLETE
Implements [EXCLUSION-CONFIG]. Two tiers of exclusion driven by a single `.codededup.toml` file in the scan root (or `--config <path>`). Generated code is the motivating case: we want to know when hand-written code duplicates a generated file, but we don't want the generated file itself to dominate the top of the report.

- [x] Config schema (TOML) with a `[defaults]` section and optional `[language.<name>]` sections. Keys `exclude: Vec<String>` and `report_hide: Vec<String>`. Patterns use `ignore::gitignore` semantics for familiarity.
- [x] `--config <path>` flag (optional). When absent, the pipeline looks for `.codededup.toml` next to the scan root and falls back to empty config. `info`-level log entry records which config was loaded.
- [x] `ExclusionConfig` lives in `codededup-core::config`, parsed via the `toml` crate. Per-language sections extend `[defaults]` — a `.rs` file is tested against `defaults.exclude ∪ language.rust.exclude`.
- [x] `exclude` patterns applied in `discover_files` — dropped paths are never registered, never counted in `files_analysed`, never parsed.
- [x] `report_hide` evaluated at render time. Hidden-only clusters are dropped and counted in `clusters_hidden`; mixed clusters (regular code duplicating generated code) stay intact.
- [x] Per-occurrence `hidden: bool` field in JSON and HTML (CSS-dimmed) for downstream consumers.
- [x] E2E: `report_hide_keeps_mixed_cluster_and_flags_hidden_occurrence`, `report_hide_drops_cluster_when_all_members_hidden`, `report_hide_per_language_overlay_flags_csharp_only`.
- [x] E2E: `exclude_pattern_drops_file_from_discovery`, `exclude_per_language_overlay_scoped_to_its_language`, `default_config_file_in_scan_root_is_loaded`, `malformed_config_file_reports_error`.
- [x] `docs/specs/SPEC.md` — added `[EXCLUSION-CONFIG]` section; cross-referenced from `[PIPELINE-DISCOVER-FILES]` and `[OUTPUT-SCHEMA-JSON]`.

### P5 Embedding pass (hybrid completion) — COMPLETE
- [x] `EmbeddingProvider` trait at `crates/codededup-core/src/embedding/provider.rs` — pluggable per [FUSION-EMBED-PROVIDER]; providers selected by string id at runtime.
- [x] Ollama HTTP client (`embedding/ollama.rs`) — loopback-only, no TLS dep; `/api/tags` → digest; `/api/embeddings` → vector. Default model `nomic-embed-text` (137 M params, 768-dim, Apache 2.0). Rationale: ensemble-LLM 2025 finding that "smaller embedding sizes, smaller tokenizer vocabularies and tailored datasets are advantageous"; user-overridable via `--embedding-model` (swap to `nomic-embed-code` / `codet5p` / `unixcoder` once pulled locally).
- [x] Stub provider (`embedding/stub.rs`) — deterministic BLAKE3-derived 64-dim vectors, spec-blessed as the `stub` slot. Lets `make ci` exercise the trait / cache / HNSW / pipeline path without needing a live Ollama daemon.
- [x] `--embeddings={auto,required,off}` (default `off`; `auto` probes and falls back with `tracing::warn!`; `required` propagates failure as a non-zero exit).
- [x] `--embedding-provider` / `--embedding-model` / `--embedding-endpoint` CLI surface; invalid values rejected with a clear error.
- [x] HNSW via `instant-distance 0.6.1` (pure Rust, zero C deps); deterministic seed; cosine distance; top-5 neighbours with cosine-similarity floor 0.80.
- [x] `PairScore.embedding_cos` populated by the ANN pass; fused score now genuinely sums three signals per [FUSION-STRATEGY-MAX-SUM]; cluster-level mean includes the embedding axis.
- [x] On-disk cache at `<scan_root>/.codededup-cache/embeddings/<provider>/<model>/<version>/<content_hash>.bin` — little-endian `f32` blobs, no external serializer dep. Round-trip verified by `stub_provider_populates_embedding_cache`.
- [x] Report schema carries `embedding_provenance: Option<EmbeddingProvenance>` (`provider_id`, `model_id`, `model_version`, `dimensions`). Text + HTML renderers surface the provenance line; JSON is canonical.
- [x] Type-4 fixture (`crates/codededup/tests/fixtures/csharp-type4/{Recursive,Iterative}.cs`) — recursive vs. iterative factorial / fibonacci / sum-to-n. Verified against live Ollama: cluster `structural=0.00, token_jaccard=1.00, embedding_cos=1.00` surfaces as a fused cluster that the pre-P5 pipeline never saw.
- [x] `make ci-ollama` target — pulls `nomic-embed-text`, runs the `ollama_`-prefixed tests (`cargo test ollama_`). `make ci` filters them out via `--skip ollama_` so the default pipeline needs no external service.
- [x] Coverage ratchet: `embedding/ollama.rs` is excluded from measurement (it's an HTTP client exercised only by `make ci-ollama`); every other P5 file is covered ≥ 93% via the stub-provider E2E tests.

### P6 Harden
- [ ] `--incremental` keyed by file content hash
- [ ] Perf: <30s on 100K-LOC C# (no embeddings)
- [ ] Ratchet coverage every PR
- [ ] Fixture-per-bug (CLAUDE.md Bug Fix Process)




