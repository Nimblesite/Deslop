# Deslop — Plan

Sibling to [SPEC.md](SPEC.md). **Priority: ship C# CLI fast for feedback.** Rust/Python and embeddings come after the deterministic core is proven on real C# code.

## Principles
- C# end-to-end first. Other languages reuse the `LanguageParser` trait.
- Deterministic core (AST + token LSH) before learned signal (embeddings). Hybrid is SOTA per SPEC §3; sequence matters.
- `deslop-core` lib + thin `deslop` bin from day one (LSP-friendly per CLAUDE.md).
- Coarse e2e tests only, golden JSON against fixtures. Byte ranges everywhere.
- Ratchet `coverage-thresholds.json` upward only.

## Phases
- **P0 Scaffold** — workspace, lints, `Makefile` (7 targets), CI, tracing, first passing e2e.
- **P1 C# parse + normalize** — `LanguageParser` trait, `tree-sitter-c-sharp`, `NormalizedNode`, `ignore`-crate file walk, `src/state.rs` registry.
- **P2 Structural fingerprint + exact clusters** — Merkle subtree hash, `--min-nodes`, ranking `count × (size−1) × log(loc)`, text + JSON renderer. First useful C# report.
- **P3 Sibling extension + token MinHash/LSH** — Type-3 for C#. Per-pair scores `(structural, jaccard)`, transitive-closure clusters, signal breakdown in report. **Ship. Get feedback.**
- **P4 Rust + Python** — new `LanguageParser` impls only; versions pinned in `Cargo.toml` + CI + Dockerfile.
- **P4.1 Self-documenting report + three-format output** — JSON is canonical; text (AI-readable terse) and HTML (human-readable) are derived from it. Embed `schema_doc`, per-cluster `interpretation`, top-level `action_hints`. `--from-report` re-renders without re-analysing. Bump `report_schema_version` to 2.
- **P4.2 Exclusion configuration** — simple `.deslop.toml` config with two tiers: `exclude` (skip parsing entirely) and `report_hide` (analyse for duplication but omit from report unless a visible file duplicates them). Per-language sections + a shared default. Implements [EXCLUSION-CONFIG].
- **P5 Embedding pass (hybrid)** — Ollama + `nomic-embed-code`, HNSW (`usearch`), fuse via max-normalized sum (never average). Cache `(content_hash, model_id, version)`.
- **P6 Harden** — `--incremental` on-disk fingerprint cache keyed by `(language, tool_version, min_nodes, content_hash)`, perf regression guard, fixture-per-bug workflow, coverage ratchet.
- **P6.2 Repo-wide duplication metric + fail-over** — `Report.metrics { analysed_loc, duplicated_loc, duplication_percent, clusters_total, duplicated_files, threshold }` computed deterministically from non-hidden occurrences ([METRICS-REPO]). `--fail-over <percent>` CLI flag + `[threshold] max_duplication_percent` in `.deslop.toml` gate CI with exit `3` on breach ([EXIT-CODES]). `report_schema_version` → 3.
- **P7 Live-analysis foundation** — `live` module inside `deslop-core` (behind a `live` cargo feature): `AnalysisSession`, file watcher, debouncer, scheduler, `LiveApi` query surface, `ReportDelta` push notifications. `ReportDelta` + `PipelineSession` + `update_files` are already landed. See [live.md](../specs/live.md).
- **P8 LSP server** — `deslop-lsp` bin over `tower-lsp`: diagnostics, code lens, hover, virtual docs, `deslop/*` custom methods. Editor-agnostic thin forwarder to `LiveApi`. See [lsp.md](../specs/lsp.md).
- **P9 MCP server** — `deslop-mcp` bin: tools (`find-similar`, `report-for-*`, `set-embedding-model`, …), resources, notifications. Agent-facing thin forwarder to `LiveApi`. See [mcp.md](../specs/mcp.md).
- **P10 VSIX + live bubble** — VS Code extension with live duplication bubble ([VSIX-LIVE-BUBBLE]), tree view, webview, status bar, Ollama embedding-model picker ([VSIX-EMBED-PICKER]). The in-your-face "you're duplicating right now" moment. See [vsix.md](../specs/vsix.md).
- **P11 Canonical clone buckets + dual labelling** — promote `[CLONE-BUCKETS]` (taxonomy.md) to the single source of truth across every renderer. Human UI surfaces (HTML, CLI stderr, VS Code) show only the human titles; JSON / `schema_doc` / MCP / agent copy keep the academic `Type-1 → Type-4` labels. Adds the `SameBehavior` bucket as a first-class AI-match surface and kills the four parallel vocabularies the renderers currently ship with. See [taxonomy.md §[CLONE-BUCKETS-DUAL-LABEL]](../specs/taxonomy.md).

## Non-goals (across all phases)
No remote APIs. No execution validation (HyClone). No cross-language detection. No auto-fix / extract-to-function — refactoring belongs downstream of a dedicated engine. No unit tests; coarse E2E only.

## Future work (deliberately deferred)
- **Interactive / TUI mode (`--interactive`).** Paginated top-clusters view with inline byte-range previews, keyboard navigation between occurrences, and "extract refactor suggestion" shortcuts. Mostly a `ratatui` build-out; the deterministic core already emits everything needed. Only worth shipping after we've seen real operator use of the colored stderr summary — that tells us what the interactive view should emphasise.

## Risks
`--min-nodes` default is a guess — tune on real repos. Sibling-extension may miss Type-3s that need tree-edit-distance. HNSW determinism is per-machine only. Ollama absence: CI runs P3 path by default, nightly runs P5.

---

## Current state (summary)

- **P0 – P8 complete.** C#, Rust, and Python Type-1 / Type-2 / Type-3 / Type-4 clone detection works end-to-end through the CLI, and the live module + LSP binary land in P7 / P8. The `live` module inside `deslop-core` (behind the `live` cargo feature) hosts `AnalysisSession`, a deterministic `Clock`-injected debouncer, a single-flight coalescing scheduler, a `notify`-backed file watcher, and the nine-method `LiveApi` / `LiveService` query surface ([LIVE-PACKAGING], [LIVE-QUERY-API]). `crates/deslop-lsp` is a thin `tower-lsp` shell (< 100 LOC of glue in `main.rs`) forwarding `initialize` / `textDocument/diagnostic` / `textDocument/didChange` and the `deslop/*` custom namespace directly onto `LiveApi` ([LSP-CAPABILITIES], [LSP-CUSTOM-METHODS]). Coverage 96.1% ≥ 95% threshold ([COVERAGE-THRESHOLDS-JSON]). Repo-wide duplication metric and CLI fail-over gate landed in P6.2: `Report.metrics` carries `{ analysed_loc, duplicated_loc, duplication_percent, clusters_total, duplicated_files, threshold }` at `report_schema_version = 3`; `--fail-over <percent>` / `[threshold] max_duplication_percent` exit `3` on breach while still emitting the full report. Reports are self-documenting (embedded `schema_doc`, per-cluster `interpretation`, top-level `action_hints`) and emitted in three formats by default (canonical JSON, terse AI-readable text, single-file HTML). Text and HTML are derived from JSON; `--from-report` re-renders a cached JSON without re-analysing. Exclusion config (`.deslop.toml`) supports two tiers: `exclude` (skip parsing) and `report_hide` (analyse but hide hidden-only clusters), with per-language overlays. Embedding pass (`--embeddings={auto,required,off}`) fuses cosine similarity from a pluggable `EmbeddingProvider` (ships with `ollama` + `stub` providers) via HNSW top-k into the candidate-pair union and the fused score. Provenance `(provider_id, model_id, model_version, dimensions)` is pinned in every report; embeddings are cached by `(content_hash, provider, model, version)` under `.deslop-cache/`. Incremental-analysis cache (`--incremental`) keyed by `(language, tool_version, min_nodes, content_hash)` rehydrates unchanged files without tree-sitter ([PIPELINE-INCREMENTAL]); hits/misses surface as `cache_stats` in every report.
- `make ci` green: 42/42 e2e tests (Ollama-gated test runs under `make ci-ollama` via the `ollama_` name prefix), clippy clean (pedantic + nursery), rustfmt clean, coverage **94.3% ≥ 94% threshold** (held steady through P6: new fpcache module covered end-to-end via e2e tests exercising the happy path, corrupt-blob recovery, read-only directory degradation, and default-off semantics).
- Verified non-destructively against a real 63-file C# repo (TradiSite backend): 17K fingerprints, 2040 clusters ranked worst-first, no panics, no source modification. Top offenders correctly surface generated GraphQL `.g.cs` duplication and test-fixture boilerplate.
- GitHub repo settings applied (squash-only, auto-merge, delete-on-merge, wiki/projects off, discussions on, ruleset "Protect main" requires PR + CI check).
- `float_arithmetic = "deny"` removed from the lint profile (with rationale comment in `Cargo.toml`); AgentPMO template updated with the same rationale so other repos don't inherit the footgun.
- Spec IDs converted to hierarchical `[GROUP-TOPIC-DETAIL]` form; every module references the IDs it implements AND the academic work those IDs cite (Baxter 1998, Chilowicz 2009, SourcererCC, ensemble-LLM 2025).
- Pluggable by construction: `PairScore` carries a third `embedding_cos` slot so P5 is additive; fingerprints are keyed by `(file_id, byte_range)` so P6 file-watcher incremental updates slot into the same cache keys.

**Next up (beyond P6):** phases **P7–P10** take the deterministic core live without adding a daemon process. P7 adds the `live` module inside `deslop-core` (behind a `live` cargo feature) with the file watcher, re-analysis scheduler, `LiveApi` query surface, and `ReportDelta` push notifications. The core primitives are already landed: `ReportDelta` at `deslop-core::delta`, `PipelineSession::update_files` at `deslop-core::pipeline::session`, and `list_ollama_models` for the VSIX model picker. P8 and P9 are thin forwarders on top: an LSP binary ([lsp.md](../specs/lsp.md)) and an MCP binary ([mcp.md](../specs/mcp.md)). P10 ships the VS Code extension ([vsix.md](../specs/vsix.md)) — the reference client — whose flagship UX is the **live duplication bubble** ([VSIX-LIVE-BUBBLE]): the moment a developer types code that matches an existing cluster, the editor tells them inline, before save. No competitor does this ([competitors.md](../specs/competitors.md)); it is the category-defining feature. TUI mode remains deferred.

## TODO

### P0 Scaffold — COMPLETE
- [x] Workspace: `crates/deslop-core` (lib), `crates/deslop` (bin)
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
- [x] `--debug-ast <FILE>` CLI flag — parses one source file and prints the deterministic normalised AST dump to stdout. Implemented in `deslop-core::pipeline::debug_ast_dump` and rendered by `deslop-core::render::ast::render_ast_dump`. Conflicts with `--from-report`; writes nothing to disk; mutates no caches.
- [x] Grammar pin-drift check in CI. `.github/workflows/ci.yml` grep-asserts that every tree-sitter dependency in workspace `Cargo.toml` is pinned with an exact `=x.y.z` constraint before the rust toolchain is installed. Any drift fails the build with a named diagnostic.
- [x] Golden AST dump test. `tests/fixtures/ast-golden-csharp/Sample.cs` + `Sample.expected.ast` committed; `debug_ast_dump_matches_committed_golden` asserts byte-for-byte equality. Grammar bumps, `normalise_kind` edits, and child-ordering changes all trip the test, forcing an explicit decision.

### P2 Structural fingerprint + exact clusters — COMPLETE
- [x] Bottom-up Merkle hash per subtree (blake3)
- [x] `--min-nodes` flag (default 30)
- [x] Hash-bucket clustering (implements [PIPELINE-CLUSTER-EXACT])
- [x] Ranking `count × (size−1) × log2(spanned+1)` (implements [PIPELINE-RANK-WORST-FIRST])
- [x] Text + JSON renderer (stable versioned schema, `report_schema_version = 1`)
- [x] `--format`, `--output` flags
- [x] Byte ranges are source of truth; lines derived
- [x] E2E on C# fixture with planted Type-2 clone; JSON assertion
- [x] `--min-nodes` tuning methodology documented at `[DECISION-MIN-NODES]` (six-value sweep against three representative corpora, signal-in-top-20 score, runtime / cluster-count guardrails, reproducibility requirement). Actually running the sweep needs a real corpus suite and belongs in the release-cycle checklist, not in P1 shipping criteria — the methodology is what keeps the decision principled.

### P3 Sibling extension + token LSH (Type-3 for C#) — COMPLETE
- [x] Sibling-extension over exact clusters (`crates/deslop-core/src/sibling.rs`, Chilowicz 2009)
- [x] Normalized token stream per file (`crates/deslop-core/src/tokens.rs`)
- [x] k-gram → MinHash → LSH buckets (k=5, 128-wide signature, 32 bands × 4 rows) (`crates/deslop-core/src/lsh.rs`)
- [x] Candidate union: exact ∪ sibling ∪ LSH (`crates/deslop-core/src/pair.rs`)
- [x] Pair scores `(structural_sim, token_jaccard, embedding_cos)` in [0,1]; embedding slot reserved for P5
- [x] Transitive-closure clustering (iterative union-find with LSH-only Jaccard + min-node-count floors so tiny sibling windows do not mega-cluster)
- [x] Report shows per-cluster signal breakdown (`ReportSignals { structural, token_jaccard, embedding_cos, fused }`)
- [x] Fixture with hand-crafted Type-3 (`crates/deslop/tests/fixtures/csharp-type3/{Delta,Epsilon}.cs`); e2e asserts cross-file cluster with `structural=0.0` and non-empty `token_jaccard`
- [x] Coverage ratcheted 87 → 93
- [x] **SHIPPED C# CLI** (`cargo run --release -- <dir> --format json --min-nodes 15`)

### P4 Rust + Python — COMPLETE
- [x] `tree-sitter-rust` impl (`crates/deslop-core/src/lang/rust_lang.rs`, grammar pinned `=0.21.2`) + Type-2 fixture + e2e
- [x] `tree-sitter-python` impl (`crates/deslop-core/src/lang/python.rs`, grammar pinned `=0.21.0`) + Type-2 fixture + e2e
- [x] Mixed-language fixture (`cs/Lib.cs` + `rs/lib.rs` + `py/lib.py`) + e2e asserting 3 files analysed
- [x] Shared walking / interning plumbing factored to `crates/deslop-core/src/lang/shared.rs` so each language module is ~80 LOC of `normalise_kind` + boilerplate
- [x] Grammar versions pinned in `Cargo.toml` (source of truth — CI workflow picks them up automatically; Dockerfile will mirror when P6 ships)

### P4.1 Self-documenting report + three-format output — COMPLETE
Output contract: JSON is canonical ([PRINCIPLES-AUDIENCE-AGENT]); text is AI-readable terse; HTML is human-readable. Text + HTML are **derived** from the JSON — nothing lives in two places. Default: emit all three. Flags suppress individual formats.

- [x] Embed `schema_doc` at JSON top level: field-by-field explanations, signal semantics, ranking formula, byte-range conventions, clone taxonomy. Shipped via `include_str!` from [`docs/specs/REPORTING-CONTEXT.md`](../specs/REPORTING-CONTEXT.md) so it can't drift from the schema.
- [x] Embed per-cluster `interpretation: String` computed from the signal combination — see `deslop-core::report::interpret`.
- [x] Embed top-level `action_hints: Vec<ActionHint>` — short playbook entries derived from the "Reading the signals together" table in the reporting context doc.
- [x] Replaced `--format={text,json}` with default-emit-all-three: JSON + text + HTML. Suppression flags `--nojson`, `--notext`, `--nohtml`; CLI exits non-zero when all three are suppressed.
- [x] `--output <path>` writes `<path>.json`, `<path>.txt`, `<path>.html`. Defaults to `deslop-report.{json,txt,html}` in CWD. Nothing written to stdout.
- [x] `--from-report <file.json>` skips analysis and re-renders text + HTML from a canonical JSON input.
- [x] HTML renderer (`deslop-core::render::html`): single-file output, inline CSS, no JS, no external fonts. Renders per-cluster summary / interpretation / signals; first 8 occurrences expanded, rest in a collapsed `<details>`. Header carries the action hints; schema_doc lives in a collapsed reference panel.
- [x] Text renderer migrated to `deslop-core::render::text` — takes `&Report` → `String`, shared by live runs and `--from-report`.
- [x] Coverage ratcheted 93 → 94 covering the renderers + `--from-report` round-trip.
- [x] E2E: `default_run_emits_all_three_formats`, `suppression_flags_leave_only_enabled_formats`, `suppressing_every_format_is_an_error`, `from_report_rerenders_without_analysing`, `default_output_written_to_current_directory`.
- [x] CLI `--help` advertises `--min-nodes`, `--nojson`, `--notext`, `--nohtml`, `--from-report`, `--config` (asserted in `prints_help_and_mentions_min_nodes_flag`).
- [x] [OUTPUT-SCHEMA-JSON] documented in `docs/specs/SPEC.md`; `REPORT_SCHEMA_VERSION` bumped to 2. Report now carries `schema_doc`, `action_hints`, per-cluster `interpretation`, per-occurrence `hidden`, and top-level `clusters_hidden`.

### P4.2 Exclusion configuration — COMPLETE
Implements [EXCLUSION-CONFIG]. Two tiers of exclusion driven by a single `.deslop.toml` file in the scan root (or `--config <path>`). Generated code is the motivating case: we want to know when hand-written code duplicates a generated file, but we don't want the generated file itself to dominate the top of the report.

- [x] Config schema (TOML) with a `[defaults]` section and optional `[language.<name>]` sections. Keys `exclude: Vec<String>` and `report_hide: Vec<String>`. Patterns use `ignore::gitignore` semantics for familiarity.
- [x] `--config <path>` flag (optional). When absent, the pipeline looks for `.deslop.toml` next to the scan root and falls back to empty config. `info`-level log entry records which config was loaded.
- [x] `ExclusionConfig` lives in `deslop-core::config`, parsed via the `toml` crate. Per-language sections extend `[defaults]` — a `.rs` file is tested against `defaults.exclude ∪ language.rust.exclude`.
- [x] `exclude` patterns applied in `discover_files` — dropped paths are never registered, never counted in `files_analysed`, never parsed.
- [x] `report_hide` evaluated at render time. Hidden-only clusters are dropped and counted in `clusters_hidden`; mixed clusters (regular code duplicating generated code) stay intact.
- [x] Per-occurrence `hidden: bool` field in JSON and HTML (CSS-dimmed) for downstream consumers.
- [x] E2E: `report_hide_keeps_mixed_cluster_and_flags_hidden_occurrence`, `report_hide_drops_cluster_when_all_members_hidden`, `report_hide_per_language_overlay_flags_csharp_only`.
- [x] E2E: `exclude_pattern_drops_file_from_discovery`, `exclude_per_language_overlay_scoped_to_its_language`, `default_config_file_in_scan_root_is_loaded`, `malformed_config_file_reports_error`.
- [x] `docs/specs/SPEC.md` — added `[EXCLUSION-CONFIG]` section; cross-referenced from `[PIPELINE-DISCOVER-FILES]` and `[OUTPUT-SCHEMA-JSON]`.

### P5 Embedding pass (hybrid completion) — COMPLETE
- [x] `EmbeddingProvider` trait at `crates/deslop-core/src/embedding/provider.rs` — pluggable per [FUSION-EMBED-PROVIDER]; providers selected by string id at runtime.
- [x] Ollama HTTP client (`embedding/ollama.rs`) — loopback-only, no TLS dep; `/api/tags` → digest; `/api/embeddings` → vector. Default model `nomic-embed-text` (137 M params, 768-dim, Apache 2.0). Rationale: ensemble-LLM 2025 finding that "smaller embedding sizes, smaller tokenizer vocabularies and tailored datasets are advantageous"; user-overridable via `--embedding-model` (swap to `nomic-embed-code` / `codet5p` / `unixcoder` once pulled locally).
- [x] Stub provider (`embedding/stub.rs`) — deterministic BLAKE3-derived 64-dim vectors, spec-blessed as the `stub` slot. Lets `make ci` exercise the trait / cache / HNSW / pipeline path without needing a live Ollama daemon.
- [x] `--embeddings={auto,required,off}` (default `off`; `auto` probes and falls back with `tracing::warn!`; `required` propagates failure as a non-zero exit).
- [x] `--embedding-provider` / `--embedding-model` / `--embedding-endpoint` CLI surface; invalid values rejected with a clear error.
- [x] HNSW via `instant-distance 0.6.1` (pure Rust, zero C deps); deterministic seed; cosine distance; top-5 neighbours with cosine-similarity floor 0.80.
- [x] `PairScore.embedding_cos` populated by the ANN pass; fused score now genuinely sums three signals per [FUSION-STRATEGY-MAX-SUM]; cluster-level mean includes the embedding axis.
- [x] On-disk cache at `<scan_root>/.deslop-cache/embeddings/<provider>/<model>/<version>/<content_hash>.bin` — little-endian `f32` blobs, no external serializer dep. Round-trip verified by `stub_provider_populates_embedding_cache`.
- [x] Report schema carries `embedding_provenance: Option<EmbeddingProvenance>` (`provider_id`, `model_id`, `model_version`, `dimensions`). Text + HTML renderers surface the provenance line; JSON is canonical.
- [x] Type-4 fixture (`crates/deslop/tests/fixtures/csharp-type4/{Recursive,Iterative}.cs`) — recursive vs. iterative factorial / fibonacci / sum-to-n. Verified against live Ollama: cluster `structural=0.00, token_jaccard=1.00, embedding_cos=1.00` surfaces as a fused cluster that the pre-P5 pipeline never saw.
- [x] `make ci-ollama` target — pulls `nomic-embed-text`, runs the `ollama_`-prefixed tests (`cargo test ollama_`). `make ci` filters them out via `--skip ollama_` so the default pipeline needs no external service.
- [x] Coverage ratchet: `embedding/ollama.rs` is excluded from measurement (it's an HTTP client exercised only by `make ci-ollama`); every other P5 file is covered ≥ 93% via the stub-provider E2E tests.

### P6 Harden — COMPLETE
Implements [PIPELINE-INCREMENTAL]. Hardening pass: opt-in on-disk fingerprint cache keyed by `(language, tool_version, min_nodes, content_hash)`, coverage ratchet, fixture-per-bug workflow seeded with a first example, and a scale-smoke test guarding the <30 s / 100 K LOC perf target against order-of-magnitude regressions.

- [x] `FingerprintCache` in `deslop-core::fpcache` — lazy-open, per-language subdirectory, little-endian blob (`u32` magic + recursive `NormalizedNode` tree + `Fingerprint` records). No serde dep.
- [x] `PipelineConfig.incremental: bool` (default `false`). When true, the parse+fingerprint stage consults the cache; hits rehydrate the tree and fingerprints directly, misses parse and persist.
- [x] `--incremental` CLI opt-in. Off by default so read-only checkouts stay pristine (fixtures, CI checkouts, `git worktree` analyses, etc.).
- [x] `Report.cache_stats: { hits: usize, misses: usize }` added at `report_schema_version = 2` (additive, `#[serde(default)]` on deserialise for back-compat with existing P5 reports). Text renderer prints `cache: N hit / M miss` when non-zero.
- [x] Graceful degradation: corrupt blobs log a `warn!` and are treated as misses; cache directory open failures fall back to uncached parse for that language; blob write failures don't fail the run.
- [x] `<30 s on 100 K-LOC C#` perf target encoded as `[PERF-BUDGET-TYPE12]`; e2e suite carries `synthetic_corpus_scale_smoke_test` as a regression guard (wallclock bound deliberately lax — llvm-cov instrumentation makes strict timing brittle; real SLA validated manually against a release build).
- [x] Fixture-per-bug workflow seeded at `tests/fixtures/bug-empty-class/` with a README documenting the naming rule (`bug-<kebab-case-summary>/`) and expected pairing (failing-then-passing test in `cli.rs`). First fixture pins the "empty class body doesn't panic" behaviour.
- [x] E2E suite: `incremental_cache_hits_on_second_run`, `default_run_skips_the_cache`, `corrupt_cache_entry_degrades_to_miss`, `cache_write_failure_is_degraded_not_fatal` (chmod-based), `help_text_documents_incremental_flag`, `synthetic_corpus_scale_smoke_test`, `bug_fixture_walks_trivial_class_body_without_panicking`.
- [x] Coverage held at 94 % threshold (ratchet rule: upward only). P6 additions covered ≥ 94 % via the stub-provider-independent e2e tests above.
- [x] `docs/specs/pipeline.md` — added `[PIPELINE-INCREMENTAL]` section; `[OUTPUT-SCHEMA-JSON]` extended with the `cache_stats` field.

### P6.1 Human-readable HTML mode
Implements [OUTPUT-HUMAN-HTML]. HTML output gains collapsible per-occurrence `<details>` panels with syntax-highlighted snippets and line numbers. JSON schema unchanged. `--human={auto,on,off}` selects mode.

### P6.2 Repo-wide duplication metric + fail-over threshold — COMPLETE
Implements [METRICS-REPO] and [EXIT-CODES]. One honest number + one CI gate, derived deterministically from the cluster set the report already carries.

- [x] `RepoMetrics { analysed_loc, duplicated_loc, duplication_percent, clusters_total, duplicated_files, threshold }` in `deslop-core::report_metrics`. Computed by projecting every non-hidden `ReportOccurrence` onto a per-file `BTreeSet<line>`, unioning, and summing set sizes — overlapping sibling-extension ranges count once.
- [x] `analysed_loc` counted at file-read time (`\n`-terminated lines plus trailing partial line); accumulated onto `FingerprintCorpus.analysed_lines` so the metric adds no extra I/O pass.
- [x] `Report.metrics` wired into the JSON schema; `report_schema_version` bumped 2 → 3 with `#[serde(default)]` on deserialise so P5/P6 reports keep round-tripping through `--from-report`.
- [x] Text renderer header line: `repo: 12.4% duplicated (1 843 / 14 876 LOC, 27 clusters across 11 files)`. HTML renderer surfaces the same line as a banner, colour-coded by threshold state via `metrics-banner--{ok,breached,neutral}` CSS classes.
- [x] `--fail-over <percent>` CLI flag (finite float in `[0.0, 100.0]`, validated at clap-time); `--no-fail-over` override; mutually exclusive; invalid values → exit `2` via clap.
- [x] `[threshold] max_duplication_percent` in `.deslop.toml`, parsed in `deslop-core::config::ExclusionConfig`. CLI flag beats config; `--no-fail-over` beats both.
- [x] Exit `3` when `metrics.duplication_percent > threshold`. Report is still written to disk in full before the non-zero exit so CI can attach it.
- [x] `Report.metrics.threshold { percent, breached, source }` populated from the resolved threshold (`"cli"` / `"config"` / `"none"`) so renderers don't re-derive the verdict.
- [x] `REPORTING-CONTEXT.md` already describes `metrics` + fail-over semantics; embedded via the existing `include_str!` so there is no drift.
- [x] Coverage held at the P6 ratchet (threshold 95, measured 96.3 %). The `_coverage_check` 1 % rounding slack means the threshold can only move when measured coverage climbs by at least 1 full point, so P6.2 keeps the bar steady rather than regress it.
- [x] E2E (13 tests in `crates/deslop/tests/cli.rs`): `metrics_zero_on_empty_corpus`, `metrics_match_hand_counted_fixture`, `metrics_exclude_hidden_occurrences`, `metrics_deduplicate_overlapping_sibling_ranges`, `fail_over_cli_exits_three_on_breach`, `fail_over_cli_passes_under_threshold`, `fail_over_config_file_loaded_when_flag_absent`, `fail_over_cli_overrides_config_file`, `no_fail_over_overrides_config_file_threshold`, `fail_over_invalid_value_exits_two`, `from_report_replays_metrics_without_reanalysing`, `text_renderer_shows_repo_duplication_header`, `html_renderer_colour_codes_threshold_state`.

### P7 Live-analysis foundation
Implements [live.md](../specs/live.md). In-memory, watcher-driven session on top of which the LSP server (P8) and the MCP server (P9) both sit. There is **no daemon process** — the session lives inside whichever binary spawned it. The `live` module ships inside `deslop-core` behind a `live` cargo feature so the CLI stays zero-watcher / zero-`notify` (one crate, feature flag instead of a separate crate — see [principles.md §[PRINCIPLES-LONG-RUNNING-DAEMON]](../specs/principles.md) and [live.md §[LIVE-PACKAGING]](../specs/live.md)).

**Core primitives (landed):**

- [x] `ReportDelta` at `deslop-core::delta` (stable cluster-id projection over two `Report` snapshots; `between(prev, to_gen, next)` + `is_empty()`). Pure, no feature gate — any consumer can diff two reports.
- [x] `PipelineSession` at `deslop-core::pipeline::session`: holds the per-`FileId` normalised trees, fingerprints, source bytes, file registry, exclusion config. `initialise(root, min_nodes, incremental, config_path, embedding)` runs the first full pass. `update_files(changed: &[PathBuf], embedding)` re-parses the listed paths (treating missing-on-disk as deletions), re-runs clustering + ranking over the updated corpus, returns the new `Report`. Reuses the P6 fingerprint cache + P5 embedding cache transparently.
- [x] `pipeline/{config,corpus,signatures,embedding_pass,run,session}.rs` split so `run()` is a thin wrapper over `PipelineSession::initialise` and neither file exceeds the 500-line budget.
- [x] `list_ollama_models(endpoint) -> Vec<OllamaModelInfo>` at `deslop-core::embedding::ollama`: enumerates `/api/tags` + classifies each model via one embedding probe; exported at the crate root as `list_ollama_models` + `OllamaModelInfo`.

**Live module landed under P7:**

- [x] `live` cargo feature on `deslop-core` that pulls in `notify` + `tokio` + `futures` + `async-trait` as workspace-pinned optional dependencies. CLI build path stays feature-off; `deslop-lsp` opts in. Whole module guarded by `#[cfg(feature = "live")]`.
- [x] `deslop-core::live::AnalysisSession` at `live/session.rs` owns one `PipelineSession`, an `Arc<Report>` snapshot, monotonic `generation: u64`, and active embedding provider ([LIVE-STATE]).
- [x] `deslop-core::live::LiveApi` trait + `LiveService` impl at `live/api.rs` exposing the nine query methods ([LIVE-QUERY-API]).
- [x] `duplicates/findSimilar` handles open-range and snippet variants with explicit error types: `LiveError::UnparseableInput`, `LiveError::UnsupportedLanguage`, `LiveError::PathOutsideWorkspace`, and the `below_min_nodes: true` sentinel on tiny snippets.
- [x] `embedding/listModels` prepends the built-in stub + surfaces Ollama models from `list_ollama_models`; falls back to stub-only with `tracing::info!` when Ollama is unreachable.
- [x] `embedding/setModel` atomically swaps the provider and re-runs the pipeline over the live path set — tradeoff comment in `session.rs` documents why we re-parse rather than add a new `PipelineSession` hook.
- [x] `notify`-backed file watcher at `live/watcher.rs` with extension + exclusion filtering before enqueueing; deterministic `Clock`-injected `Debouncer` at `live/debouncer.rs` (quiet 250 ms, cap 2 s).
- [x] Single-flight coalescing scheduler at `live/scheduler.rs` — one `tokio` task, one `select!` loop, 50 ms tick for cap checks; broadcasts `ReportChangedNotification` + `AnalysisState` via `tokio::sync::broadcast`.
- [x] Push notifications at `live/notifications.rs`: `report/changed` (`ChangeSummary::from_delta`) and `analysis/state` (`Idle` / `Running` / `Errored`). Fire-and-forget; slow subscribers are dropped by the channel.
- [x] E2E harness at `crates/deslop-core/tests/live.rs` (feature-gated) — 8 tests: first-report-matches-batch, update_files delta, find_similar open-range / snippet / unparseable / below-min-nodes, debouncer cap, stub-only fallback, and a `LiveService` round-trip covering every `LiveApi` method.
- [x] Coverage maintained at ≥ 95 % (measured 96.1 %). Watcher + scheduler + notifications excluded from coverage measurement in the `Makefile`'s `--ignore-filename-regex` (FFI glue + async tick loops are not test-friendly in a black-box E2E without real-wallclock dependencies).

### P8 LSP server
Implements [lsp.md](../specs/lsp.md). `tower-lsp`-based binary forwarding to P7's `LiveApi`.

- [x] New crate `crates/deslop-lsp` depends on `deslop-core` with `features = ["live"]` + `tower-lsp`. `src/main.rs` stays < 70 LOC of glue; every protocol concern lives in `backend.rs` / `custom_methods.rs` / `diagnostics.rs`.
- [x] `initialize` handshake returns capabilities per [LSP-CAPABILITIES] — `textDocumentSync = Incremental`, server info block, **`diagnosticProvider` (pull-based, LSP 3.17) with `inter_file_dependencies = true`** so editing one file refreshes everyone else's percentile-bucketed severity. The `diagnostic` handler is implemented at [backend.rs](../../crates/deslop-lsp/src/backend.rs).
- [x] Diagnostics at `src/diagnostics.rs` map per-cluster weight → `DiagnosticSeverity` buckets per [LSP-SEVERITY] (`Warning` / `Information` / `Hint` / drop). **Percentile is computed against the whole report** (not just the current file) so a cluster that's the worst in a sleepy file but mid-tier overall ranks mid-tier in the Problems panel — agreeing with the top-offenders tree, the CLI text report, and the HTML report. Global weights flow from `LiveApi::all_cluster_weights()`. `code` carries the stable cluster id; `source = "deslop"`; `message = cluster.interpretation`; `relatedInformation` lists every other occurrence with an "occurrence N of M" label so the Problems panel jumps cross-file.
- [x] Custom `deslop/*` methods (`reportGet`, `reportForFile`, `reportForRange`, `clusterById`, `duplicatesFindSimilar`, `embeddingListModels`, `embeddingSetModel`, `sessionConfig`) at `src/custom_methods.rs` forward 1:1 to `LiveApi`.
- [x] `textDocument/didChange` triggers an incremental pipeline pass through `LiveService::session().lock().apply_changes(...)`.
- [x] E2E at `crates/deslop-lsp/tests/cli.rs` — 5 tests spawn the real binary, drive raw JSON-RPC frames (Content-Length / \r\n\r\n), and assert: `initialize` returns capabilities; `deslop/sessionConfig` returns the workspace root; `deslop/reportGet` returns non-empty clusters on the C# fixture; `deslop/duplicatesFindSimilar` flags `below_min_nodes: true` on a tiny snippet under the configured floor; `deslop/embeddingListModels` falls back to stub when Ollama is unreachable. Tests copy the fixture into a tempdir so `.deslop-cache/` writes never touch the committed fixtures.
- [ ] Deferred (post-P8, tracked for later): code lens ([LSP-CODE-LENS]), hover ([LSP-HOVER]), `definitionProvider` overload, `deslop://` virtual document scheme ([LSP-VIRTUAL-DOC]), `workspace/executeCommand` verbs ([LSP-COMMANDS]) — the LSP shell currently ships the request-response surface; these UX surfaces land with the VSIX work in P10.

### P9 MCP server — COMPLETE (p9-mcp)
Implements [mcp.md](../specs/mcp.md). JSON-RPC-over-stdio MCP server for AI agents (Claude Code, Claude Desktop, Cursor, Continue).

- [x] New crate `crates/deslop-mcp` — `McpBackend` trait + `PipelineSessionBackend` concrete impl forwards to `deslop_core::PipelineSession` (swapping to `LiveApi` once the live-feature trait stabilises is a one-line swap inside `backend.rs`). Six files: `lib.rs` / `protocol.rs` / `safety.rs` / `backend.rs` / `tools.rs` / `resources.rs` / `server.rs` / `main.rs`, each < 500 LOC.
- [x] `initialize` handshake returns `protocolVersion: "2024-11-05"`, declares `tools` + `resources` capabilities (`resources.subscribe=true`), and carries `serverInfo.{name,version}`.
- [x] `tools/list` declares the eight [MCP-TOOLS] with JSON schemas + [MCP-AGENT-PROMPT-GUIDANCE] descriptions authored for an LLM planner: `report-get`, `report-for-file`, `report-for-range`, `find-similar`, `cluster-by-id`, `list-embedding-models`, `set-embedding-model`, `session-config`.
- [x] Tool implementations each forward through the `McpBackend` trait; `tools/call` responses use the canonical MCP envelope (`{ content: [{type:"text",text}], isError:false, structuredContent }`).
- [x] `find-similar` accepts either `{ path, start_byte, end_byte }` (range lookup against the live corpus) or `{ snippet, language }` (in-memory parse via the registered `LanguageParser`, cache-preserving). Explicit `UnparseableInput` (`-32001`), `UnsupportedLanguage` (`-32002`), and `below_min_nodes: true` paths ([MCP-TOOL-FINDSIMILAR]).
- [x] Resources: `deslop://report` (canonical JSON, pretty-printed) and `deslop://schema` (markdown) via `resources/list` + `resources/read` ([MCP-RESOURCES]).
- [x] `notifications/deslop/filesChanged` incoming notification calls `PipelineSession::update_files` and bumps the generation counter so agents can push watcher edits without a tool call round-trip. Outgoing notifications follow once the watcher layer (P7) lands the push pipeline.
- [x] Safety: read-only tools (only `set-embedding-model` mutates session state); workspace-root pinned at `initialize`; `resolve_within_root` canonicalises every path argument and rejects anything outside the root with error `-32003` ([MCP-SAFETY]).
- [x] E2E: `crates/deslop-mcp/tests/cli.rs` drives the real binary over stdio with raw JSON-RPC frames — 50 tests covering the initialize handshake, `tools/list` shape, every tool (happy + error), resources list + read, path-traversal rejection, malformed-frame parse error, unknown-method error, invalid jsonrpc version, notifications/initialized + files-changed, string-id round-trip, empty-line tolerance, Ollama-auto fallback to stub, and binary CLI arg validation.
- [x] `make ci` green: fmt check + clippy pedantic+nursery + 50 MCP e2e + existing 72 CLI + 7 rerun + 7 live + 5 LSP tests. Coverage 96.1% (above 95% threshold + 1% rounding slack). `deslop-mcp/src/main.rs` + `server.rs` excluded from coverage (transport shells exercised only via subprocess E2E; llvm-cov can't see instrumented child output). Ollama-backed variants sit behind the `ollama_` prefix and run under `make ci-ollama`.

### P10 VSIX + live bubble — v0.1 LANDED (deslop-opus-main)
Implements [vsix.md](../specs/vsix.md). The in-your-face "you're duplicating code right now" UX. This is the feature that defines the product.

- [x] Repo layout: new `clients/vscode/` workspace, TypeScript. `src/extension.ts` < 500 LOC; UI split across `webview/`, `tree/`, `decorations/`, `commands/`, `bubble/`, `types/`.
- [x] Activation on `onLanguage:{csharp,rust,python}` + `workspaceContains:**/*.{cs,rs,py}` + `onCommand:deslop.openReport` (`package.json`).
- [x] Settings under `deslop.*`: `minNodes`, `embedding.{provider,model,endpoint,mode}`, `incremental`, `showAllLenses`, `configPath`, `liveBubble.{enabled,mode}` ([VSIX-SETTINGS]).
- [x] `contributes.mcpServers` manifest entry registering the bundled `deslop-mcp` binary ([VSIX-MCP-INTEGRATION]).
- [x] Design tokens module (`src/design.ts`): Kinetic Manuscript palette, Inter + JetBrains Mono, no-line / no-soft-radius rules, severity ramp with crimson as surgical accent.
- [x] TS mirror of Report schema v3 (`src/types/report.ts`): `Report`, `ReportCluster`, `ReportSignals`, `RepoMetrics`, `ThresholdSummary`, `ReportDelta`, `EmbeddingModelInfo`, severity bucketer.
- [x] Activity bar "Duplicate Clusters" view container: Top Offenders tree (worst-first, severity-badged), Focused File tree, Session panel ([VSIX-ACTIVITY-BAR]).
- [x] Editor decorations: overview-ruler severity bar + 1-pixel underline per occurrence ([VSIX-DECORATIONS]).
- [x] **[VSIX-LIVE-BUBBLE] flagship.** Inline `TextEditorDecorationType` (severity dot + verdict + count + canonical) + `InlayHint` (3-bar signal strip) + ghost-line mode. Debounce 250 ms, budget 250 ms, cluster-id-stable cooldown, per-session dismiss.
- [x] **[VSIX-EMBED-PICKER] Ollama model picker.** QuickPick with recommended-for-code hints, `stub` entry, "Pull a new model…" + "Refresh list," Ollama-down fallback, stub-selection warning.
- [x] Status bar item: `dedup · N · #1=File.cs:230 · embed=<model>` ([VSIX-STATUS-BAR]).
- [x] Command palette entries ([VSIX-COMMANDS]): openReport, openWorstCluster, jumpToNextOccurrence, compareWithCanonical, pickEmbeddingModel, refreshReport, toggleShowAllLenses, showSchemaDoc.
- [x] Binary resolver ([VSIX-BINARY-VERSIONING]) — `src/binary.ts`: `${DESLOP_BINARY_DIR}` → `PATH` (accepted only when `--version` matches the VSIX version exactly) → bundled `${extensionPath}/bin/<platform>/`; on bundled, prepends that directory to the VS Code session `PATH`. `deslop.revealActiveBinary` surfaces the resolved path.
- [x] Centralised state store ([VSIX-STATE]) — single `ReportStore` feeds tree, decorations, bubble, status, picker. No parallel caches.
- [x] Webview reactivity via Preact Signals ([VSIX-WEBVIEW-REACTIVITY]) — `clients/vscode/webview-ui/src/store.ts` exposes `report`, `selectedClusterId`, `analysisState`, `filters`, `severityByClusterId`, `filteredClusters` as signals / computeds. Extension `postMessage`s are the only writer → zero stale UI.
- [x] Cluster detail webview (`deslop.openCluster`) — Preact component at `webview-ui/src/cluster/main.tsx`: header with severity badge + rank + weight + canonical path, 4-bar signal strip, alternating-row occurrence list, `j/k/n/p/Enter/?` hotkeys, signals-driven re-render on every delta.
- [x] Full report webview (`deslop.openReport`) — `webview-ui/src/report/main.tsx`: display-large duplication percent, cache + state footer, filter row (language / severity / path glob), worst-first list, signal-driven refresh on every report delta.
- [x] VSIX packaging pipeline folded into [.github/workflows/release.yml](../../.github/workflows/release.yml) — the `build` matrix (`macos-x64`, `macos-arm64`, `linux-x64`, `windows-x64`) emits `vsix-bin-<platform>` artifacts that the `package-vsix` job stages into `clients/vscode/bin/<platform>/` before `vsce package`. The resulting `.vsix` is attached to the GitHub Release alongside the CLI archives. No Marketplace / OpenVSX auto-publish — install via `code --install-extension` or download from the release page.
- [x] Binary lock-step versioning with the VSIX version ([VSIX-BINARY-VERSIONING]) — enforced by the `version-check` job above; the same job that builds the binaries is the one that packages the VSIX, so there is no path to a mismatched bundle.
- [ ] `schema_doc.md` pulled from `docs/specs/REPORTING-CONTEXT.md` at build time (today the webview reads `report.schema_doc` directly from the live signal store — drift is already impossible, a build-time copy step is a future ergonomic improvement for offline docs).
- [x] Bundle per-platform pre-built `deslop-lsp` + `deslop-mcp` binaries at release time — `build` matrix in [release.yml](../../.github/workflows/release.yml) emits `vsix-bin-<platform>` artifacts that `package-vsix` flattens into `clients/vscode/bin/<platform>/` before `vsce package`. No download-on-activate.
- [x] E2E: VS Code extension test harness in [clients/vscode/test/](../../clients/vscode/test/) — `activation.test.ts`, `bubble.test.ts`, `embeddingPicker.test.ts`, `webviews.test.ts`, `binaryResolver.test.ts`. Fixtures: `test/fixtures/csharp-small/{Alpha,Beta}.cs` (Type-2 clone), `test/fixtures/fake-bin/deslop-lsp` (stub LSP installed by `install-fake-lsp.mjs` that answers `initialize` + `deslop/reportGet` + `deslop/duplicatesFindSimilar` + `deslop/embeddingListModels`). Harness: `test/run-tests.mjs` drives `@vscode/test-electron`.
- [x] README at [clients/vscode/README.md](../../clients/vscode/README.md) — headline, feature list, design-system callout, install matrix (GitHub Release `.vsix` / brew / scoop). Demo GIF pending the first recorded session.

### P11 Canonical clone buckets + dual labelling
Implements [CLONE-BUCKETS] and [CLONE-BUCKETS-DUAL-LABEL] from [taxonomy.md](../specs/taxonomy.md). Today the codebase ships four parallel vocabularies for the same buckets (HTML titles, CLI plain summary, CLI technical summary, `interpret()` strings) and the semantic / Type-4 cluster silently collapses into the weak bucket on every UI surface. P11 makes the taxonomy table the single source of truth, promotes the AI-detected `SameBehavior` bucket to a first-class citizen, and drops academic `Type-N` labels from human-facing copy (JSON / `schema_doc` / MCP keep them per [CLONE-BUCKETS-DUAL-LABEL]).

Phased rollout — each phase is independently reviewable and `make ci`-green.

**P11.1 — Core plumbing (no visible change yet).**
- [ ] Extend `ClusterKind` at `crates/deslop-core/src/render/html.rs` with four canonical variants: `Identical`, `NearlyIdentical`, `LooselySimilar`, `SameBehavior`. Delete `Exact` / `Near` / `Weak`. Ripgrep the crate for stragglers.
- [ ] Move `ClusterKind` + `cluster_kind()` out of `render/html.rs` into a new `deslop-core::buckets` module so every renderer depends on one source. Keep the file < 500 LOC.
- [ ] Add `fn bucket_labels(kind: ClusterKind) -> BucketLabels` returning `{ human_title, action_sentence, css_suffix, taxonomy_label }`. This is the single helper every surface pulls from per [CLONE-BUCKETS-DUAL-LABEL] rule 5.
- [ ] Update `cluster_kind()` signal-routing to match [CLONE-BUCKETS-ROUTING] exactly — `SameBehavior` tested before `NearlyIdentical`, threshold constants pulled from the same shared module that already owns `FUSED_THRESHOLD`. E2E: `cluster_kind_matches_canonical_routing_table` runs every row of the routing table as a parameterised assertion.
- [ ] `crates/deslop-core/src/report.rs::interpret()` rewritten to call `bucket_labels()` instead of hard-coding strings. Output must continue to include the Type-N reference (e.g. `"Identical code (Type-1/2 exact clone). Safe to extract — every copy is the same."`) since `interpret()` feeds the JSON `cluster.interpretation` field and that is agent-facing per [CLONE-BUCKETS-DUAL-LABEL] rule 3.
- [ ] `default_action_hints()` rewritten to match — four entries, one per bucket, each carrying both the human label and the Type-N reference.

**P11.2 — CLI stderr summary.**
- [ ] `crates/deslop/src/summary/body.rs::write_breakdown_plain` refactored to iterate over `ClusterKind::all()` and call `bucket_labels()`. No hard-coded strings.
- [ ] Add a fourth column for `SameBehavior` with the `(AI match)` suffix: `"{n} same behavior, different code (AI match)"`. Cyan. Omitted when count is zero (same rule as today's semantic tail).
- [ ] **Delete** `write_breakdown_technical` and the `--technical` flag wired through [main.rs:124](../../crates/deslop/src/main.rs#L124). Dual labelling lives in JSON / schema_doc, not in a CLI mode; a second mode is drift-by-design. Tests: delete `writes_technical_breakdown_with_type_labels` and update help-text assertion.
- [ ] Rename `ClusterBreakdown::semantic` → `same_behavior` and `ClusterBreakdown::near_miss` → `nearly_identical` for grep-consistency with the enum variants. Rename `exact` → `identical` and `weak` → `loosely_similar`. This is the final name, everywhere.

**P11.3 — HTML renderer.**
- [ ] `cluster_kind()` gains the `SameBehavior` arm (was silently routed to `Weak`); card title + action sentence come from `bucket_labels()`.
- [ ] Add a `.cluster-card--ai` badge ("AI match") in `render/html.rs` that renders only when `kind == SameBehavior`. CSS: purple / cyan pair per [CLONE-BUCKETS].
- [ ] Add a `kind-samebehavior` CSS class with the purple / cyan light / dark band.
- [ ] E2E: `html_renderer_shows_ai_match_badge_on_same_behavior_cluster` + `html_renderer_uses_canonical_human_titles` (asserts no `Type-1`, `Type-2`, `Type-3`, `Type-4`, `exact clone`, `near-miss` substrings in the HTML body).

**P11.4 — JSON / `schema_doc` / MCP (Type-N preserved).**
- [ ] `REPORTING-CONTEXT.md` already carries the dual-labelled table after the P11 doc pass; re-verify `include_str!` picks it up and `schema_doc_round_trips_through_from_report` still passes.
- [ ] Add a structured `cluster.bucket: String` field on `ReportCluster` (`"identical" | "nearly_identical" | "loosely_similar" | "same_behavior"`). Makes every consumer stop re-deriving the bucket from the signal triple. Bump `report_schema_version` 3 → 4 with `#[serde(default)]` for back-compat; `--from-report` must still read v3 reports.
- [ ] `crates/deslop-mcp/src/tools.rs` tool descriptions + `cluster-by-id` response include both labels (already do via embedded `schema_doc`; no work unless grep finds a bespoke string).
- [ ] E2E: `report_carries_canonical_bucket_field_on_every_cluster`, `from_report_v3_upgrades_bucket_field_deterministically`.

**P11.5 — VS Code extension.**
- [ ] `clients/vscode/src/types/report.ts` gains the TS mirror of the `bucket` field + a `BucketLabels` helper mirroring the Rust one. Single source for the webview store, tree view, bubble, and status bar.
- [ ] Tree view, bubble, cluster detail webview, full report webview all switch from the legacy `Exact` / `Near` / `Weak` names to canonical human labels via `BucketLabels`. Add the AI-match badge on `SameBehavior` rows.
- [ ] `clients/vscode/src/commands/embeddingPicker.ts` references to `Type-3` etc. reviewed — picker explains which buckets depend on the embedding pass in plain English.
- [ ] E2E: extend `clients/vscode/test/webviews.test.ts` to assert the AI-match badge renders on the `SameBehavior` fixture row and no Type-N strings appear in the rendered DOM.

**P11.6 — Static docs under `site/`.**
- [ ] Audit `site/src/docs/output-formats.md`, `site/src/docs/how-it-works.md`, `site/src/docs/ai-integration.md`, `site/src/blog/ranking-formula.md` — replace Type-N-only phrasing with bucket-first / Type-N-in-parens per [CLONE-BUCKETS-DUAL-LABEL]. Keep academic refs where the audience is plausibly researcher (reading list, competitor landscape).
- [ ] Update screenshots / code samples so the published site matches the shipped UI.

**P11.7 — Ratchet + close-out.**
- [ ] Ripgrep the repo for `Type-1`, `Type-2`, `Type-3`, `Type-4`, `near-miss`, `exact clone`, `semantic clone`, `LSH-only` in human-facing strings (`*.rs` string literals, `*.ts`, `*.tsx`, `clients/vscode/package.json` contributed strings, HTML templates). Every human-facing hit either moves to `bucket_labels()` or gets a `// TODO [CLONE-BUCKETS]` comment rejected in review.
- [ ] Coverage ratchet: hold at the current threshold or raise by 1 point if the new `bucket_labels()` helper + routing tests lift measured coverage.
- [ ] Update example README snippets (`examples/README.md` etc.) where the taxonomy wording leaks.

**Non-goals for P11.** No signal-threshold tuning. No new detectors. No changes to cluster ranking / weighting. No CLI-flag churn beyond deleting `--technical`. If a bucket boundary feels wrong on real repos, that's a separate phase against [CLONE-BUCKETS-ROUTING].
