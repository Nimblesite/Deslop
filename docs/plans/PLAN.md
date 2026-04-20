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
- **P5 Embedding pass (hybrid)** — Ollama + `nomic-embed-code`, HNSW (`usearch`), fuse via max-normalized sum (never average). Cache `(content_hash, model_id, version)`.
- **P6 Harden** — `--incremental`, perf pass (<30s/100K LOC C#, no embeddings), coverage ratchet.

## Non-goals
No LSP/daemon, no remote APIs, no execution validation (HyClone), no cross-language detection, no auto-fix, no unit tests.

## Risks
`--min-nodes` default is a guess — tune on real repos. Sibling-extension may miss Type-3s that need tree-edit-distance. HNSW determinism is per-machine only. Ollama absence: CI runs P3 path by default, nightly runs P5.

---

## Current state (summary)

- **P0, P1, P2 complete.** C# Type-1 and Type-2 clone detection works end-to-end through the CLI.
- `make ci` green: 4/4 e2e tests, clippy clean (pedantic + nursery), rustfmt clean, coverage **88.8% ≥ 87% threshold**.
- GitHub repo settings applied (squash-only, auto-merge, delete-on-merge, wiki/projects off, discussions on, ruleset "Protect main" requires PR + CI check).
- `float_arithmetic = "deny"` removed from the lint profile (with rationale comment in `Cargo.toml`); AgentPMO template updated with the same rationale so other repos don't inherit the footgun.
- Spec IDs converted to hierarchical `[GROUP-TOPIC-DETAIL]` form; all new code comments reference the IDs they implement.

**Next up (P3):** sibling-extension over exact clusters + token MinHash/LSH → union candidates → per-cluster signal breakdown in JSON. That's the point where we ship for feedback.

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

### P3 Sibling extension + token LSH (Type-3 for C#)
- [ ] Sibling-extension over exact clusters
- [ ] Normalized token stream per file
- [ ] k-gram → MinHash → LSH buckets (k=5)
- [ ] Candidate union: exact ∪ sibling ∪ LSH
- [ ] Pair scores `(structural_sim, token_jaccard)` in [0,1]
- [ ] Transitive-closure clustering
- [ ] Report shows per-cluster signal breakdown
- [ ] Fixture with hand-crafted Type-3; golden JSON
- [ ] **SHIP C# CLI**

### P4 Rust + Python
- [ ] `tree-sitter-rust` impl + fixture + golden
- [ ] `tree-sitter-python` impl + fixture + golden
- [ ] Mixed-language fixture
- [ ] All grammar versions pinned in CI + Dockerfile

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
