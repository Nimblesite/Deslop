# Deslop Report Context

## What this is

You are being given a code-duplication report from **Deslop**, a static analysis tool that detects duplicated code using a hybrid pipeline of AST fingerprinting + token MinHash/LSH. The report lists **worst offenders first** — clusters ranked by a duplication-impact score, not just count.

## How it works (so you know what the signals mean)

The pipeline normalizes each source file's AST (collapsing identifier names, literals, and comments so renamed-identifier clones hash the same), then finds clones through three complementary passes:

1. **Structural (Merkle AST hash)** — bottom-up `blake3` hash of every AST subtree ≥ `min-nodes`. Detects Type-1 (identical) and Type-2 (renamed-identifier) clones. `structural=1.0` means "these subtrees are identical after normalization."
2. **Sibling-extension** — contiguous sibling windows under a common parent are hashed as groups. Catches near-miss clones where the same sequence of statements appears in different enclosing contexts.
3. **Token LSH (MinHash over k=5 k-grams of normalized node kinds)** — catches Type-3 clones where structure diverged but the token bag is close. `token_jaccard` is the estimated Jaccard similarity in `[0.0, 1.0]`.

Candidate pairs from all three passes are unioned, filtered to same-language endpoints by default, then transitively closed into clusters. `.deslop.toml` can opt back into cross-language comparison with `[analysis] allow_cross_language_comparison = true`. Boilerplate-only ranges such as imports, C# `using` directives, namespace/package headers, Python route decorators, and equivalent module prologues are filtered before they become clone clusters. Repeated C# `using` directives may appear as a style/action hint suggesting `global using`; they should not be interpreted as duplicate business logic.

## Clone buckets (canonical)

Every cluster in this report belongs to exactly one of five buckets. The human label is what end-users see in the CLI / HTML / VS Code UI; the academic `Type-N` label is retained here because agents often read the literature. Both labels refer to the **same** bucket.

| Bucket            | Human label                              | Academic ref     | Meaning                                                                                  |
|-------------------|------------------------------------------|------------------|------------------------------------------------------------------------------------------|
| `Identical`       | Identical code                           | Type-1, Type-2   | Identical after normalization (ignoring whitespace, comments, renamed identifiers).      |
| `NearlyIdentical` | Nearly identical code                    | Type-3           | Type-2 + added/removed/modified statements. Small differences may matter — review both.  |
| `StructuralOnly`  | Same shape, different content            | structural-only  | The AST shape is the *only* positive evidence (no token overlap, no semantic support). Usually a sibling boilerplate family (REST CRUD, settings getters); weight-demoted in the ranking by default. |
| `LooselySimilar`  | Loosely similar code                     | weak LSH-only    | Loose textual overlap below the near-miss bar; no other axis corroborates it.            |
| `SameBehavior`    | Same behavior, different code *(AI)*     | Type-4           | Semantically equivalent, syntactically different. Requires the embedding pass.           |

`SameBehavior` is populated only when the embedding pass ran. If `embedding_cos` is `0.00` across the whole report, the pass was disabled and the `SameBehavior` bucket is empty — structural / token-based clusters (`Identical`, `NearlyIdentical`, `StructuralOnly`, `LooselySimilar`) are unaffected.

`StructuralOnly` clusters rank with a configurable weight multiplier (`.deslop.toml` `[ranking] structural_only = "demote" | "ignore" | "keep"`, default `demote` at `0.15`) — so a low rank does not mean low copy count; check `size`. To exclude or isolate them in queries, filter on `bucket = "structural_only"`.

Full canonical definition including routing thresholds: [taxonomy.md §[CLONE-BUCKETS]](taxonomy.md).

## Clone category (`logic` vs `data`)

Each cluster also carries a `category` field, orthogonal to its bucket. The bucket answers *how similar* the copies are; the category answers *whether the repetition is extractable logic or un-refactorable data*.

| `category` | Meaning | Action |
|---|---|---|
| `logic` (default) | Ordinary duplicated code. | Extract the duplicated logic into a shared function. |
| `data` | A data-structure literal repeated across sibling rows (e.g. a top-level `List<Model>` of near-identical constructor literals). Real repetition, but the constructor's purpose is to enumerate per-row fields. | Consider a builder with default arguments, or move the rows to a JSON/CSV/asset file. |

`data` clusters are demoted in the ranking by default (a configurable weight multiplier) so they sink below comparable `logic` clones, and may be dropped entirely via `.deslop.toml` `[ranking] data_clones = "ignore"`. A *verbatim*-copied table (byte-identical members) stays `logic` — that is genuine copy-paste. An absent or empty `category` means `logic`. Full definition: [pipeline.md §[RANK-CATEGORY]](pipeline.md).

## How to read the report format

Each line starts with `#N` (rank, lower = worse) and looks like:

```
#1 [abc123def4567890] weight=271716.65 size=20 nodes=825
  20 copies of a 825-node subtree at path/A.cs:230-5843, path/B.cs:195-5843, path/C.cs:0-5844 (+17 more) [structural=0.00, token_jaccard=0.97, embedding_cos=0.00]
```

Fields:

| Field | Meaning |
|---|---|
| `#N` / `rank` | Rank. `#1` is the worst offender. One-based over the whole report, stamped by the engine — never re-number it from a filtered list. |
| `rank_band` | The rank's percentile band: `worst` (top 1%) · `top10` · `mid` (top 50%) · `faint`. Drives glyph density on visual surfaces. Computed by the engine; do not re-derive a percentile. |
| `[abc123…]` | Stable 16-char cluster id (hex). Same clone across runs → same id. |
| `weight` | Ranking score: `node_count × (cluster_size − 1) × log2(1 + total_spanned_bytes)`. Higher = more impact. |
| `size` | How many copies of this subtree exist. `size=20` means the same pattern appears 20 times. |
| `occurrence_count` | The authoritative display count of a cluster's occurrences. Live-wire responses cap `occurrences[]`, so this is the number to show — never `occurrences.length`. |
| `language` | The language id the parser registry resolved for the cluster (`csharp`, `rust`, `python`, `dart`, `javascript`, `typescript`, `tsx`, `php`, `fsharp`, `go`, or `unknown`). Group and filter on this rather than on a file extension. |
| `evidence_verdict` | One plain-English sentence reading the shape score against the measured content evidence — why the bucket held, fell, or came from the embedding pass. Engine-authored; render it verbatim. |
| `nodes` | AST node count of one canonical copy. Bigger subtree = more meaningful clone. |
| `path:line:column` | Human-readable occurrence location in text/HTML/hover summaries. JSON keeps `start_byte` / `end_byte` for machine navigation. |
| `structural` | `[0, 1]`. `1.0` = exact Merkle hash match. `0.0` = no exact structural match (found via LSH only). |
| `token_jaccard` | `[0, 1]`. Estimated Jaccard of normalized k-gram token sets. |
| `shape` | `[0, 1]`. The shape reading: the stronger of `structural` and `token_jaccard`. Those two are views of one normalised representation, so the max is what "the shape matched" means — take this field rather than computing the max. |
| `embedding_cos` | `[0, 1]` cosine similarity from the semantic-embedding pass, or `0.00` if that pass was disabled for this run. |
| `embedding_provenance.succeeded_subtrees` | Occurrences that obtained a vector. Together with `failed_subtrees` this accounts for every `attempted_subtrees`, so a reader can confirm nothing vanished silently. |
| `embedding_provenance.indexed_subtrees` | Count of unique successful subtree embeddings fed into ANN. Lower than `succeeded_subtrees` when duplicate snippets collapse before indexing. It counts index points, not occurrences — do not read it as coverage. |
| `embedding_provenance.failed_subtrees` | Count of subtree embeddings the provider rejected. Rejected subtrees are excluded from embedding ANN rather than substituted with zero vectors. |
| `boilerplate_hints[]` | Optional low-severity import/prologue hygiene hints emitted only when `.deslop.toml` sets `boilerplate.imports = "report"`. These carry suppressed byte ranges but are not clone clusters and do not affect `weight` or metrics. |

Byte ranges come from `tree-sitter` and remain in JSON/tool payloads. Human-facing summaries derive line and column from the same source bytes so users do not have to read raw byte offsets.

## Reading the signals together

| `structural` | `token_jaccard` | `embedding_cos` | Bucket → human label | What it means |
|---|---|---|---|---|
| `1.00` | `1.00` | any | `Identical` → **Identical code** *(Type-1/2)* | Safe candidate for extraction into a shared function/method. |
| `1.00` | `0.05 – 1.00` | any | `NearlyIdentical` → **Nearly identical code** *(Type-3)* | Same AST shape but slightly different token k-grams. Usually overlapping sibling-extension ranges. |
| `1.00` | `< 0.05` | `< 0.05` | `StructuralOnly` → **Same shape, different content** | Shape-only match: the exact-structural pass leaves `token_jaccard` unscored at `0.00`, so there is no token or semantic evidence. Sibling method families (REST CRUD, settings getters) live here; byte-equivalent copies are upgraded to `Identical` instead. Demoted in ranking by default. |
| `0.00` | `≥ 0.90` | `<0.80` or disabled | `NearlyIdentical` → **Nearly identical code** *(Type-3)* | Similar token content, different structure. Review before merging — may differ in a semantically important way (loop vs recursion, added guard, etc.). |
| `<0.50` | any | `≥ 0.80` | `SameBehavior` → **Same behavior, different code** *(Type-4, AI match)* | The embedding pass noticed these do the same thing written two syntactically distinct ways. Read both before merging. |
| `0.00` | `0.70 – 0.90` | `<0.80` or disabled | `LooselySimilar` → **Loosely similar code** | Weak signal. Likely rejected; if present, endpoints were substantial (≥ 40 nodes). |
| `0.20 – 0.80` | `≥ 0.95` | any | `NearlyIdentical` → **Nearly identical code** *(Type-3)* | Transitive closure merged several smaller exact clusters via near-miss links. Usually genuine duplication across a family of variants. |

`SameBehavior` is tested before `NearlyIdentical` when the embedding pass is enabled, so a strong semantic signal on syntactically divergent code gets the AI-match label rather than being absorbed into near-miss. Full routing table: [taxonomy.md §[CLONE-BUCKETS-ROUTING]](taxonomy.md).

## Repo-wide duplication metric

The report header carries one honest number: `metrics.duplication_percent = 100 × duplicated_loc / analysed_loc`.

- `duplicated_loc` = lines covered by ≥ 2 non-hidden clone occurrences, deduplicated per file so overlapping sibling-extension ranges count once.
- `analysed_loc` = physical lines across every file in `files_analysed`.
- Hidden occurrences (built-in generated-code defaults or `.deslop.toml` `report_hide`) are excluded so they cannot inflate the metric.
- CI gating: `--fail-over <percent>` (or `[threshold] max_duplication_percent` in `.deslop.toml`) exits `3` when `duplication_percent > threshold`. No threshold → no gate. Use this, not the `weight` column, for pass/fail decisions.

## Thresholds (typical defaults)

- `min-nodes = 15` — smaller subtrees are excluded to cut noise. The header of the report will state the value actually used.
- `FUSED_THRESHOLD = 0.85` — the default **pair admission** bar, decided pair by pair on `bounded_fused`, the strongest single axis ([FUSED-STRATEGY-BOUNDED-MAX]); per-pair data (`CandidatePair::fused_min_score`), never a global constant — explicit cross-language candidates with no structural anchor lower it to 0.10. Every threshold is a configurable default, never hard-coded. Do not assert that nothing below 0.85 was admitted — a cross-language audit legitimately admits far below it.
- **There is no cluster-level `fused`** — it exists at the level of the pair only ([FUSED-SCOPE](fused.md#fused-scope)). Clusters carry their `bucket` (the engine's verdict), the elected pair's measured axes, and their content evidence. Filter reported clusters on `bucket`, never on any confidence value.
- `LSH_ONLY_MIN_JACCARD = 0.90` and `LSH_ONLY_MIN_NODE_COUNT = 40` — extra gates for LSH-only candidates (no structural anchor), to keep tiny trivial windows from mega-clustering.
- Cross-language comparison is off by default. Enable `[analysis] allow_cross_language_comparison = true` only when intentionally auditing ports, generated clients, or semantic equivalents across ecosystems.

## What to do with this report

1. **Start at #1 and work down.** The weight formula already prioritises by impact.
2. **Check if it's generated code.** Generated files (e.g. `.g.cs`, `.generated.cs`, OpenAPI clients, protobuf output) are hidden by default because they duplicate by design; visible generated-handwritten overlap is still worth reviewing.
3. **Treat import/using-only repetition as hygiene, not duplication.** For C#, the preferred remediation is usually a shared `GlobalUsings.cs` or project-file `<Using Include="..." />`, not extraction.
4. **Check byte ranges for overlap.** Adjacent/overlapping ranges in the same file mean the sibling-extension pass is firing on several enclosing contexts of the same physical code — count it as one logical clone, not N.
5. **For `Identical` clusters** — the engine verified byte-identical occurrences; extract or consolidate them unless their surrounding ownership makes the duplication intentional.
6. **For `NearlyIdentical` clusters** — read the elected pair and every occurrence before refactoring. The pair passed admission, but its differences may still be meaningful; decide whether to unify them through a parameter or strategy, or keep them intentionally divergent.
7. **Ignore clusters where `weight` is low and `size` is 2 and node counts are tiny.** Those are usually boilerplate (constructors, test setup, property accessors) that don't reward extraction.

## Things to keep in mind when interpreting a report

- **Type-4 detection may be disabled.** If `embedding_cos` is `0.00` across the entire report, the semantic-embedding pass was not run — semantically equivalent but syntactically different code (iterative vs recursive, LINQ vs foreach) will not appear.
- **Overlapping byte ranges** are expected. The sibling-extension pass emits fingerprints for nested windows of the same physical code. Same cluster id → same clone; different cluster ids over the same bytes → different granularities of the same match.
- **Only supported languages are analyzed.** Files in languages the tool does not support are skipped silently. The report header lists which languages were active for this run.

## Tool metadata

- Tool: `deslop`. The report header states the tool version.
- The text report is a pretty-printer over the canonical JSON schema. For machine consumption prefer `--format json`, which includes full `occurrences[]` arrays, per-cluster `signals { structural, token_jaccard, shape, embedding_cos, agreement, rename_consistency, literal_fraction }`, and an agent-oriented `summary` string per cluster.
- **Every figure is stated, not implied.** Rank, bucket, occurrence count, language, the shape reading and the evidence sentence are all carried on the cluster because they are the engine's answers. Recomputing one from the other fields is how a consumer ends up disagreeing with the report it is quoting.
