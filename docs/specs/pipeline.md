# Pipeline stages (v1, hybrid by default)

### [PIPELINE-LANG-TRAIT] Language plugin trait
The single extension point. Implementations live in `deslop-core::lang::<name>`. Each implementation provides: (a) tree-sitter grammar factory, (b) file-extension filter, (c) per-language node-kind normalization rules that collapse identifier / literal / trivia nodes into their structural kind. The trait output type (`NormalizedNode`) is identical across languages so downstream stages are language-agnostic. v1 ships with three plug-ins: `csharp` (`tree-sitter-c-sharp`), `rust` (`tree-sitter-rust`), and `python` (`tree-sitter-python`). Adding a language = one `LanguageParser` impl + pinning the grammar version in `Cargo.toml`. Shared walking / interning plumbing lives in `lang::shared` so every language module is just a `normalise_kind` match plus boilerplate.

### [PIPELINE-DISCOVER-FILES] File discovery
Walk the target path with the `ignore` crate, respecting `.gitignore` and Git's standard ignore rules. Filter by the set of file extensions contributed by registered `LanguageParser`s. Additionally drop paths matching built-in or configured `[EXCLUSION-CONFIG]` `exclude` patterns — those files are never parsed. Every surviving path is registered with [STATE-FILE-REGISTRY] and downstream code traffics in `FileId`, never `Path`.

### [PIPELINE-NORMALIZE-AST] AST normalization
For each file, parse with the selected language's tree-sitter grammar and walk the resulting tree bottom-up, producing `NormalizedNode { kind: &'static str, children: Vec<Self>, byte_range, file_id }`. Identifier / literal / comment / whitespace nodes are collapsed to their structural kind so Type-2 clones (renamed identifiers) hash identically. Byte ranges are preserved and are the source of truth for any later rendering — line numbers are derived.

### [PIPELINE-BOILERPLATE-FILTER] Boilerplate-only clone filtering
Language front-ends classify syntax-only scaffolding before fingerprinting. Import declarations, C# `using` directives, namespace/package headers, Python decorators such as FastAPI route declarations, and equivalent module prologues are treated as **boilerplate carriers**, not business logic. A subtree or sibling window made only from these carriers is excluded from structural fingerprints, sibling-extension windows, token LSH input, and embedding input by default.

Rationale: clone detection literature and mature tools normalize or filter irrelevant syntactic features before comparison. Repeated import blocks produce high-copy false positives that drown out actionable duplication. They are still useful style signals, so the renderer may emit a low-noise action hint rather than a clone warning.

C# special case: if the same non-static `using` directives appear across many files in the same project, the human-facing hint is `Consider moving repeated usings to a global using file` and links to the affected namespaces. This is not a clone cluster and does not contribute to `weight` or `duplicated_loc`. The JSON/AI report may carry the suppressed byte ranges as `boilerplate_hints` so an agent can propose a safe `GlobalUsings.cs` or project-file `<Using Include="..." />` change.

Configuration:

- Default: boilerplate-only clones are suppressed and no hint is emitted (`boilerplate.imports = "suppress"`).
- Opt-in diagnostic mode: `.deslop.toml` can set `boilerplate.imports = "report"` under `[defaults]` or `[language.<id>]` to include them as low-severity hints for teams that explicitly want import hygiene audits. This mode does not restore import-only clone warnings; it emits structured `boilerplate_hints` instead.
- No mode may rank import/using-only ranges above executable or declarative code clones.

### [PIPELINE-FINGERPRINT-MERKLE] Structural fingerprint (Merkle)
Bottom-up Merkle hash over `NormalizedNode`. Each node's hash combines its own `kind` string with the ordered hashes of its children using `blake3`. Each node stores `(hash, subtree_node_count, byte_range, file_id)`. Nodes whose subtree size is below `--min-nodes` are excluded from clustering per [DECISION-MIN-NODES].

### [PIPELINE-CLUSTER-EXACT] Exact subtree clustering
Group `NormalizedNode` fingerprints by `hash`. Every bucket with ≥ 2 entries is a candidate clone cluster. Covers Type-1 and normalized Type-2 deterministically in O(n). Candidate pairs are language-scoped by default per [CONFIG-CROSS-LANGUAGE]; the exact same hash may still be compared across languages when `.deslop.toml` opts into cross-language comparison.

### [PIPELINE-RANK-WORST-FIRST] Ranking: worst offenders first
`weight = clone_node_count × (cluster_size − 1) × log2(1 + total_spanned_loc)`. Clusters are sorted by weight descending. A cluster with one member (no duplication) scores zero by construction. Later stages multiply in the fusion score from [FUSION-STRATEGY-MAX-SUM]. For rendered (visible) ordering, `cluster_size` counts only non-hidden occurrences, so a mixed cluster's [EXCLUSION-CONFIG] `report_hide` members do not push it above fully-actionable clusters.

### [STATE-FILE-REGISTRY] File registry (the only global state)
`deslop-core::state::FileRegistry` maps `FileId ↔ PathBuf`. This is the *only* place mutable state associated with a pipeline run may live. Instances are per-run (not process-global) so a future long-running daemon can keep multiple analyses side-by-side.

### [OUTPUT-SCHEMA-JSON] Canonical JSON schema
JSON is the canonical report format ([PRINCIPLES-AUDIENCE-AGENT]). Text and HTML are derived from it — nothing lives in two places. Text is terse and AI-readable (ASCII, line-oriented, no colour). HTML is single-file, inline-CSS, human-readable, and embeds the same `schema_doc` and `action_hints` the JSON carries so a human opening the file cold understands what they are looking at.

Top level:

- `tool_version: String` — producer binary version.
- `min_nodes: u32` — subtree size floor used for the run.
- `files_analysed: usize` — count of files actually parsed.
- `clusters_hidden: usize` — clusters that existed but were suppressed from `clusters` because every occurrence matched a [EXCLUSION-CONFIG] `report_hide` pattern. Surfaces the volume of ignored duplication without leaking the content.
- `cache_stats: { hits: usize, misses: usize }` — incremental fingerprint-cache telemetry per [PIPELINE-INCREMENTAL]. Both zero when `--incremental` was not passed; otherwise `hits + misses == files_analysed` for files whose language has a registered parser.
- `metrics: RepoMetrics` — repo-wide duplication totals per [METRICS-REPO]. Always populated; zero when no duplication exists.
- `schema_doc: &'static str` — markdown explaining every field, signal, threshold, ranking formula, byte-range convention, and clone taxonomy. Shipped via `include_str!` so it cannot drift from the schema.
- `action_hints: Vec<ActionHint>` — short playbook entries ("high structural + high jaccard → extract shared function", etc.) agents can consult before deciding how to act.
- `embedding_provenance: Option<EmbeddingProvenance>` — provider/model identity plus `attempted_subtrees`, `indexed_subtrees`, and `failed_subtrees` so embedding coverage is visible. Duplicate successful snippets collapse before ANN indexing, and provider-rejected subtrees are omitted from the embedding ANN input; they are never represented as zero vectors.
- `clusters: Vec<ReportCluster>` — ranked worst-offenders-first per [PIPELINE-RANK-WORST-FIRST].

`ReportCluster`:

- `id`, `weight`, `size`, `canonical_node_count`, `signals { structural, token_jaccard, embedding_cos, fused }`, `summary` — as in v1.
- `interpretation: String` (new in v2) — one-line synthesis computed from the signal combination ("Type-1 exact clone, safe to extract", "Type-3 near-miss, review before merging", "Low-information LSH-only match, treat as hint"). Derived, so rendering is deterministic.
- `occurrences: Vec<ReportOccurrence>` — each with `path`, `start_byte`, `end_byte`, and `hidden: bool` (true when the occurrence matched a `report_hide` pattern per [EXCLUSION-CONFIG]).

`--from-report <file.json>` skips analysis and re-renders the text + HTML views from a canonical JSON report. Keeps the rendering pipeline testable in isolation and makes re-formatting a cached report free.

The default invocation writes all three formats to disk (`deslop-report.{json,txt,html}` in CWD, or `<path>.{json,txt,html}` when `--output <path>` is given). `--nojson`, `--notext`, `--nohtml` suppress individual formats; at least one must remain enabled.

### [METRICS-REPO] Repo-wide duplication metrics

One honest number, computed deterministically from the same cluster set the report already carries. Lives at `Report.metrics` and drives the fail-over threshold in [EXIT-CODES].

`RepoMetrics` fields:

- `analysed_loc: u64` — physical lines across every file in `files_analysed`. Counted once per file, regardless of clustering. Lines are `\n`-terminated plus the trailing partial line if any; empty files contribute zero.
- `duplicated_loc: u64` — lines covered by **≥ 2 clone occurrences across the whole corpus**, deduplicated per file so overlapping sibling-extension ranges do not double-count. Computed by projecting every `ReportOccurrence` from every non-hidden cluster onto a per-file `BTreeSet<line>`, unioning, and summing set sizes. Hidden occurrences (`[EXCLUSION-CONFIG]` `report_hide`) are **excluded** so a noisy generated-code tier cannot inflate the metric.
- `duplication_percent: f64` — `100.0 × duplicated_loc / analysed_loc`, clamped into `[0.0, 100.0]`. Zero when `analysed_loc == 0`. Rounded to two decimals in text + HTML; carried at full `f64` precision in JSON.
- `clusters_total: usize` — count of clusters contributing to `duplicated_loc` (i.e. non-hidden clusters with ≥ 2 occurrences). Matches `clusters.len()` but is carried explicitly so downstream consumers don't re-derive it.
- `duplicated_files: usize` — count of files containing at least one non-hidden clone occurrence. Upper-bounded by `files_analysed`.

Deliberate non-metrics:

- No weight-sum percentage. `weight` is a ranking quantity, not a fraction, and mixing a log term into a percentage produces a number nobody can reason about.
- No byte-level percentage. Developers reason in lines; a 3-line and a 30-line occurrence are not interchangeable even if their byte counts are similar.
- No "clone density per KLOC". Derivable from `duplicated_loc / analysed_loc * 1000`; we don't ship two spellings of the same ratio.

The text renderer prints a one-line header: `repo: 12.4% duplicated (1 843 / 14 876 LOC, 27 clusters across 11 files)`. HTML surfaces the same line in the report header and colours it by the fail-over threshold (green < threshold, red ≥ threshold, neutral when no threshold is set). JSON is canonical; both renderers read from `metrics`.

### [EXIT-CODES] CLI exit codes and fail-over threshold

Deslop's default exit code is `0` on a successful analysis regardless of how much duplication exists — the tool is diagnostic, not opinionated. Opt-in CI gating is expressed through a single flag and a single config key.

Exit codes:

- `0` — analysis succeeded; `duplication_percent ≤ threshold` (or no threshold was set).
- `1` — unexpected runtime error (parse failure, I/O error, cache corruption that couldn't be recovered). Pre-existing behaviour; unchanged by this spec.
- `2` — invalid CLI invocation (bad flag, incompatible combination, missing required argument). Pre-existing behaviour; unchanged.
- `3` — **duplication threshold breached.** `metrics.duplication_percent > threshold` after a successful analysis. The report is still written to disk in full so CI can surface the offenders.

Threshold sources, highest precedence first:

1. `--fail-over <percent>` CLI flag. Accepts a finite float in `[0.0, 100.0]`. `--fail-over 0` means "fail on any duplication". Invalid values → exit `2` with a named error.
2. `[threshold] max_duplication_percent` in `.deslop.toml` (or the file passed via `--config`). Same validation rules.
3. Absent — no threshold is enforced; exit `3` is unreachable and the text/HTML headers render the metric without a pass/fail verdict.

A `--no-fail-over` flag (mutually exclusive with `--fail-over`) overrides a config-file threshold and restores the "report only" behaviour, so a developer can run the CLI locally against a repo whose CI gate they don't want to trip.

The renderer always states the active threshold in the report header (`threshold: 10.00% (breached)` / `threshold: 10.00% (ok)` / `threshold: none`) so the report is self-explanatory when read out of context. The threshold value and breach flag are carried on `Report.metrics.threshold { percent: f64, breached: bool, source: "cli" | "config" | "none" }` so downstream tools do not re-derive the verdict.

### [PIPELINE-INCREMENTAL] Incremental fingerprint cache
Opt-in on-disk cache keyed by `(language_id, tool_version, min_nodes, content_hash)`. Cache hit rehydrates both the structural fingerprints and the normalised AST from a compact little-endian binary blob, so unchanged files skip tree-sitter entirely; cache miss parses the file and persists the result. Any mismatch on the cache key — tool upgrade, grammar pin, `--min-nodes` change, source edit — degrades gracefully to a miss; stale blobs never leak into a run.

**Activation.** Enabled with `--incremental` (off by default so read-only checkouts never get mutated). Stats land on every report as `cache_stats { hits, misses }` at top level. Text renderer surfaces them as `cache: N hit / M miss`.

**Layout.** `<root>/.deslop-cache/fingerprints/<language_id>/<tool_version>/<min_nodes>/<content_hash>.bin`. Shares `.deslop-cache/` with the embedding cache from [FUSION-EMBED-PROVIDER]; the two layers invalidate independently.

**Format.** `u32` magic, then a recursive `NormalizedNode` tree (`u32 kind_len`, kind UTF-8 bytes, `u64 start`, `u64 end`, `u32 child_count`, children...), then `u64 fingerprint_count` followed by one `{ [u8;32] hash, u64 start, u64 end, u64 node_count }` record per fingerprint. No serde, no schema drift: the magic + tool-version path segment bracket every format change.

**Failure modes.**

- Corrupt or truncated blob → treated as a miss, logged at `warn!`, overwritten by the next successful parse.
- Cache directory unavailable (permissions, read-only fs) → `FingerprintCache::open` fails, the pipeline falls back to the full parse path for the affected language, logs `warn!`, keeps running.
- Blob write fails (e.g. disk full) → `warn!`, return the in-memory result, pipeline continues.

Zero-zero stats indicate the pass ran without the cache (`--incremental` not passed or discovery yielded nothing). Any non-zero counter proves the cache was consulted.

### [OUTPUT-HUMAN-HTML] Human-readable HTML mode

The default HTML renderer embeds, for each occurrence, the source bytes covered by `[start_byte, end_byte)` inside a collapsible `<details>` panel with line numbers and tree-sitter-driven syntax highlighting (server-side, no JS). Snippets are computed at render time from the source tree — not added to the JSON schema. `--human=off` falls back to the terse byte-offset-only HTML.

## Pipeline summary (numbered)

1. **Parser:** tree-sitter per language (C#, Rust, Python) — already mandated by CLAUDE.md.
2. **Normalization:** strip identifiers, literals, comments, whitespace. Keep operators, keywords, and structural node kinds. Per-language rules; identical output format across languages so downstream layers are language-agnostic.
3. **Structural fingerprint:** bottom-up Merkle hash of every AST subtree with ≥ N nodes (configurable, default ~30). Each node stores `(hash, size, byte_range, file_id)`.
4. **Exact-clone clustering:** group subtrees by hash. O(n) after hashing. Covers Type-1 and normalized Type-2.
5. **Near-clone extension:** for each exact cluster, extend matches over adjacent sibling subtrees (Chilowicz's approach). Catches a chunk of Type-3 without tree-edit-distance.
6. **Token MinHash/LSH pass:** normalized k-grams → MinHash → LSH buckets. Catches Type-3 where structure diverged but token bag is close. Deterministic.
7. **Embedding pass (pluggable provider, local-by-default):** embed every AST subtree from step 3 through the configured `EmbeddingProvider`. Provider (`--embedding-provider`, default `ollama`) and model (`--embedding-model`, default `nomic-embed-code`) are runtime-selectable — never hard-coded. Index via HNSW (`usearch` or `instant-distance`, both pure Rust). For each subtree, retrieve top-k neighbors above a cosine threshold. Catches Type-3 and Type-4 the prior passes miss. **First-class pipeline stage, not optional.**
8. **Candidate union + fusion:** union the pairs produced by steps 4, 6, 7. Drop pairs whose fingerprints come from different language ids unless [CONFIG-CROSS-LANGUAGE] is explicitly enabled. For each remaining pair compute `(structural_sim, token_jaccard, embedding_cos)`, normalize, combine via **max-normalized sum** (per ensemble-LLM 2025). Cluster by transitive closure above a threshold.
9. **Ranking score:** `weight = clone_node_count × (cluster_size − 1) × log(total_spanned_loc) × fusion_score`. Sort descending. `cluster_size − 1` ensures singletons score zero.
10. **Output (agent-first):** JSON is canonical; text is a pretty-printer over the same struct. Each cluster: exact byte ranges, file paths, canonical representative snippet, per-signal scores (structural / LSH / embedding), a short agent-oriented `summary`, and a refactor hint where reliably inferrable. ASCII-only text format; no colour codes, no paging. See "Audience for the report" above.
11. **Incremental cache:** `(file_content_hash, provider_id, model_id, model_version) → (parse_tree, subtree_fingerprints, embeddings)`. Re-runs with unchanged files skip all inference. v1 uses this to make batch runs cheap; v2 uses the same keys for a watcher-driven update loop. Switching embedding provider/model invalidates only the embedding layer, not the structural/LSH caches.
12. **Library vs binary split:** `deslop-core` owns the pipeline. `deslop` binary is a thin shell. An MCP/LSP daemon binary is a later sibling shell over the same crate — no pipeline code moves.
13. **Incremental update entry point from day one:** `deslop-core` exposes `update_files(changed: &[FileId]) -> ReportDelta` as a first-class API, even though v1's only caller is `main`. This is the function the future file watcher will call.
14. **Embedding disable flag:** `--embeddings={auto,required,off}`. `off` runs the deterministic two-pass pipeline only; the report header notes reduced Type-4 recall. `auto` (default) uses embeddings when the configured provider is reachable and falls back with a `tracing::warn!` otherwise. `required` hard-fails if the provider is unreachable.
15. **Provider/model pinning:** `provider_id`, `model_id`, and `model_version` are written into the cache header and into every report. Changing any of them invalidates the embedding layer deterministically and explicitly.
