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

## TODO

### P0 Scaffold — COMPLETE
- [x] Workspace: `crates/codededup-core` (lib), `crates/codededup` (bin)
- [x] Strict `[workspace.lints]`: clippy pedantic + nursery, `unsafe_code = "deny"`, `expect_used = "deny"`, `arithmetic_side_effects = "deny"`
- [x] `Makefile` with exactly 7 targets: build, test, lint, fmt, clean, ci, setup (cross-platform, AgentPMO-stamped)
- [x] `coverage-thresholds.json` at repo root (ratcheted to 37)
- [x] `.github/workflows/ci.yml` runs `make ci`; pinned dep versions; 10-min timeout
- [x] `.devcontainer/devcontainer.json` mirrors CI versions
- [x] `tracing` + `tracing-subscriber` wired; workspace lints forbid `print_stdout`/`print_stderr`
- [x] AgentPMO remediation: CLAUDE.md canonical, pointer files (AGENTS.md, .clinerules, .cursorrules, .windsurfrules, copilot-instructions, opencode.json), `.claude/skills/{ci-prep,code-dedup,submit-pr}`, PR template
- [x] E2E: `--version`, `--help` mentions `--min-nodes`, empty-path-no-panic — 3/3 green
- [x] `make ci` green (lint + fmt + test + build)

### P1 C# parse + normalize
- [x] `src/state.rs` — `FileId ↔ path` registry (only global state; scaffolded)
- [ ] `LanguageParser` trait in core
- [ ] `tree-sitter-c-sharp` impl; version pinned in CI + devcontainer
- [ ] `NormalizedNode { kind, children, byte_range, file_id }`
- [ ] Normalization collapses identifiers, literals, comments, whitespace
- [ ] `ignore`-crate file walk
- [ ] `--debug-ast` dump
- [ ] Fixture `tests/fixtures/csharp-small/`
- [ ] E2E: parse fixture, golden AST dump

### P2 Structural fingerprint + exact clusters
- [ ] Bottom-up Merkle hash per subtree
- [ ] `--min-nodes` flag (default 30)
- [ ] Hash-bucket clustering
- [ ] Ranking `count × (size−1) × log(loc)`
- [ ] Text + JSON renderer (stable versioned schema)
- [ ] `--format`, `--output` flags
- [ ] Byte ranges are source of truth; lines derived
- [ ] E2E on C# fixture with planted Type-1/2 clones; golden JSON
- [ ] Tune `--min-nodes` default on real C# repo

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
