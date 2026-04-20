# CodeDedup Report Context

## What this is

You are being given a code-duplication report from **CodeDedup**, a static analysis tool that detects duplicated code using a hybrid pipeline of AST fingerprinting + token MinHash/LSH. The report lists **worst offenders first** — clusters ranked by a duplication-impact score, not just count.

## How it works (so you know what the signals mean)

The pipeline normalizes each source file's AST (collapsing identifier names, literals, and comments so renamed-identifier clones hash the same), then finds clones through three complementary passes:

1. **Structural (Merkle AST hash)** — bottom-up `blake3` hash of every AST subtree ≥ `min-nodes`. Detects Type-1 (identical) and Type-2 (renamed-identifier) clones. `structural=1.0` means "these subtrees are identical after normalization."
2. **Sibling-extension** — contiguous sibling windows under a common parent are hashed as groups. Catches near-miss clones where the same sequence of statements appears in different enclosing contexts.
3. **Token LSH (MinHash over k=5 k-grams of normalized node kinds)** — catches Type-3 clones where structure diverged but the token bag is close. `token_jaccard` is the estimated Jaccard similarity in `[0.0, 1.0]`.

Candidate pairs from all three passes are unioned, then transitively closed into clusters. Small LSH-only pairs are filtered out (they're usually trivial scaffolding like imports or namespace declarations).

## Clone taxonomy

- **Type-1** — identical code (ignoring whitespace/comments).
- **Type-2** — identical up to renaming of identifiers/literals.
- **Type-3** — Type-2 + added/removed/modified statements ("near-miss").
- **Type-4** — semantically equivalent, syntactically different. Requires an embedding pass; may or may not be enabled for the report you are reading. When disabled, `embedding_cos` is always `0.00`.

## How to read the report format

Each line starts with `#N` (rank, lower = worse) and looks like:

```
#1 [abc123def4567890] weight=271716.65 size=20 nodes=825
  20 copies of a 825-node subtree at path/A.cs:230-5843, path/B.cs:195-5843, path/C.cs:0-5844 (+17 more) [structural=0.00, token_jaccard=0.97, embedding_cos=0.00]
```

Fields:

| Field | Meaning |
|---|---|
| `#N` | Rank. `#1` is the worst offender. |
| `[abc123…]` | Stable 16-char cluster id (hex). Same clone across runs → same id. |
| `weight` | Ranking score: `node_count × (cluster_size − 1) × log2(1 + total_spanned_bytes)`. Higher = more impact. |
| `size` | How many copies of this subtree exist. `size=20` means the same pattern appears 20 times. |
| `nodes` | AST node count of one canonical copy. Bigger subtree = more meaningful clone. |
| `path:start-end` | **Byte offsets** (NOT line numbers), half-open `[start, end)`. First 3 occurrences shown; `(+N more)` means there are more. |
| `structural` | `[0, 1]`. `1.0` = exact Merkle hash match. `0.0` = no exact structural match (found via LSH only). |
| `token_jaccard` | `[0, 1]`. Estimated Jaccard of normalized k-gram token sets. |
| `embedding_cos` | `[0, 1]` cosine similarity from the semantic-embedding pass, or `0.00` if that pass was disabled for this run. |

Byte ranges come from `tree-sitter`. To display line numbers you must re-derive them from the source file, because byte offsets are the canonical location (they're what an LSP/editor consumes directly).

## Reading the signals together

| `structural` | `token_jaccard` | What it means |
|---|---|---|
| `1.00` | `1.00` | **Type-1 or Type-2 exact clone.** Safe candidate for extraction into a shared function/method. |
| `1.00` | `<1.00` | Unusual — same AST shape but slightly different token k-grams. Usually means overlapping byte ranges from sibling-extension. |
| `0.00` | `≥ 0.90` | **Type-3 near-miss.** Similar token content, different structure. Review before merging — may differ in a semantically important way (loop vs recursion, added guard, etc.). |
| `0.00` | `0.70 – 0.90` | Weak signal. Likely rejected; if present, endpoints were substantial (≥ 40 nodes). Treat as a hint, not a directive. |
| `0.20 – 0.80` | `≥ 0.95` | **Fused cluster spanning multiple exact-clone bands.** Transitive closure merged several smaller exact clusters via near-miss links. Usually genuine duplication across a family of variants. |

## Repo-wide duplication metric

The report header carries one honest number: `metrics.duplication_percent = 100 × duplicated_loc / analysed_loc`.

- `duplicated_loc` = lines covered by ≥ 2 non-hidden clone occurrences, deduplicated per file so overlapping sibling-extension ranges count once.
- `analysed_loc` = physical lines across every file in `files_analysed`.
- Hidden occurrences (generated code flagged via `.codededup.toml` `report_hide`) are excluded so they cannot inflate the metric.
- CI gating: `--fail-over <percent>` (or `[threshold] max_duplication_percent` in `.codededup.toml`) exits `3` when `duplication_percent > threshold`. No threshold → no gate. Use this, not the `weight` column, for pass/fail decisions.

## Thresholds (typical defaults)

- `min-nodes = 15` — smaller subtrees are excluded to cut noise. The header of the report will state the value actually used.
- `FUSED_THRESHOLD = 0.85` — a pair must score ≥ this on the fused signal (combination of `structural`, `token_jaccard`, `embedding_cos`) to enter a cluster.
- `LSH_ONLY_MIN_JACCARD = 0.90` and `LSH_ONLY_MIN_NODE_COUNT = 40` — extra gates for LSH-only candidates (no structural anchor), to keep tiny trivial windows from mega-clustering.

## What to do with this report

1. **Start at #1 and work down.** The weight formula already prioritises by impact.
2. **Check if it's generated code.** Generated files (e.g. `.g.cs`, `.generated.cs`, OpenAPI clients, protobuf output) are expected to duplicate by design — usually not worth refactoring the generator unless a pattern emerges across many generators.
3. **Check byte ranges for overlap.** Adjacent/overlapping ranges in the same file mean the sibling-extension pass is firing on several enclosing contexts of the same physical code — count it as one logical clone, not N.
4. **For `structural=1.00` clusters** — safe to extract. Identical subtree after normalization.
5. **For `structural=0.00, token_jaccard≥0.95` clusters** — Type-3 candidate. Read both occurrences. The differences are meaningful. Decide whether to (a) unify via a parameter / strategy, or (b) accept them as intentionally divergent.
6. **Ignore clusters where `weight` is low and `size` is 2 and node counts are tiny.** Those are usually boilerplate (constructors, test setup, property accessors) that don't reward extraction.

## Things to keep in mind when interpreting a report

- **Type-4 detection may be disabled.** If `embedding_cos` is `0.00` across the entire report, the semantic-embedding pass was not run — semantically equivalent but syntactically different code (iterative vs recursive, LINQ vs foreach) will not appear.
- **Overlapping byte ranges** are expected. The sibling-extension pass emits fingerprints for nested windows of the same physical code. Same cluster id → same clone; different cluster ids over the same bytes → different granularities of the same match.
- **Only supported languages are analyzed.** Files in languages the tool does not support are skipped silently. The report header lists which languages were active for this run.

## Tool metadata

- Tool: `codededup`. The report header states the tool version and report schema version.
- The text report is a pretty-printer over the canonical JSON schema. For machine consumption prefer `--format json`, which includes full `occurrences[]` arrays, per-cluster `signals { structural, token_jaccard, embedding_cos, fused }`, and an agent-oriented `summary` string per cluster.
