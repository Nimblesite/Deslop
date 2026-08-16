# Configuration

### [EXCLUSION-CONFIG] Exclusion configuration
Deslop ships with conservative built-in defaults, and a `.deslop.toml` in the scan root, or `--config <path>`, extends those defaults with project-specific rules. Motivating case: generated code. We want to know when hand-written code duplicates a generated file, but we do not want generated files or build outputs to dominate the top of the report.

**Tiers.**

- `exclude` — matching files are dropped in [PIPELINE-DISCOVER-FILES] before parsing. They are not counted in `files_analysed`, never fingerprinted, never embedded, and cannot appear in any cluster. Use for third-party vendored code you do not want analysed at all.
- `report_hide` — matching files **are analysed** and can contribute to clustering, but each occurrence is flagged `hidden = true` at render time. A cluster where **every** occurrence is hidden is dropped from the rendered `clusters` list and counted under `clusters_hidden`. A cluster with at least one non-hidden occurrence is kept intact so the user sees "regular code duplicates generated code." This is the default tier for generated output like `*.g.cs`, `*.generated.cs`, OpenAPI clients, protobuf output.

**Built-in defaults.** Without a config file, Deslop excludes dependency and build directories per [CONFIG-EXCLUDE-BUILTIN] and report-hides generated output (`generated` path components, Alembic migration files under `alembic/versions`, plus suffixes such as `.g.cs`, `.generated.cs`, `.designer.cs`, `.pb.cs`, `.openapi.cs`, `.generated.py`, `_generated.py`, `_pb2.py`, `_pb2_grpc.py`). Project config adds to these defaults.

**File format.** TOML. Parsed via the `toml` crate. Minimal, familiar, diffable:

```toml
[defaults]
exclude = ["vendor/**", "third_party/**"]
report_hide = ["**/*.generated.cs", "**/*.g.cs"]

[language.csharp]
report_hide = ["**/Migrations/**/*.cs"]

[language.rust]
report_hide = ["**/target/**"]
```

**`[ranking]` section.** Controls the clone-category policy of [RANK-CATEGORY]:

```toml
[ranking]
data_clones = "demote"      # "demote" (default) | "ignore" | "keep"
data_clone_weight = 0.15    # multiplier in demote mode; finite, in (0.0, 1.0]
```

`data_clones` selects how `data`-category clusters ([CLONE-NOISE-DART-DATA-TABLE-LITERAL]) are ranked: `demote` (default) down-weights them by `data_clone_weight`, `ignore` drops them from the report, `keep` ranks them at full weight. `data_clone_weight` must be finite and strictly inside `(0.0, 1.0]`; `NaN`, infinity, `0.0`, and values above `1.0` are rejected with a `ConfigThreshold`-style error naming the config path. The weight is consulted only in `demote` mode. Both keys are omittable; absence yields the default `demote` / `0.15`.

**`[metrics]` section** (lands with gh #344, [pipeline.md §METRICS-REPO-WEIGHTED](pipeline.md#metrics-repo-weighted)). Overrides the evidence weights of the weighted duplication percentage:

```toml
[metrics.bucket_weights]
structural_only = 0.15      # each key optional; defaults per [METRICS-REPO-WEIGHTED]
same_behavior = 0.5

[metrics.category_weights]
data = 0.15
```

Every value must be finite and in `[0.0, 1.0]`, rejected otherwise with the same `ConfigThreshold`-style error as `[ranking]`. `0.0` is legal here (unlike the ranking multipliers): it removes that class from the weighted numerator only. Nothing in this section can alter the mechanical `duplication_percent` — that figure has no configuration surface, by design.

**`[tuning]` section.** Every accuracy lever of the detector, one sub-table per pipeline stage. The levers, their defaults, and the provenance of each default are specified in [fusion.md §FUSION-TUNING-LEVERS](fusion.md#fusion-tuning-levers); this section defines only the file surface. Migration is planned in [`plans/unhardcode-tuning-plan.md`](../plans/unhardcode-tuning-plan.md).

```toml
[tuning.admission]            # every key optional; absence inherits the default
fused_threshold = 0.85
lsh_only_min_jaccard = 0.90

[tuning.content_gate]
support_floor = 0.7
promote_floor = 0.85

[tuning.representation]       # cache-keyed — see [CONFIG-TUNING-CACHE]
kgram_width = 5
```

Similarity, agreement, and cosine keys must be finite and in `[0.0, 1.0]`; multiplier keys finite and in `(0.0, 1.0]`; count keys `≥ 1`. Violations are rejected with the same `ConfigThreshold`-style error as `[ranking]`, naming the key and the invariant. Clamping a bad value is prohibited — it would produce a report the config does not describe.

**Cross-key invariants**, each rejected at load because violating it makes a downstream stage unreachable: `content_gate.support_floor ≤ content_gate.promote_floor` (or the middle zone of [FUSION-CONTENT-GATE] is empty); `content_gate.structural_only_max_support < content_gate.support_floor` (or the two routes into `structural_only` stop being distinguishable); `admission.lsh_only_min_jaccard ≥ admission.fused_threshold` (or the unanchored-pair guard is dead code); `candidates.embedding_min_cosine ≤ admission.fused_threshold` (or embedding candidates are filtered by a bar admission never applies); `content_gate.saturating_token_floor ≤ routing.identical_token_floor` (or a shape match routes `identical` without the gate firing); `representation.minhash_signature_len % representation.lsh_bands == 0`.

**Precedence**, highest first: `--tune <table>.<key>=<value>` CLI flag, then the editor settings channel (`crate::state`, as [VSIX-SETTINGS-RANKING] already does for `structural_only`), then `[tuning]`, then the compiled default. Resolution happens once at config load into one immutable value; no stage reads a global at comparison time.

### [CONFIG-TUNING-CACHE] Representation keys are cache-keyed

`[tuning.representation]` keys change what is hashed or what is dispatched to the embedding provider, so a cached artefact written under one value describes nothing under another. `min_nodes` is already in the fingerprint cache key ([PIPELINE-INCREMENTAL]); the rest are not, because they could not previously vary. The key extends to the whole sub-table in the same change that makes any of them configurable — serving stale fingerprints as fresh is a false negative manufactured by the cache. `rows_per_band` is derived as `minhash_signature_len / lsh_bands` and is never a key.

Every other `[tuning]` sub-table applies downstream of both caches and never invalidates them.

### [CONFIG-TUNING-DECLARED] The report declares the tuning that produced it

A percentage or a cluster count is only meaningful alongside the thresholds that produced it. Every report carries the effective value and source (`default` | `config` | `cli` | `editor`) of each lever: in full in the JSON report ([PRINCIPLES-AUDIENCE-AGENT]), and as a one-line statement — `tuning: defaults`, or `tuning: N overridden` naming them — in the human HTML and text summaries, because a reader comparing two reports must know whether the levers moved before concluding the code did.

The corpus gate and its known-failures ratchet ([CORPUS-*]) run at defaults, always. Figures from a tuned run are not comparable to a default run's, and a corpus baseline recorded under non-default tuning is invalid.

**Pattern semantics.** `ignore::gitignore` syntax. Same engine as [PIPELINE-DISCOVER-FILES] so patterns behave identically to `.gitignore`. Paths are matched relative to the scan root.

**Merge rule.** Per-language sections **extend** `[defaults]`, they do not replace it. A `.rs` file is checked against `defaults.report_hide ∪ language.rust.report_hide`. Keeps the config declarative — you never have to repeat shared patterns in every language block.

**No config is valid.** Absence of `.deslop.toml` is not an error and is not warned on; Deslop still applies the built-in generated/build filters above.

**`report_hide` membership is a rendering decision, not an analysis one.** Hidden files still participate in fingerprinting, LSH, and (later) embedding. The `hidden: bool` per occurrence is the only surface-level signal of the policy, so downstream consumers that want the unfiltered view can ignore `clusters_hidden` and inspect `occurrences[].hidden` directly.

### [CLONE-NOISE-DART-DATA-TABLE-LITERAL] Dart collection-literal data tables

A top-level Dart collection literal whose elements are repeated near-identical data — `List<Highlight> highlights = [ Highlight(title: …, wonder: …), Highlight(title: …, wonder: …), … ]`, a `Set` of constructor calls, or a `Map` of literal entries — clusters via the sibling-window pass because Type-2 normalisation collapses every field value to the same shape. This is real repetition, but it is *data*, not extractable logic: the constructor's purpose is to enumerate per-row fields. The class-field registry filter (#169, [pipeline.md §PIPELINE-RANK-WORST-FIRST](pipeline.md#pipeline-rank-worst-first)) only covers runs of declarations inside a `class_body`; a top-level `List`/`Set`/`Map` literal has no enclosing `class_body`, so those tables previously fell through at full weight and dominated the ranking.

**Predicate.** A cluster is classified `data` ([RANK-CATEGORY]) when, for every member, the member's reported range covers a run of one or more sibling elements inside a `list_literal` or `set_or_map_literal`, and every covered element is a pure data shape — a constructor/factory invocation (`call_expression`), a `record_literal`, a map `pair`, or a bare literal — with **no** embedded `function_body` or `function_expression` (a closure-bearing element keeps clustering as logic). Reuses the same CST-walk helpers as #169 (`enclosing_kind`, `node_contains_kind`, `node_intersects_range`).

**Verbatim escape hatch.** Classification requires at least two members to differ in raw bytes (`raw_snippet_texts_differ`), matching #104/#133/#169. A *verbatim*-copied table is genuine copy-paste duplication and must still surface at full `logic` weight, never demoted.

### [CLONE-NOISE-LITERAL-TABLE] Language-agnostic literal-dominated tables

The Dart predicate above ships per-grammar CST knowledge, so every other language reported data tables at full `logic` weight (gh #336: an F# integer array literal family ranked #1 on `dotnet/fsharp`). The language-agnostic test needs no grammar tables: the pipeline already walks each cluster member's **normalised** tree for the content gate ([fusion.md §FUSION-CONTENT-GATE](fusion.md#fusion-content-gate)), and a subtree whose collapsed leaves are overwhelmingly `__literal__` positions *is* a data literal in any language.

**Predicate.** A cluster is classified `data` ([RANK-CATEGORY]) when the canonical member's collapsed leaves are ≥ 0.8 literal positions with at least 8 literals (so a tiny literal-heavy subtree — a tuple return, a short argument list — never registers as a table), and at least two members differ in raw bytes — the same verbatim escape hatch as the Dart predicate. The fraction is measured in the pipeline, where normalised trees live, and travels on the cluster; the classifier composes with the per-language predicates rather than replacing them (identifier-heavy constructor-row tables stay Dart-specific).

**Routing interaction.** A literal-dominated shape-only family stays in the surfaced `structural_only` tier instead of the hidden cross-file-scaffolding one: the `[ranking] data_clones` policy ([RANK-CATEGORY]) owns its visibility — demoted by default, dropped under `ignore`, restored by `data_clone_weight = 1.0` — and a policy knob cannot govern a cluster the renderer already hid.

This predicate feeds the [RANK-CATEGORY] policy: under the default **demote** mode the table is down-weighted and labelled `category="data"`; under **ignore** it is dropped; under **keep** it ranks at full weight.

### [CONFIG-EXCLUDE-BUILTIN] Built-in component exclusion

Two fixed lists of directory names, matched case-insensitively against **path components**, not glob patterns. They apply before any `.deslop.toml` rule and independently of `.gitignore`.

**Dependency components** — `node_modules`, `vendor`, `.cargo`, `.pub-cache`, `.venv`. Third-party library source installed or vendored into the corpus. Real, readable source the user did not write. Governed by [CONFIG-EXCLUDE-DEPENDENCIES].

**Artefact components** — `target`, `dist`, `build`, `__pycache__`, `.dart_tool`, `.git`, `.claude`. Compiler and codegen output, tool caches, and whole additional checkouts of the same repository (`.claude/worktrees/<id>/`, gh #222 — without this every file reports as N identical copies). No configuration opts back into these: none of it is source the user wrote, and none of it is a library the code depends on.

**Corpus scope.** Only components **at or below the scan root** are tested. A component above the scan root records where the checkout happens to sit on disk and says nothing about its contents; the user's choice of scan root *is* the request to analyse what is under it.

Matching ancestors was gh #342: a checkout at `~/build/myrepo` (or under `dist`, `target`, `vendor`, `node_modules`) excluded **every** file in the repository, and the run reported `files_analysed: 0`, `clusters: []`, `duplication_percent: 0.0`, `threshold.breached: false` and exited `0`. A total, silent false negative — indistinguishable from a genuinely clean repository, so neither the user nor the `--fail-over` CI gate could detect it. The report-hide tier already scoped itself this way: `scan_root_contains_component_pair` exempts a root that sits inside a hidden component pair for exactly this reason.

**Unknown boundary.** When no scan root is bound, or the path lies outside it, the rule does not fire. That direction can only admit a file for analysis; it can never silently discard one. Every discovery path binds its root: batch discovery ([PIPELINE-DISCOVER-FILES]), incremental session updates, and the live watcher ([live.md §LIVE-WATCHER](live.md#live-watcher-file-watcher)). The latter two have neither a hidden-directory filter nor a `.gitignore` pass, so this rule is their only built-in filter and a missing root would silently widen what they analyse.

Code: `crates/deslop-core/src/config.rs::corpus_built_in_excluded`. Tests: `crates/deslop/tests/issue_342_scan_root_under_excluded_ancestor.rs`, `crates/deslop/tests/go_vendor_exclusion.rs`.

### [CONFIG-EXCLUDE-DEPENDENCIES] Analysing dependencies

```toml
[analysis]
include_dependencies = false
```

Default `false`: the dependency components of [CONFIG-EXCLUDE-BUILTIN] are excluded. Ranking is worst-offenders-first ([PIPELINE-RANK-WORST-FIRST]), so dependency duplication the user cannot act on would otherwise outrank every first-party finding.

Opt-in: `include_dependencies = true` stops the dependency list applying, admitting third-party library source into discovery. Use it to audit a dependency for duplication, or to ask whether first-party code re-implements a library it already depends on.

The artefact components apply under either setting — "analyse the libraries I depend on" is not "analyse my compiler output". The setting is global for the run and orthogonal to scan-root ancestry: a checkout that merely lives under a directory named `vendor` behaves identically to one that does not, under either value.

Code: `crates/deslop-core/src/config.rs::dependency_components`. Tests: `crates/deslop/tests/config_include_dependencies.rs`.

### [CONFIG-CROSS-LANGUAGE] Cross-language comparison
The same `.deslop.toml` file controls whether clone candidates may span different parser language ids.

```toml
[analysis]
allow_cross_language_comparison = false
```

Default: `false`. Candidate pairs whose two fingerprints belong to different languages are dropped before fusion and transitive-closure clustering. This keeps normal reports focused on code that developers can realistically refactor together and prevents mixed-language scaffolding from dominating the top offenders list.

Opt-in: set `allow_cross_language_comparison = true` to preserve the full language-agnostic candidate union. This is useful for audits that intentionally compare ports, generated client libraries, or semantic equivalents across ecosystems. The option is global for the run; per-language overlays still apply only to exclusion and reporting policy.
