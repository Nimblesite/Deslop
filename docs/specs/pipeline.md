# Pipeline stages (v1, hybrid by default)

### [PIPELINE-LANG-TRAIT] Language plugin trait
The single extension point. Implementations live in `deslop-core::lang::<name>`. Each implementation provides: (a) tree-sitter grammar factory, (b) file-extension filter, (c) per-language node-kind normalization rules that collapse identifier / literal / trivia nodes into their structural kind. The trait output type (`NormalizedNode`) is identical across languages so downstream stages are language-agnostic. v1 ships with `csharp` (`tree-sitter-c-sharp`), `rust` (`tree-sitter-rust`), `python` (`tree-sitter-python`), `dart` (`tree-sitter-dart`), `javascript` (`tree-sitter-javascript`), `typescript`, `tsx` (`tree-sitter-typescript`), `php` (`tree-sitter-php`), `fsharp` (`tree-sitter-fsharp`, source grammar `LANGUAGE_FSHARP` for `.fs`/`.fsx`), and `go` (`tree-sitter-go`). Adding a language = one `LanguageParser` impl + pinning the grammar version in `Cargo.toml`. Shared walking / interning plumbing lives in `lang::shared`; JavaScript, TypeScript, and TSX additionally share `lang::ecmascript` so their normalisation surface stays aligned.

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

### [PIPELINE-DETERMINISM] Cross-run determinism
Two runs of the pipeline over an unchanged corpus produce bit-identical
deterministic output: identical MinHash signatures (blake3 XOF, fixed k-gram
ordering), identical fused signal scores (`token_jaccard` compared bit-for-bit),
identical candidate sets, cluster ids, and ranking. Determinism is what makes the
fingerprint cache ([PIPELINE-INCREMENTAL]) sound and cluster ids stable across
sessions. The embedding/ANN layer is the only approximate stage and is bounded
separately ([FUSION-EMBED-PROVIDER]); a missed ANN neighbour only loses recall,
never changes existing cluster content.

### [PIPELINE-INCREMENTAL] Incremental fingerprint cache
On-disk cache keyed by `(language_id, tool_version, min_nodes, content_hash)`. Cache hit rehydrates both the structural fingerprints and the normalised AST from a compact little-endian binary blob, so unchanged files skip tree-sitter entirely; cache miss parses the file and persists the result. Any mismatch on the cache key — tool upgrade, grammar pin, `--min-nodes` change, source edit — degrades gracefully to a miss; stale blobs never leak into a run.

**Activation.** On by default on every surface. Incremental analysis is a first-class path, not a bolt-on: the LSP runs on it permanently, and a batch CLI run is just "incremental starting from an empty cache". `deslop --no-incremental` opts out for callers that must not write to the tree at all. Stats land on every report as `cache_stats { hits, misses }` at top level. Text renderer surfaces them as `cache: N hit / M miss`.

**[PIPELINE-INCREMENTAL-INVALIDATION] Invalidation is addressing, not bookkeeping.** The cache cannot serve a stale parse, because a stale parse is *unaddressable*: the blob's filename is `blake3(file contents)`, under path segments for language, tool version, and `min_nodes`. Edit a file with nothing watching — an agent writing, a `git checkout`, an editor with the LSP stopped — and its content hash changes, so the lookup lands on a path that does not exist and the file is re-parsed from disk. There is no mtime heuristic, no watcher-maintained index, and no invalidation step that could be skipped or get out of sync.

Two further properties make a cache-on-by-default CLI safe:

- **Corpus membership never comes from the cache.** Every run performs a fresh discovery walk, so files added or deleted while nothing was watching are picked up regardless of cache state. A deleted file's blob is simply orphaned — unused, never consulted.
- **A warm run and a cold run agree.** Wiping `.deslop/cache/` changes the `cache_stats` counters and nothing else about the report. The cache is an accelerator; the source tree is the only source of truth.

The one artefact that *can* go stale is the live state file `live-report.json` ([LIVE-STATE-FILE]) — a whole-report snapshot, not a content-addressed entry. The CLI never reads it. The LSP seeds from it for instant warm-start and immediately runs a cold pass that replaces it, reporting `Running` until that pass installs ([LIVE-CACHE-SEED]).

**Layout.** `<root>/.deslop/cache/fingerprints/<language_id>/<tool_version>/<min_nodes>/<content_hash>.bin`. Shares `.deslop/cache/` with the embedding cache from [FUSION-EMBED-PROVIDER]; the two layers invalidate independently.

**Format.** `u32` magic, then a recursive `NormalizedNode` tree (`u32 kind_len`, kind UTF-8 bytes, `u64 start`, `u64 end`, `u32 child_count`, children...), then `u64 fingerprint_count` followed by one `{ [u8;32] hash, u64 start, u64 end, u64 node_count }` record per fingerprint. No serde, no schema drift: the magic + tool-version path segment bracket every format change.

**Failure modes.**

- Corrupt or truncated blob → treated as a miss, logged at `warn!`, overwritten by the next successful parse.
- Cache directory unavailable (permissions, read-only fs) → `FingerprintCache::open` fails, the pipeline falls back to the full parse path for the affected language, logs `warn!`, keeps running.
- Blob write fails (e.g. disk full) → `warn!`, return the in-memory result, pipeline continues.

Zero-zero stats indicate the pass ran without the cache (`--no-incremental` passed, or discovery yielded nothing). Any non-zero counter proves the cache was consulted.

### [PIPELINE-RANK-WORST-FIRST] Ranking: worst offenders first
`weight = clone_node_count × (cluster_size − 1) × log2(1 + total_spanned_loc)`. Clusters are sorted by weight descending. A cluster with one member (no duplication) scores zero by construction. Later stages multiply in the fusion score from [FUSION-STRATEGY-MAX-SUM]. For rendered (visible) ordering, `cluster_size` counts only non-hidden occurrences, so a mixed cluster's [EXCLUSION-CONFIG] `report_hide` members do not push it above fully-actionable clusters. The final ranking weight is multiplied by the clone-category coefficient from [RANK-CATEGORY] before the visible sort, so a data-table cluster ranks below comparable logic clones.

### [RANK-CATEGORY] Clone category and the ranking policy
Every cluster carries a **clone category** that is orthogonal to the similarity bucket of [taxonomy.md §CLONE-BUCKETS](taxonomy.md#clone-buckets). The canonical category table (seven values including the literal family) lives at [taxonomy.md §CLONE-CATEGORY-REGISTRY](taxonomy.md#clone-category-registry); this section governs the two fragment-clone categories. The bucket answers *"how similar are these copies?"*; the category answers *"is this repetition extractable logic or un-refactorable data?"*:

- `logic` — ordinary duplicated code. Full ranking weight. The default.
- `data` — a data-structure literal repeated across sibling elements (e.g. a top-level `List<Model>` of near-identical constructor literals). Real repetition, but the constructor's purpose *is* to enumerate per-row fields; at best a user hoists a builder with defaults or moves the rows to a JSON/CSV/asset. Detected by [CLONE-NOISE-DART-DATA-TABLE-LITERAL].

The category drives a **three-way ranking policy** configured in `.deslop.toml` under `[ranking]` (see [CLONE-NOISE-DART-DATA-TABLE-LITERAL] for the keys):

- **keep** — both categories rank at full weight. Restores pre-category ordering.
- **demote** (default) — `data` clusters are multiplied by `data_clone_weight` (default `0.15`, strictly in `(0.0, 1.0]`) so they rank below comparable `logic` clones but remain in the report, labelled `category="data"`. The multiplier is never zero, so a pathologically large verbatim blob can still rise.
- **ignore** — `data` clusters are dropped from the report entirely (reuses the [EXCLUSION-CONFIG] cluster-hide path) and counted under `clusters_hidden`.

`data` clusters carry a category-specific action hint ("consider a builder with default args, or move the rows to a JSON/CSV/asset") instead of the "extract the duplicate" hint. The category and its label travel on the JSON `ReportCluster.category` field so every downstream surface — text, HTML, and the VSIX tree — orders and labels identically from one source of truth ([OUTPUT-SCHEMA-JSON]).

### [RANK-STRUCTURAL-ONLY] Structural-only evidence and the ranking policy

`StructuralOnly` is the [taxonomy.md §CLONE-BUCKETS](taxonomy.md#clone-buckets) bucket for clusters whose **only positive evidence is the normalized AST shape** — `structural ≥ 0.99` with token and embedding support both below `STRUCTURAL_ONLY_MAX_SUPPORT` (0.05). Normalization strips identifiers and literals, so a sibling method family (REST CRUD endpoints, settings getters, builders) collides into one shape; the exact-structural pass also leaves `token_jaccard` *unscored* at `0.0`, so the triple alone cannot distinguish boilerplate from a true Type-2 rename. The history (#134 → #154 → #169 → #197) shows why shape-specific suppressions alone never closed the hole: each fix allowlisted one geometry (≥3-file scaffolding; single-file declaration families; Dart field registries) and every other geometry kept full `NearlyIdentical`-grade weight.

This section closes the hole structurally:

1. **One predicate.** `deslop-core::buckets::is_structural_only_signals` is the single source of truth, shared by the bucket routing (the wire label) and the ranking demotion. A cluster labelled `structural_only` is by construction the cluster the policy demotes — the #197 label/ranking divergence cannot recur.
2. **Weight policy.** The `[ranking]` section gains `structural_only = "demote" | "ignore" | "keep"` (default **demote**) and `structural_only_weight` (default `0.15`, strictly in `(0.0, 1.0]`), exactly parallel to [RANK-CATEGORY]'s data knobs and validated by the same rule. The multiplier folds into the visible re-rank next to the category coefficient, so a shape-only family sinks below comparable token- or semantics-supported clones regardless of its file spread or declaration shape.
3. **Existing suppressions stay.** Cross-file ≥3-member/≥3-file scaffolding still demotes to `LooselySimilar` (hidden, #134); single-file sibling-declaration families are still hidden by the AST pass (#197); Dart data registries stay with [RANK-CATEGORY] (#169). The weight policy catches everything those shapes miss (e.g. two-file method families split by Dart `part`/extension idioms).
4. **Editor override.** The VS Code setting `deslop.ranking.structuralOnly` (`default` | `demote` | `ignore` | `keep`, [VSIX-SETTINGS-RANKING]) feeds `deslop-lsp --ranking-structural-only`, recorded once at startup in the central state module (`deslop-core::state`) and consulted by every config load — the editor channel wins over `.deslop.toml`; `default` defers to it.
5. **Filterable.** The MCP `duplicates` filter block ([MCP-TOOL-FILTERS]) derives its `buckets` enum from `ClusterKind::all()`, so `structural_only` is filterable by agents (#195/#197).

### [RANK-LITERAL-FAMILY] Literal-family weight formula and policy

Literal-family clusters ([CLONE-CATEGORY-REGISTRY] categories `magic_literal`,
`shadowed_constant`, `constant_duplicate`, `constant_drift`, `constant_alias`) interleave in the
**same** worst-first list as fragment clones — no separate section, no second sort; the facets
([FACET-MODEL]) are how users isolate them. Their base weight is a deliberate, documented fork of
[PIPELINE-RANK-WORST-FIRST], because `clone_node_count` degenerates at 1 for a bare literal token:

`weight = max(visible_occurrences − occurrence_floor + 1, 1) × length_factor × log2(1 + distinct_files)`

where `occurrence_floor` is the category's minimum (3 for `magic_literal`, 2 elsewhere) — the
`max(…, 1)` keeps the term positive when `report_hide` hides occurrences after the trigger
counted them; `length_factor = min(content_chars(normalized_value), 40)` with `content_chars` =
Unicode scalar count of the normalised value (escape sequences counted as spelled; for numbers,
the canonical normalised spelling; for `constant_drift`, the longest variant value); and
`distinct_files` counts files with visible occurrences. A literal-family cluster with fewer than 2
visible occurrences is dropped and counted in `clusters_hidden`, matching the fragment-clone rule.
The linear occurrence term follows Sonar's per-occurrence remediation model; the file-spread log
term mirrors the existing `spanned_loc` factor and rewards the cross-file repetition that the
literature ties to faults. Micro-findings are **not** down-ranked for being small — micro-clones
are measurably more bug-prone than regular clones
([reading-list.md](reading-list.md#read-list-literals)); a 40-site magic URL
belongs in top offenders.

Policy knobs in `[ranking]` (same `keep | demote | ignore` enum, validation, and
`clusters_hidden` accounting as the data/structural-only knobs; editor channel per
[LITERAL-CONFIG]):

- `magic_literals = "keep"` (default) with `magic_literal_weight = 0.3` applied only under
  `demote`. Default-keep is the evidence call: noise is controlled at detection time
  ([LITERAL-NOISE], [LITERAL-CENSUS]), not by hiding confirmed findings.
- `constant_findings = "keep"` (default) with `constant_findings_weight = 0.5` under `demote` —
  one knob covering `constant_duplicate`, `constant_alias`, and `shadowed_constant`.
- `constant_drift = "keep"` (default) | `"ignore"` — **no demote**: same-name-conflicting-values is
  a correctness risk, never quietly down-weighted. Drift clusters stamp `nearly_identical`
  ([LITERAL-WIRE]), which resolves to the warning tier under the default bucket-keyed severity
  maps — severity has no category channel; it is keyed by bucket only (#177 tracks a category-keyed
  override as out of scope here).

### [RANK-UNUSED-PUBLIC] Unused-public-constant boost (monorepo)

When the [LITERAL-UNUSED-MARKER] (literals.md) fires for **every** declaration occurrence in a
constant-family cluster, the cluster's weight is multiplied by `unused_public_weight` — an
**up**-weight, mirroring the `ClonePolicy` pattern with an inverted validation range:

- `[ranking] unused_public = "boost"` (default) | `"ignore"`.
- `unused_public_weight = 1.5`, finite, validated in `[1.0, 10.0]`.

The boost (and the marker itself) only activates in a monorepo with a non-publishable declaring
package (`[workspace] monorepo`, [LITERAL-UNUSED-MARKER]) — published public constants are exempt by
design. Conservative 1.5× because confidence caps at 90: the marker raises a duplicate constant's
priority, it never asserts deletability.

### [STATE-FILE-REGISTRY] File registry (the only global state)
`deslop-core::state::FileRegistry` maps `FileId ↔ PathBuf`. This is the *only* place mutable state associated with a pipeline run may live. Instances are per-run (not process-global) so a future long-running daemon can keep multiple analyses side-by-side.

### [OUTPUT-SCHEMA-JSON] Canonical JSON schema
JSON is the canonical report format ([PRINCIPLES-AUDIENCE-AGENT]). Text and HTML are derived from it — nothing lives in two places. Text is terse and AI-readable (ASCII, line-oriented, no colour). HTML is single-file, inline-CSS, human-readable, and embeds the same `schema_doc` and `action_hints` the JSON carries so a human opening the file cold understands what they are looking at.

Top level:

- `tool_version: String` — producer binary version.
- `min_nodes: u32` — subtree size floor used for the run.
- `files_analysed: usize` — count of files actually parsed.
- `clusters_hidden: usize` — clusters that existed but were suppressed from `clusters` because every occurrence matched a [EXCLUSION-CONFIG] `report_hide` pattern. Surfaces the volume of ignored duplication without leaking the content.
- `cache_stats: { hits: usize, misses: usize }` — incremental fingerprint-cache telemetry per [PIPELINE-INCREMENTAL]. Both zero when `--no-incremental` was passed; otherwise `hits + misses == files_analysed` for files whose language has a registered parser.
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

The default invocation writes all three formats to disk (`.deslop/deslop-report.{json,txt,html}` under the scan root per [OUTPUT-DIR], or `<path>.{json,txt,html}` when `--output <path>` is given). `--nojson`, `--notext`, `--nohtml` suppress individual formats; at least one must remain enabled.

### [OUTPUT-DIR] Workspace output directory
Everything Deslop writes for a scanned workspace lands under a single `.deslop/` directory at the **scan root**, so a user has exactly one path to gitignore, inspect, or delete, and the three surfaces never disagree about where a workspace's artefacts live:

```text
<scan-root>/
  .deslop.toml                     # config — user-authored, tracked, NOT output
  .deslop/                         # everything Deslop writes
    deslop-report.{json,txt,html}  # rendered reports ([OUTPUT-SCHEMA-JSON])
    deslop-report.delta.json       # generation delta, when `--rerun-touch` ran ([LIVE-DELTA])
    logs/deslop-<unix-seconds>.log # tracing sink ([UX-LOG-CONSOLE])
    cache/                         # derived state — safe to delete, always rebuildable
      fingerprints/                # [PIPELINE-INCREMENTAL]
      embeddings/                  # [FUSION-EMBED-PROVIDER]
      live-report.json             # [LIVE-STATE-FILE]
      deslop.sock deslop.port      # [LIVE-IPC-SOCKET], [LIVE-IPC-TCP]
```

`deslop-core::paths` is the single source of truth for this layout; the CLI, LSP, and MCP all resolve through it rather than joining path literals of their own. Three consequences are normative:

- **The scan root, not the working directory, anchors the default.** `deslop /other/repo` writes into `/other/repo/.deslop/`, matching where the LSP and MCP already read and write for that workspace. A CLI run therefore never litters the directory the operator happened to be standing in.
- **`--output <prefix>` overrides the report base, and the logs follow it** into `<prefix-dir>/logs/`. It is the only knob: the cache stays at `<scan-root>/.deslop/cache` because the LSP and MCP must locate it from the scan root alone, with no flags to consult.
- **Logs get their own subdirectory.** Report file names are fixed and few; log file names are timestamped and accumulate, so they never bury the three files a user actually opens.

`.deslop/` is dot-prefixed, so the discovery pass's hidden-directory prune keeps Deslop's own artefacts out of the corpus it analyses. The `.gitignore` entry the VSIX offers to write is the directory-only form `.deslop/` ([VSIX-CACHE-IGNORE]), which leaves the sibling `.deslop.toml` config file tracked.

### [OUTPUT-HUMAN-HTML] Human-readable HTML mode

The default HTML renderer embeds, for each occurrence, the source bytes covered by `[start_byte, end_byte)` inside a collapsible `<details>` panel with line numbers and tree-sitter-driven syntax highlighting (server-side, no JS). Snippets are computed at render time from the source tree — not added to the JSON schema. `--human=off` falls back to the terse byte-offset-only HTML.

#### [OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS] Per-language sections

`[report] split_by_language` in `.deslop.toml` (default `false`, with a `--split-by-language` CLI mirror) divides the report body into one `<section>` per language instead of the single "Duplicate groups" section. In both modes cluster cards group into per-bucket expanders within each section ([FACET-HTML]). With the flag **off** the output is byte-identical to the single-section form — a hard no-regression invariant. With it **on**, `write_clusters` groups clusters by their canonical occurrence's `language_for_path(...)`, emits one `<h2>` per language carrying the language's display name and its group count, preserves worst-first order within each section, and orders sections by their worst cluster weight. The intro summary line ([OUTPUT-HUMAN-HTML]) gains a per-language breakdown. Each cluster is single-language ([CONFIG-CROSS-LANGUAGE]), so every group lands in exactly one section.

### [METRICS-REPO] Repo-wide duplication metrics

One honest number, computed deterministically from the same cluster set the report already carries. Lives at `Report.metrics` and drives the fail-over threshold in [EXIT-CODES].

`RepoMetrics` fields:

- `analysed_loc: u64` — physical lines across every file in `files_analysed`. Counted once per file, regardless of clustering. Lines are `\n`-terminated plus the trailing partial line if any; empty files contribute zero.
- `duplicated_loc: u64` — lines covered by **≥ 2 clone occurrences across the whole corpus**, deduplicated per file so overlapping sibling-extension ranges do not double-count. Computed by projecting every `ReportOccurrence` from every non-hidden cluster onto a per-file `BTreeSet<line>`, unioning, and summing set sizes. Hidden occurrences (`[EXCLUSION-CONFIG]` `report_hide`) are **excluded** so a noisy generated-code tier cannot inflate the metric. Literal-family clusters ([RANK-LITERAL-FAMILY]) are **excluded** from `duplicated_loc` / `duplication_percent` — the headline percentage keeps meaning fragment-clone duplication; `clusters_total` still counts them.
- `duplication_percent: f64` — `100.0 × duplicated_loc / analysed_loc`, clamped into `[0.0, 100.0]`. Zero when `analysed_loc == 0`. Rounded to two decimals in text + HTML; carried at full `f64` precision in JSON.
- `clusters_total: usize` — count of non-hidden clusters carried in `clusters`, literal-family included; always equals `clusters.len()` but is carried explicitly so downstream consumers don't re-derive it. Only fragment-clone clusters contribute lines to `duplicated_loc` — [RANK-LITERAL-FAMILY] clusters are excluded from the line projection, not from this count.
- `duplicated_files: usize` — count of files containing at least one non-hidden clone occurrence. Upper-bounded by `files_analysed`.
- `per_file: Vec<FileMetric>` — per-file breakdown, one `FileMetric { path, analysed_loc, duplicated_loc, duplication_percent }` per analysed file (clean files included with `duplicated_loc == 0` so percentage denominators stay exact). Same per-file line-set computation as the repo aggregate, scoped to one file; `duplication_percent` uses that file's own `analysed_loc` as the denominator. Sorted by `duplication_percent` desc, path tiebreaker. **Folders are not carried on the wire** — per-folder rollups are derived by consumers (the VSIX [VSIX-METRICS-PANEL], the HTML report) by summing the `analysed_loc` and `duplicated_loc` of every file under a path prefix, which keeps both numerator and denominator exact. Powers the per-folder/per-file breakdown in [VSIX-METRICS-PANEL].

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
