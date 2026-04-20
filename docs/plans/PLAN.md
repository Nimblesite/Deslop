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
- **P6 Harden** — `--incremental` on-disk fingerprint cache keyed by `(language, tool_version, min_nodes, content_hash)`, perf regression guard, fixture-per-bug workflow, coverage ratchet.
- **P6.2 Repo-wide duplication metric + fail-over** — `Report.metrics { analysed_loc, duplicated_loc, duplication_percent, clusters_total, duplicated_files, threshold }` computed deterministically from non-hidden occurrences ([METRICS-REPO]). `--fail-over <percent>` CLI flag + `[threshold] max_duplication_percent` in `.codededup.toml` gate CI with exit `3` on breach ([EXIT-CODES]). `report_schema_version` → 3.
- **P7 Live-analysis foundation** — `live` module inside `codededup-core` (behind a `live` cargo feature): `AnalysisSession`, file watcher, debouncer, scheduler, `LiveApi` query surface, `ReportDelta` push notifications. `ReportDelta` + `PipelineSession` + `update_files` are already landed. See [live.md](../specs/live.md).
- **P8 LSP server** — `codededup-lsp` bin over `tower-lsp`: diagnostics, code lens, hover, virtual docs, `codededup/*` custom methods. Editor-agnostic thin forwarder to `LiveApi`. See [lsp.md](../specs/lsp.md).
- **P9 MCP server** — `codededup-mcp` bin: tools (`find-similar`, `report-for-*`, `set-embedding-model`, …), resources, notifications. Agent-facing thin forwarder to `LiveApi`. See [mcp.md](../specs/mcp.md).
- **P10 VSIX + live bubble** — VS Code extension with live duplication bubble ([VSIX-LIVE-BUBBLE]), tree view, webview, status bar, Ollama embedding-model picker ([VSIX-EMBED-PICKER]). The in-your-face "you're duplicating right now" moment. See [vsix.md](../specs/vsix.md).

## Non-goals (across all phases)
No remote APIs. No execution validation (HyClone). No cross-language detection. No auto-fix / extract-to-function — refactoring belongs downstream of a dedicated engine. No unit tests; coarse E2E only.

## Future work (deliberately deferred)
- **Interactive / TUI mode (`--interactive`).** Paginated top-clusters view with inline byte-range previews, keyboard navigation between occurrences, and "extract refactor suggestion" shortcuts. Mostly a `ratatui` build-out; the deterministic core already emits everything needed. Only worth shipping after we've seen real operator use of the colored stderr summary — that tells us what the interactive view should emphasise.

## Risks
`--min-nodes` default is a guess — tune on real repos. Sibling-extension may miss Type-3s that need tree-edit-distance. HNSW determinism is per-machine only. Ollama absence: CI runs P3 path by default, nightly runs P5.

---

## Current state (summary)

- **P0 – P6 complete.** C#, Rust, and Python Type-1 / Type-2 / Type-3 / Type-4 clone detection works end-to-end through the CLI. Reports are self-documenting (embedded `schema_doc`, per-cluster `interpretation`, top-level `action_hints`) and emitted in three formats by default (canonical JSON, terse AI-readable text, single-file HTML). Text and HTML are derived from JSON; `--from-report` re-renders a cached JSON without re-analysing. Exclusion config (`.codededup.toml`) supports two tiers: `exclude` (skip parsing) and `report_hide` (analyse but hide hidden-only clusters), with per-language overlays. Embedding pass (`--embeddings={auto,required,off}`) fuses cosine similarity from a pluggable `EmbeddingProvider` (ships with `ollama` + `stub` providers) via HNSW top-k into the candidate-pair union and the fused score. Provenance `(provider_id, model_id, model_version, dimensions)` is pinned in every report; embeddings are cached by `(content_hash, provider, model, version)` under `.codededup-cache/`. Incremental-analysis cache (`--incremental`) keyed by `(language, tool_version, min_nodes, content_hash)` rehydrates unchanged files without tree-sitter ([PIPELINE-INCREMENTAL]); hits/misses surface as `cache_stats` in every report.
- `make ci` green: 42/42 e2e tests (Ollama-gated test runs under `make ci-ollama` via the `ollama_` name prefix), clippy clean (pedantic + nursery), rustfmt clean, coverage **94.3% ≥ 94% threshold** (held steady through P6: new fpcache module covered end-to-end via e2e tests exercising the happy path, corrupt-blob recovery, read-only directory degradation, and default-off semantics).
- Verified non-destructively against a real 63-file C# repo (TradiSite backend): 17K fingerprints, 2040 clusters ranked worst-first, no panics, no source modification. Top offenders correctly surface generated GraphQL `.g.cs` duplication and test-fixture boilerplate.
- GitHub repo settings applied (squash-only, auto-merge, delete-on-merge, wiki/projects off, discussions on, ruleset "Protect main" requires PR + CI check).
- `float_arithmetic = "deny"` removed from the lint profile (with rationale comment in `Cargo.toml`); AgentPMO template updated with the same rationale so other repos don't inherit the footgun.
- Spec IDs converted to hierarchical `[GROUP-TOPIC-DETAIL]` form; every module references the IDs it implements AND the academic work those IDs cite (Baxter 1998, Chilowicz 2009, SourcererCC, ensemble-LLM 2025).
- Pluggable by construction: `PairScore` carries a third `embedding_cos` slot so P5 is additive; fingerprints are keyed by `(file_id, byte_range)` so P6 file-watcher incremental updates slot into the same cache keys.

**Next up (beyond P6):** phases **P7–P10** take the deterministic core live without adding a daemon process. P7 adds the `live` module inside `codededup-core` (behind a `live` cargo feature) with the file watcher, re-analysis scheduler, `LiveApi` query surface, and `ReportDelta` push notifications. The core primitives are already landed: `ReportDelta` at `codededup-core::delta`, `PipelineSession::update_files` at `codededup-core::pipeline::session`, and `list_ollama_models` for the VSIX model picker. P8 and P9 are thin forwarders on top: an LSP binary ([lsp.md](../specs/lsp.md)) and an MCP binary ([mcp.md](../specs/mcp.md)). P10 ships the VS Code extension ([vsix.md](../specs/vsix.md)) — the reference client — whose flagship UX is the **live duplication bubble** ([VSIX-LIVE-BUBBLE]): the moment a developer types code that matches an existing cluster, the editor tells them inline, before save. No competitor does this ([competitors.md](../specs/competitors.md)); it is the category-defining feature. TUI mode remains deferred.

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
- [x] `--debug-ast <FILE>` CLI flag — parses one source file and prints the deterministic normalised AST dump to stdout. Implemented in `codededup-core::pipeline::debug_ast_dump` and rendered by `codededup-core::render::ast::render_ast_dump`. Conflicts with `--from-report`; writes nothing to disk; mutates no caches.
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

### P6 Harden — COMPLETE
Implements [PIPELINE-INCREMENTAL]. Hardening pass: opt-in on-disk fingerprint cache keyed by `(language, tool_version, min_nodes, content_hash)`, coverage ratchet, fixture-per-bug workflow seeded with a first example, and a scale-smoke test guarding the <30 s / 100 K LOC perf target against order-of-magnitude regressions.

- [x] `FingerprintCache` in `codededup-core::fpcache` — lazy-open, per-language subdirectory, little-endian blob (`u32` magic + recursive `NormalizedNode` tree + `Fingerprint` records). No serde dep.
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

### P6.2 Repo-wide duplication metric + fail-over threshold
Implements [METRICS-REPO] and [EXIT-CODES]. One honest number + one CI gate, derived deterministically from the cluster set the report already carries.

- [ ] `RepoMetrics { analysed_loc, duplicated_loc, duplication_percent, clusters_total, duplicated_files, threshold }` in `codededup-core::report::metrics`. Computed by projecting every non-hidden `ReportOccurrence` onto a per-file `BTreeSet<line>`, unioning, and summing set sizes — overlapping sibling-extension ranges count once.
- [ ] `analysed_loc` counted at file-read time (`\n`-terminated lines plus trailing partial line); accumulated onto the existing corpus struct so the metric adds no extra I/O pass.
- [ ] `Report.metrics` wired into the JSON schema; `report_schema_version` bumped 2 → 3 with `#[serde(default)]` on deserialise so P5/P6 reports keep round-tripping through `--from-report`.
- [ ] Text renderer header line: `repo: 12.4% duplicated (1 843 / 14 876 LOC, 27 clusters across 11 files)`. HTML renderer surfaces the same line, colour-coded by threshold state.
- [ ] `--fail-over <percent>` CLI flag (finite float in `[0.0, 100.0]`); `--no-fail-over` override; mutually exclusive; invalid values → exit `2`.
- [ ] `[threshold] max_duplication_percent` in `.codededup.toml`, parsed in `codededup-core::config`. CLI flag beats config; `--no-fail-over` beats both.
- [ ] Exit `3` when `metrics.duplication_percent > threshold`. Report is still written to disk in full before the non-zero exit so CI can attach it.
- [ ] `Report.metrics.threshold { percent, breached, source }` populated from the resolved threshold (`"cli"` / `"config"` / `"none"`) so renderers don't re-derive the verdict.
- [ ] `schema_doc` updated to describe `metrics` + fail-over semantics (still via `include_str!` from `REPORTING-CONTEXT.md`, no drift).
- [ ] Coverage ratchet (target: ≥ 94 % → step upward once the renderer + config paths land).
- [ ] E2E: `metrics_zero_on_empty_corpus`, `metrics_match_hand_counted_fixture`, `metrics_exclude_hidden_occurrences`, `metrics_deduplicate_overlapping_sibling_ranges`, `fail_over_cli_exits_three_on_breach`, `fail_over_cli_passes_under_threshold`, `fail_over_config_file_loaded_when_flag_absent`, `fail_over_cli_overrides_config_file`, `no_fail_over_overrides_config_file_threshold`, `fail_over_invalid_value_exits_two`, `from_report_replays_metrics_without_reanalysing`, `text_renderer_shows_repo_duplication_header`, `html_renderer_colour_codes_threshold_state`.

### P7 Live-analysis foundation
Implements [live.md](../specs/live.md). In-memory, watcher-driven session on top of which the LSP server (P8) and the MCP server (P9) both sit. There is **no daemon process** — the session lives inside whichever binary spawned it. The `live` module ships inside `codededup-core` behind a `live` cargo feature so the CLI stays zero-watcher / zero-`notify` (one crate, feature flag instead of a separate crate — see [principles.md §[PRINCIPLES-LONG-RUNNING-DAEMON]](../specs/principles.md) and [live.md §[LIVE-PACKAGING]](../specs/live.md)).

**Core primitives (landed):**

- [x] `ReportDelta` at `codededup-core::delta` (stable cluster-id projection over two `Report` snapshots; `between(prev, to_gen, next)` + `is_empty()`). Pure, no feature gate — any consumer can diff two reports.
- [x] `PipelineSession` at `codededup-core::pipeline::session`: holds the per-`FileId` normalised trees, fingerprints, source bytes, file registry, exclusion config. `initialise(root, min_nodes, incremental, config_path, embedding)` runs the first full pass. `update_files(changed: &[PathBuf], embedding)` re-parses the listed paths (treating missing-on-disk as deletions), re-runs clustering + ranking over the updated corpus, returns the new `Report`. Reuses the P6 fingerprint cache + P5 embedding cache transparently.
- [x] `pipeline/{config,corpus,signatures,embedding_pass,run,session}.rs` split so `run()` is a thin wrapper over `PipelineSession::initialise` and neither file exceeds the 500-line budget.
- [x] `list_ollama_models(endpoint) -> Vec<OllamaModelInfo>` at `codededup-core::embedding::ollama`: enumerates `/api/tags` + classifies each model via one embedding probe; exported at the crate root as `list_ollama_models` + `OllamaModelInfo`.

**Still to land under P7:**

- [ ] `live` cargo feature on `codededup-core` that pulls in `notify` as a workspace-pinned optional dependency. CLI build path (`codededup` binary) does **not** enable the feature; LSP/MCP binaries in P8/P9 do. Guarded by `#[cfg(feature = "live")]` on the whole `live` module.
- [ ] `codededup-core::live::AnalysisSession`: owns one `PipelineSession`, an `Arc<Report>` current snapshot, a monotonic `generation: u64`, a subscriber list, and the active embedding provider. No new global state — `AnalysisSession` is the only live struct ([LIVE-STATE]).
- [ ] `codededup-core::live::LiveApi` trait exposing the nine query methods from [LIVE-QUERY-API]: `report/get`, `report/delta`, `report/forFile`, `report/forRange`, `cluster/byId`, `duplicates/findSimilar`, `embedding/listModels`, `embedding/setModel`, `session/config`. Concrete impl `LiveService` wraps an `Arc<Mutex<AnalysisSession>>`.
- [ ] `duplicates/findSimilar` with two input variants: open-buffer range (cache lookup against the live corpus) and `{ snippet, language }` (in-memory parse, no cache mutation). Explicit error types for unparseable / unsupported-language / below-min-nodes inputs.
- [ ] `embedding/listModels` wraps `list_ollama_models` and prepends the built-in `stub` provider. Falls back to stub-only when Ollama is unreachable (downgrades `ProviderError::Unreachable` to `vec![stub_info]` with `tracing::info!`).
- [ ] `embedding/setModel` swaps providers atomically, invalidates only the embedding cache layer ([FUSION-EMBED-PROVIDER]), re-runs the embedding pass on the existing fingerprint set. Returns the new `EmbeddingProvenance`.
- [ ] `notify`-backed file watcher ([LIVE-WATCHER]) with deterministic `Clock`-injected debouncer (250 ms quiet / 2 s cap). Excluded paths filtered before debounce. Debouncer test-injectable so E2E doesn't depend on wall-clock timing.
- [ ] Single-flight scheduler ([LIVE-SCHEDULER]) with queued coalescing: at most one `update_files` in flight; a new changeset during a running pass is queued and merged before dispatch. Budget: < 500 ms end-to-end for ≤ 10 changed files on 100 K LOC ([LIVE-PERF-BUDGETS]) surfaced as `tracing::warn!` on miss.
- [ ] Push notifications ([LIVE-NOTIFICATIONS]): `report/changed` (payload `{ generation, ChangeSummary }`) and `analysis/state` (idle / running / errored). Fire-and-forget — slow subscribers never block the scheduler.
- [ ] E2E harness in `crates/codededup-core/tests/live.rs` (feature-gated) driving `LiveService` directly (no transport yet — those land in P8/P9). Fixtures reuse `crates/codededup/tests/fixtures/csharp-small/`. Assertions: initial report shape matches batch `run()` output; `update_files` after an edit produces a `ReportDelta` with non-empty added/removed/updated; `find-similar` on a known range returns the expected cluster; `find-similar` on unparseable input returns the explicit error; `find-similar` on a below-min-nodes snippet returns `below_min_nodes: true`; `set-embedding-model` swaps provenance without dropping fingerprint-cache hits; `list-embedding-models` falls back to stub when Ollama is unreachable; debouncer coalesces a burst and flushes at the cap.
- [ ] Coverage ratchet once the `live` module lands (target: maintain ≥ 94 %). Watcher + notify glue is covered via the deterministic debouncer tests plus one smoke test with a real file write; the `notify` FFI path itself does not count toward coverage.

### P8 LSP server
Implements [lsp.md](../specs/lsp.md). `tower-lsp`-based binary forwarding to P7's `LiveApi`.

- [ ] New crate `crates/codededup-lsp` (< 100 LOC of glue); depends on `codededup-core` with `features = ["live"]` + `tower-lsp`.
- [ ] `initialize` handshake returns capabilities per [LSP-CAPABILITIES]. Spawns an `AnalysisSession` rooted at the first workspace folder; multi-root = one process per root.
- [ ] Diagnostics (pull-based, LSP 3.17): one per clone occurrence in the active documents; severity mapped from weight percentile per [LSP-SEVERITY]; `relatedInformation` links to other occurrences; `code` = stable cluster id.
- [ ] Code lens at the first line of every occurrence: severity glyph + signal summary + "jump to next" action ([LSP-CODE-LENS]).
- [ ] Hover over a clone range returns markdown with cluster id, interpretation, signal table, occurrence list, matching action hints ([LSP-HOVER]).
- [ ] `definitionProvider` overloaded: inside a clone range, jumps to the canonical occurrence of that cluster.
- [ ] `codededup://` virtual document scheme: `cluster/<id>`, `report`, `schema` ([LSP-VIRTUAL-DOC]).
- [ ] Custom LSP methods in the `codededup/*` namespace forwarding 1:1 to `LiveApi` ([LSP-CUSTOM-METHODS]).
- [ ] `workspace/executeCommand` verbs: `refreshReport`, `openCluster`, `openReport`, `pickEmbeddingModel`, `toggleIncremental` ([LSP-COMMANDS]).
- [ ] E2E: `crates/codededup-lsp/tests/cli.rs` spawns the real binary over stdio, drives the JSON-RPC handshake, asserts diagnostics on a fixture workspace, asserts delta on buffer edit, asserts `codededup/duplicatesFindSimilar` returns the expected cluster.

### P9 MCP server
Implements [mcp.md](../specs/mcp.md). JSON-RPC-over-stdio MCP server for AI agents (Claude Code, Claude Desktop, Cursor, Continue).

- [ ] New crate `crates/codededup-mcp` (< 100 LOC of glue); depends on `codededup-core` with `features = ["live"]`.
- [ ] `initialize` + `tools/list` declares eight tools with schemas + agent-facing descriptions per [MCP-TOOLS] and [MCP-AGENT-PROMPT-GUIDANCE].
- [ ] Tool implementations (each forwards to `LiveApi`): `report-get`, `report-for-file`, `report-for-range`, `find-similar`, `cluster-by-id`, `list-embedding-models`, `set-embedding-model`, `session-config`.
- [ ] `find-similar` accepts either a `{ path, start_byte, end_byte }` open-buffer range or `{ snippet, language }`. Cache-preserving. Returns top-N fused clusters; explicit `UnparseableInputError` / `UnsupportedLanguageError` / `below_min_nodes: true` paths ([MCP-TOOL-FINDSIMILAR]).
- [ ] Resources: `codededup://report`, `codededup://schema` via `resources/list` + `resources/read` ([MCP-RESOURCES]).
- [ ] Notifications: `notifications/resources/updated` on report refresh; `notifications/codededup/reportChanged` custom with `{ generation, summary }`.
- [ ] Safety: all tools are read-only except `set-embedding-model`; workspace-root pinned at `initialize`; no path traversal outside it ([MCP-SAFETY]).
- [ ] E2E: `crates/codededup-mcp/tests/cli.rs` drives raw JSON-RPC frames over a pipe: initialize → tools/list → tools/call for each of the eight tools → resources/read → edit-triggered `notifications/resources/updated`.
- [ ] `make ci` integrates MCP E2E under the standard (non-Ollama-gated) runner; `make ci-ollama` runs the Ollama-backed find-similar path.

### P10 VSIX + live bubble — IN PROGRESS (codededup-opus-main)
Implements [vsix.md](../specs/vsix.md). The in-your-face "you're duplicating code right now" UX. This is the feature that defines the product.

- [x] Repo layout: new `clients/vscode/` workspace, TypeScript. `src/extension.ts` < 500 LOC; UI split across `webview/`, `tree/`, `decorations/`, `commands/`, `bubble/`, `types/`.
- [x] Activation on `onLanguage:{csharp,rust,python}` + `workspaceContains:**/*.{cs,rs,py}` + `onCommand:codededup.openReport` (`package.json`).
- [x] Settings under `codededup.*`: `minNodes`, `embedding.{provider,model,endpoint,mode}`, `incremental`, `showAllLenses`, `configPath`, `liveBubble.{enabled,mode}` ([VSIX-SETTINGS]).
- [x] `contributes.mcpServers` manifest entry registering the bundled `codededup-mcp` binary ([VSIX-MCP-INTEGRATION]).
- [x] Design tokens module (`src/design.ts`): Kinetic Manuscript palette, Inter + JetBrains Mono, no-line / no-soft-radius rules, severity ramp with crimson as surgical accent.
- [x] TS mirror of Report schema v3 (`src/types/report.ts`): `Report`, `ReportCluster`, `ReportSignals`, `RepoMetrics`, `ThresholdSummary`, `ReportDelta`, `EmbeddingModelInfo`, severity bucketer.
- [x] Activity bar "Duplicate Clusters" view container: Top Offenders tree (worst-first, severity-badged), Focused File tree, Session panel ([VSIX-ACTIVITY-BAR]).
- [x] Editor decorations: overview-ruler severity bar + 1-pixel underline per occurrence ([VSIX-DECORATIONS]).
- [x] **[VSIX-LIVE-BUBBLE] flagship.** Inline `TextEditorDecorationType` (severity dot + verdict + count + canonical) + `InlayHint` (3-bar signal strip) + ghost-line mode. Debounce 250 ms, budget 250 ms, cluster-id-stable cooldown, per-session dismiss.
- [x] **[VSIX-EMBED-PICKER] Ollama model picker.** QuickPick with recommended-for-code hints, `stub` entry, "Pull a new model…" + "Refresh list," Ollama-down fallback, stub-selection warning.
- [x] Status bar item: `dedup · N · #1=File.cs:230 · embed=<model>` ([VSIX-STATUS-BAR]).
- [x] Command palette entries ([VSIX-COMMANDS]): openReport, openWorstCluster, jumpToNextOccurrence, compareWithCanonical, pickEmbeddingModel, refreshReport, toggleShowAllLenses, showSchemaDoc.
- [ ] Binary resolver ([VSIX-BINARY-VERSIONING]): `${CODEDEDUP_BINARY_DIR}` → `PATH` (when `--version` matches) → bundled; session-local PATH prepend.
- [ ] Cluster detail webview (`codededup.openCluster`): header, interpretation + action hints, 4-bar signal chart, per-occurrence collapsible panels; keyboard nav (`j/k/n/p/Enter/?`) ([VSIX-WEBVIEW]).
- [ ] Full report webview (`codededup.openReport`): live-refreshing HTML, `report/changed` wiring, filters, fixed worst-first sort ([VSIX-REPORT-WEBVIEW]).
- [ ] Bundle per-platform pre-built `codededup-lsp` + `codededup-mcp` binaries (darwin-arm64, darwin-x64, linux-x64, linux-arm64, win32-x64). **No download-on-activate.**
- [ ] Binary lock-step versioning with the VSIX version ([VSIX-BINARY-VERSIONING]).
- [ ] `schema_doc.md` pulled from `docs/specs/REPORTING-CONTEXT.md` at build time; no drift.
- [ ] Marketplace + OpenVSX publishing pipeline in `.github/workflows/publish-vsix.yml`.
- [ ] E2E: VS Code extension test harness in `clients/vscode/test/` — activation → tree populates → edit triggers bubble within 1 s → embedding picker lists stub with Ollama down → embedding picker lists Ollama models against a mock `127.0.0.1:11434` server → cluster + report webviews render → binary resolver prefers PATH when version matches.
- [ ] README screenshots / demo GIF emphasising the live bubble. Marketplace listing headline: "the first clone detector that tells you you're duplicating code as you type."
