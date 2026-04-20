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
- **P5 Embedding pass (hybrid)** — Ollama + `nomic-embed-code`, HNSW (`usearch`), fuse via max-normalized sum (never average). Cache `(content_hash, model_id, version)`.
- **P6 Harden** — `--incremental`, perf pass (<30s/100K LOC C#, no embeddings), coverage ratchet.

## Non-goals
No LSP/daemon, no remote APIs, no execution validation (HyClone), no cross-language detection, no auto-fix, no unit tests.

## Risks
`--min-nodes` default is a guess — tune on real repos. Sibling-extension may miss Type-3s that need tree-edit-distance. HNSW determinism is per-machine only. Ollama absence: CI runs P3 path by default, nightly runs P5.

---

## Current state (summary)

- **P0, P1, P2, P3, P4 complete.** C#, Rust, and Python Type-1 / Type-2 / Type-3 clone detection works end-to-end through the CLI, with per-cluster `{structural, token_jaccard, embedding_cos, fused}` signal breakdown in the JSON report. Multi-language runs dispatch per-file by extension.
- `make ci` green: 8/8 e2e tests (csharp Type-2, csharp Type-3, rust Type-2, python Type-2, mixed 3-language), clippy clean (pedantic + nursery), rustfmt clean, coverage **93.2% ≥ 93% threshold** (ratcheted from 87).
- Verified non-destructively against a real 63-file C# repo (TradiSite backend): 17K fingerprints, 2040 clusters ranked worst-first, no panics, no source modification. Top offenders correctly surface generated GraphQL `.g.cs` duplication and test-fixture boilerplate.
- GitHub repo settings applied (squash-only, auto-merge, delete-on-merge, wiki/projects off, discussions on, ruleset "Protect main" requires PR + CI check).
- `float_arithmetic = "deny"` removed from the lint profile (with rationale comment in `Cargo.toml`); AgentPMO template updated with the same rationale so other repos don't inherit the footgun.
- Spec IDs converted to hierarchical `[GROUP-TOPIC-DETAIL]` form; every module references the IDs it implements AND the academic work those IDs cite (Baxter 1998, Chilowicz 2009, SourcererCC, ensemble-LLM 2025).
- Pluggable by construction: `PairScore` carries a third `embedding_cos` slot so P5 is additive; fingerprints are keyed by `(file_id, byte_range)` so P6 file-watcher incremental updates slot into the same cache keys.

**Next up (P4.1):** self-documenting JSON + three-format output (JSON canonical, text = AI-readable terse, HTML = human-readable). Embed `schema_doc`, per-cluster `interpretation`, and top-level `action_hints` so reports are understandable cold without a side-channel doc. Text and HTML are **derived** from JSON — `--from-report <file.json>` re-renders without re-analysing. Bumps `report_schema_version` to 2.

**After that (P5):** embedding pass via `EmbeddingProvider` trait — pluggable provider (`--embedding-provider`, default `ollama`) and model (`--embedding-model`, default `nomic-embed-code`) per [FUSION-EMBED-PROVIDER]. Fuse via max-normalized sum (never average) per the ensemble-LLM 2025 finding. Cache by `(content_hash, provider_id, model_id, model_version)`. The `embedding_cos` slot in `PairScore` / `ReportSignals` is already reserved, so P5 is additive — no schema bump.

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

### P4.1 Self-documenting report + three-format output — TODO
Output contract: JSON is canonical ([PRINCIPLES-AUDIENCE-AGENT]); text is AI-readable terse; HTML is human-readable. Text + HTML are **derived** from the JSON — nothing lives in two places. Default: emit all three. Flags suppress individual formats.

- [ ] Embed `schema_doc` at JSON top level: field-by-field explanations, signal semantics (what `structural=1.0` vs `token_jaccard=0.97` mean), ranking formula, byte-range conventions, clone taxonomy. Ship via `include_str!` so it can't drift from the schema.
- [ ] Embed per-cluster `interpretation: String` computed from the signal combination — one line like `"Type-1 exact clone, safe to extract"`, `"Type-3 near-miss, review before merging"`, `"Low-information LSH-only match, treat as hint"`.
- [ ] Embed top-level `action_hints: Vec<ActionHint>` — short playbook entries: `"high structural + high jaccard → extract shared function"`, `"low structural + high jaccard → Type-3 candidate, may need tree-edit-distance verification"`, etc.
- [ ] Replace `--format={text,json}` with **default-emit-all-three**: JSON + text + HTML. New suppression flags `--nojson`, `--notext`, `--nohtml` (at least one format must remain enabled or the CLI exits non-zero with a clear message).
- [ ] When `--output <path>` is set, write `<path>.json`, `<path>.txt`, `<path>.html`. When omitted, write `codededup-report.{json,txt,html}` in CWD. Never interleave three streams on stdout.
- [ ] `--from-report <file.json>` takes an existing canonical report as input, skips analysis, and re-renders the text + HTML views. Makes re-rendering deterministic and cheap; keeps the rendering pipeline testable in isolation.
- [ ] HTML renderer: single-file output, inline CSS, no JS, no external fonts. Renders per-cluster `summary`, `interpretation`, `signals`, first N occurrences, and a collapsed `<details>` for the rest. Includes the `schema_doc` and `action_hints` in a header section so a human opening the file cold understands what they're looking at.
- [ ] Text renderer migrates from an ad-hoc `String`-builder in `codededup/src/main.rs` to a `codededup-core::render::text` module that takes `&Report` → `String`, so `--from-report` and tests reuse it.
- [ ] HTML renderer lives in `codededup-core::render::html` alongside the text one.
- [ ] Ratchet coverage to cover the new renderers + `--from-report` round-trip.
- [ ] E2E: run once with default flags, assert all three files exist. Run with `--nojson --nohtml`, assert only text. Run with `--from-report` on a committed golden JSON, assert text + HTML derived outputs match expectations.
- [ ] Update CLI `--help` so the three-format default and `--no*` flags are discoverable by agents.
- [ ] Update `docs/specs/SPEC.md` [OUTPUT-SCHEMA-JSON] to document `schema_doc`, per-cluster `interpretation`, and `action_hints` as required fields at `report_schema_version = 2`; bump `REPORT_SCHEMA_VERSION` to 2.

### P5 Embedding pass (hybrid completion)
- [ ] Ollama client; runtime detection
- [ ] `--embeddings={auto,required,off}`
- [ ] Pin `nomic-embed-code` id + version in cache + report header
- [ ] Embed subtrees ≥ `--min-nodes` from P2
- [ ] HNSW via `usearch` (pure Rust); fixed seed + params
- [ ] Top-k cosine → embedding candidate pairs
- [ ] Fuse 3 signals via **max-normalized sum**
- [ ] Cache at `.codededup-cache/` keyed by `(content_hash, model_id, version)`
- [ ] Fixture with Type-4 (iterative/recursive, LINQ/foreach) — P3 misses, P5 catches
- [ ] Nightly CI with Ollama

### P6 Harden
- [ ] `--incremental` keyed by file content hash
- [ ] Perf: <30s on 100K-LOC C# (no embeddings)
- [ ] Ratchet coverage every PR
- [ ] Fixture-per-bug (CLAUDE.md Bug Fix Process)




