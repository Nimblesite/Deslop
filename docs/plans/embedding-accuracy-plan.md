# Embedding accuracy plan

Deferred from the `fused` branch on 2026-08-14. Everything here is diagnosed with measured evidence and pinned by committed red tests; nothing here blocks the branch, and none of the pinned assertions may be weakened while executing it.

## The red tests this plan turns green

All three are `#[ignore]`d under issue #369 so the branch can merge. **No assertion was deleted, skipped, or softened** — each still runs under `cargo test -- --ignored`, and un-ignoring them is the acceptance criterion for this plan.

- `deslop::pair_size_coherence::an_embedding_only_pair_does_not_join_occurrences_of_different_size` — two-ledger scan must report exactly the one real family.
- `deslop::issue_343_sum_clamp_saturation::mid_band_cluster_confidence_never_exceeds_its_strongest_axis` — fails `cluster_count 2 != 1` on the same fixture.
- `deslop-lsp::lsp_embedding_determinism::lsp_embedding_refresh_is_bounded_and_reproducible` — "fixture lost the second correlated signal".

Measured while diagnosing #369: the two-ledger scan renders **two clusters, both false positives** — `45986e47bfc430a2` (whole `ledger_a` vs an arithmetic chunk of `ledger_c`) and `e021161df1cf4142` (two parameter lists plus an arithmetic chunk). Both carry `structural = 0` *and* `token_jaccard = 0`, surviving on the mock's cosine alone, which is what §3 and §4 below address. The real 380-node clone is not among them.

§4 is further along than the text below suggests: replacing `embed_vector` with the L2-normalised indicator of a snippet's distinct 5-character shingles turns all three tests green (verified locally, 6/6 across both `deslop` binaries; real pair 0.966, whole-fn vs chunk 0.803, params vs chunk 0.085, identical text exactly 1.0). It is not landable as written because `exact_embedding_pairs` is `O(N² · D)` — at 4096 lanes `cli::embedding_ollama::issue_286_large_subtree_survives_when_the_model_declares_the_context` ran past ten minutes. A signed-lane signature holds the separations at 128 lanes (0.954 / 0.782 / 0.107 / 1.0); that is the width to time first, keeping `CosinePoint`'s per-point norm precomputation rather than recomputing norms inside the pair loop.

All three share the `ts-mixed-band` fixture. They were green before the `[PIPELINE-NORMALIZE-AST]` root-span correction only because the mock embedder's `sin(len)` cosine manufactured a visible cluster that papered over a recall hole; the correction moved byte lengths and the paper-over vanished. The defects below were always there.

## Fixes, in dependency order

### 1. [FUSION-SIGNALS-THREE-LAYER] Token similarity understates Type-3 clones (issue #367) — the root cause

Measured 2026-08-14 by instrumenting `survival_decision`. Minimal repro, **no fixture and no embedder required**: two 1235-byte TypeScript files, identical except a consistent rename and **one added pair of parentheses** (one node in ~380), scan to `Found 0 groups of duplicated code`. Remove the parentheses and the same pair is found correctly (`nearly_identical`, 266/268 pairs survive). The dropped whole-function pair measures:

```
structural=0.0  token_jaccard=0.6640625  embedding_cos=0.0  nodes=(380, 381)
```

Two functions 99.7% identical by node count score `token_jaccard = 0.664`. `structural` is 0 because the added `parenthesized_expression` changes the Merkle hash of every ancestor to the root; with embeddings off — the default — `bounded_fused = 0.664 < FUSED_THRESHOLD` and the pair dies before any cluster exists. Nothing downstream can recover it.

Mechanism: MinHash estimates Jaccard over the *set* of distinct k-grams. Repetitive bodies have a small distinct-shingle set, so one added node displaces a large fraction of it. This is a property of the token signature, not of `ts-mixed-band`, and it makes every shape-changing Type-3 clone invisible on the default path.

**This is not a branch regression** — verified, not assumed: only one axis is non-zero, so the `fused() = Σ axes` → `bounded_fused() = max axes` change ([FUSION-STRATEGY-BOUNDED-MAX]) is irrelevant here (`sum == max == 0.664`) and `main` drops the pair identically.

Fix direction (needs design, not a threshold bump): the signature should be robust to small local insertions — e.g. multiset/weighted-Jaccard rather than set-Jaccard, or shingling that does not let one node displace a large share of a small distinct set. Any candidate must be validated against the corpus for *both* directions: recall on shape-changing Type-3 clones, and no new false positives on repetitive scaffolding. **Do not raise recall by lowering `FUSED_THRESHOLD`** — that admits every weak pair in the corpus.

New E2E to write first (red), independent of embeddings: seed the two-file paren pair above, scan with `--min-nodes 100`, assert 1 visible cluster spanning both files with 2 occurrences each covering the whole function (`start_byte ≤ 9`, `end_byte ≥ 1200`), bucket in the act-now set, and `clusters_hidden == 0`.

*Superseded:* an earlier revision of this plan blamed `cluster/subsume.rs` precision inversion. The trace disproves it — all four clusters that form are `structural = 1.0` sibling windows and the enclosing clone is never a cluster at all. Subsumption among equal-structural windows may still deserve review, but it is not this defect.

### 2. [RANK-CATEGORY] Un-gate the LSH Type-3 promotion from C# (issue #359, "ts-mixed-band recall")

`crates/deslop-core/src/report_render.rs::is_csharp_lsh_type3_near_miss`. The evidence profile it promotes (structural ≈ 0, cos ≈ 0, `token_jaccard ≥ LSH_ONLY_MIN_JACCARD`, every member ≥ `LSH_ONLY_MIN_NODE_COUNT`, cross-file) is language-agnostic, but the predicate requires `language == "csharp"`, so an identical TypeScript pair routes `LooselySimilar` → hidden. Remove the language test, rename accordingly, and sweep the corpus fixtures for unhidden noise — any new visible cluster must be adjudicated as genuine or get its own filter with its own fixture, never a threshold bump.

Note this cannot fix §1 on its own: that path also requires `token_jaccard ≥ LSH_ONLY_MIN_JACCARD` (0.90), and the measured value is 0.664. §1 must land first for §2 to reach anything.

### 3. [PAIR-SIZE-COHERENCE] adjacent: corroboration floors for unanchored pairs (issue #365)

`crates/deslop-core/src/pair.rs::survival_decision` applies the LSH-only floors only when `structural <= 0 && embedding_cos <= 0`, so any ε of cosine waives both the 0.90 Jaccard floor and the 40-node floor — weaker corroboration promotes a pair. Fix without new magic numbers: for every `structural <= 0` pair require the node floor (`lsh_only_node_floor`, which already carries the cross-language opt-in), and waive the Jaccard floor only when `embedding_cos >= fused_min_score` — i.e. when the embedding axis independently clears the survival bar. The 18-node garbage edge in cluster `e021161df1cf4142` dies on the node floor regardless of cosine.

### 4. [FUSION-EMBED-PROVIDER] Content-sensitive mock embedder (issue #366)

`crates/deslop/tests/cli/mock_ollama.rs::embed_vector` returns `[sin(len), cos(first_byte), 0.5, -0.5]`: two constant lanes give every pair a high cosine floor and `sin` aliases over length — a 67-byte and an 865-byte text score 0.99997. ~88 `embedding_cos` assertions across 15 files currently calibrate against this noise, and the 379-node `45986e47bfc430a2` cluster (whole `ledger_a` vs `ledger_c`'s bare arithmetic chain, cos 0.9513) is manufactured by it — no gate can kill that profile without also blinding real Type-4 detection.

Fix: replace with a deterministic content-similarity vector (e.g. hashed byte-n-gram frequency lanes) such that renamed near-clones land high, the genuine `ts-mixed-band` rename family stays inside the `(0.80..=0.99)` mid-band the tests assert, and unrelated code lands low. Then re-run every mock-dependent suite; each moved bound is recalibrated to the honest instrument with equal or stronger discriminating power — no assertion may be deleted or loosened against the same instrument.

### 5. Re-verify the chain

Order matters: 1 is the root cause and must land first — without it §2 has nothing to promote; 2 then lets the recovered pair reach a real bucket in every language; 3 stops the mock garbage surviving on ε-cosine; 4 stops the garbage existing at all. After each step: the three red tests, then the full workspace sweep (`cargo test --workspace --all-targets --features deslop-core/live`), then the self-scan duplication gate.

## Also carried

- `deslop-lsp::embedding_failure_progress` hangs (pre-existing; revert-proved past 300 s) — needs its own root-cause pass.
- Hidden clusters are absent from the JSON wire entirely (`clusters: [], clusters_hidden: 1`) — an AI consumer cannot reconstruct the human view or audit the hide reasons; consider putting hidden clusters with their drop reason on the wire (#344 adjacent).
- `report.rs::log_hidden_cluster` now logs the signal triple and content evidence for every hidden cluster (landed on `fused`) — keep using it to audit hides.
