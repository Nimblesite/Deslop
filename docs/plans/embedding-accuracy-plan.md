# Embedding accuracy plan

Deferred from the `fused` branch on 2026-08-14. Everything here is diagnosed with measured evidence and pinned by committed red tests; nothing here blocks the branch, and none of the pinned assertions may be weakened while executing it.

## The red tests this plan turns green

- `deslop::pair_size_coherence::an_embedding_only_pair_does_not_join_occurrences_of_different_size` — two-ledger scan must report exactly the one real family.
- `deslop::issue_343_sum_clamp_saturation::mid_band_cluster_confidence_never_exceeds_its_strongest_axis` — fails `cluster_count 2 != 1` on the same fixture.
- `deslop-lsp::lsp_embedding_determinism::lsp_embedding_refresh_is_bounded_and_reproducible` — "fixture lost the second correlated signal".

All three share the `ts-mixed-band` fixture. They were green before the `[PIPELINE-NORMALIZE-AST]` root-span correction only because the mock embedder's `sin(len)` cosine manufactured a visible cluster that papered over a recall hole; the correction moved byte lengths and the paper-over vanished. The defects below were always there.

## Fixes, in dependency order

### 1. [PIPELINE-CLUSTER-SUBSUME] Enclosure must decide against a nested view (issue #367)

`crates/deslop-core/src/cluster/subsume.rs`. A nested sibling window scores `structural = 1.0` by construction (normalisation collapsed the very leaves that differ), while the enclosing near-verbatim clone scores `structural = 0.0` because it is a real Type-3. `evaluate_pair` routes the strictly-enclosed case through `precision_preference`, so the 19-node name+params window discards the 380-node whole-function clone (`drop_inner survivor=7471d78aa8529cc6`).

Fix: when one view strictly encloses the other, enclosure decides — precision applies only to crossed and identical occurrence sets. Keep the file-coverage guard and the embedding-dominant nomination for the no-nesting case.

### 2. [RANK-CATEGORY] Un-gate the LSH Type-3 promotion from C# (issue #359, "ts-mixed-band recall")

`crates/deslop-core/src/report_render.rs::is_csharp_lsh_type3_near_miss`. The evidence profile it promotes (structural ≈ 0, cos ≈ 0, `token_jaccard ≥ LSH_ONLY_MIN_JACCARD`, every member ≥ `LSH_ONLY_MIN_NODE_COUNT`, cross-file) is language-agnostic, but the predicate requires `language == "csharp"`, so an identical TypeScript pair routes `LooselySimilar` → hidden. Remove the language test, rename accordingly, and sweep the corpus fixtures for unhidden noise — any new visible cluster must be adjudicated as genuine or get its own filter with its own fixture, never a threshold bump.

New E2E to write first (red): scan `ts-mixed-band/ledger_a.ts` + `ledger_c.ts`, embeddings off, `--min-nodes 12` — assert 1 visible cluster spanning both files, 2 occurrences, each occurrence `start_byte ≤ 9` and `end_byte ≥ 1200` (the whole function, not the 71-byte signature window), bucket `nearly_identical`, `token_jaccard ≥ 0.9`, `clusters_hidden == 0`. Today this scans to `clusters: []`.

### 3. [PAIR-SIZE-COHERENCE] adjacent: corroboration floors for unanchored pairs (issue #365)

`crates/deslop-core/src/pair.rs::survival_decision` applies the LSH-only floors only when `structural <= 0 && embedding_cos <= 0`, so any ε of cosine waives both the 0.90 Jaccard floor and the 40-node floor — weaker corroboration promotes a pair. Fix without new magic numbers: for every `structural <= 0` pair require the node floor (`lsh_only_node_floor`, which already carries the cross-language opt-in), and waive the Jaccard floor only when `embedding_cos >= fused_min_score` — i.e. when the embedding axis independently clears the survival bar. The 18-node garbage edge in cluster `e021161df1cf4142` dies on the node floor regardless of cosine.

### 4. [FUSION-EMBED-PROVIDER] Content-sensitive mock embedder (issue #366)

`crates/deslop/tests/cli/mock_ollama.rs::embed_vector` returns `[sin(len), cos(first_byte), 0.5, -0.5]`: two constant lanes give every pair a high cosine floor and `sin` aliases over length — a 67-byte and an 865-byte text score 0.99997. ~88 `embedding_cos` assertions across 15 files currently calibrate against this noise, and the 379-node `45986e47bfc430a2` cluster (whole `ledger_a` vs `ledger_c`'s bare arithmetic chain, cos 0.9513) is manufactured by it — no gate can kill that profile without also blinding real Type-4 detection.

Fix: replace with a deterministic content-similarity vector (e.g. hashed byte-n-gram frequency lanes) such that renamed near-clones land high, the genuine `ts-mixed-band` rename family stays inside the `(0.80..=0.99)` mid-band the tests assert, and unrelated code lands low. Then re-run every mock-dependent suite; each moved bound is recalibrated to the honest instrument with equal or stronger discriminating power — no assertion may be deleted or loosened against the same instrument.

### 5. Re-verify the chain

Order matters: 1–2 make the true family visible without embeddings; 3 stops the mock garbage surviving on ε-cosine; 4 stops the garbage existing at all. After each step: the three red tests, then the full workspace sweep (`cargo test --workspace --all-targets --features deslop-core/live`), then the self-scan duplication gate.

## Also carried

- `deslop-lsp::embedding_failure_progress` hangs (pre-existing; revert-proved past 300 s) — needs its own root-cause pass.
- Hidden clusters are absent from the JSON wire entirely (`clusters: [], clusters_hidden: 1`) — an AI consumer cannot reconstruct the human view or audit the hide reasons; consider putting hidden clusters with their drop reason on the wire (#344 adjacent).
- `report.rs::log_hidden_cluster` now logs the signal triple and content evidence for every hidden cluster (landed on `fused`) — keep using it to audit hides.
