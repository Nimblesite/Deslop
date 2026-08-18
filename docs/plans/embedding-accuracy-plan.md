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

## Measured 18 Aug — §4 has landed, and the blocker moved

`MockOllama::embed_vector` is now the 128-lane signed 5-byte-shingle signature §4 prescribes (commit
31d5efd18). It works: `issue_343` no longer renders the two embedding-only false positives, and the real
`settleLedger`/`settleQuarter` pair is a visible `same_behavior` cluster at `embedding_cos = 0.979`, so
`cluster_count == 1` now passes.

**The remaining blocker is `clusters_hidden == 1`, not the cosine.** The extra cluster is the two function
*signatures*: `structural = 1.00, token_jaccard = 1.00, agreement = 0.80` — four of five collapsed leaves
agree, only `settleLedger` vs `settleQuarter` differs — `rename_consistency = 0.50`. It is admitted as a
candidate at `--min-nodes 12` and then suppressed at render by `cluster_filters::is_signature_only_cluster`,
which is the right verdict for a bare signature. So the assertion is not asking for a different bucket; it is
asking that a bare signature never become a candidate. **Fix it at admission, never by relaxing the
assertion.**

Two changes landed alongside §4 that it did not prescribe and that nothing pins:
`exact_embedding_pairs` and `EXACT_PAIR_LIMIT` were deleted from `embedding/pairs.rs`, leaving ANN
`TOP_K = 5` as the only pair source. Both this plan and `rename-recall-plan.md` say to *shrink the width to
128 so the exact pass is affordable* — "that is the width to time first" — not to remove it. The deleted doc
named the guard: "small fixture and edited-file runs can have many near-tied subtree embeddings; exact
scoring prevents top-k neighbour truncation from dropping the only declaration-level Type-4 pair."
Restoring it does **not** change `issue_343`'s outcome (measured), so it is not the blocker — but the guard
it removed is now unpinned, and small-corpus recall needs a test either way.

## 🔴 The release gate is red — a real Type-4 clone is missed ([#407](https://github.com/Nimblesite/Deslop/issues/407))

`dart_issue_119_embedding_role_mismatch::dart_same_role_function_pair_still_surfaces` is red and is **not**
`#[ignore]`d, so `make ci` fails. It is red for a true reason and must not be weakened.

The fixture is a textbook Type-4 semantic clone — `totalRecursive` vs `totalIterative`, both summing
`1..limit`, different code and identical behaviour. It no longer surfaces: no embedding pair forms, so no
cluster is built. It was never really detected; the pre-#369 mock's two constant lanes floored every cosine
near 1.0, so it only looked detected. Making the instrument honest made the recall hole visible.

Measured against **live Ollama**, so this is production behaviour and not a harness artifact
(`MIN_COSINE = 0.80`, `embedding/pairs.rs:21`): same-role **0.7763**, role-mismatch 0.6101, same-role with a
shared docstring 0.8311. Production misses the clone by 0.037 of cosine.

Do **not** lower `MIN_COSINE` to make it green — widening a threshold to pass an assertion is prohibited, and
0.80 was never chosen against this evidence.

The deeper blocker: `MockOllama` structurally cannot express a Type-4 clone. Its signature is lexical, so a
pair the real embedder scores 0.8311 the mock scores 0.5727 and drops, while any pair lexically close enough
to clear 0.80 in the mock is near-verbatim and routes `identical`/`nearly_identical`, never `same_behavior`.
No trustworthy `same_behavior` assertion can be driven through it. Either those fixtures move behind the
real-Ollama gate (renamed `ollama_*`), or the mock gains a deterministic semantic mode. That is a
test-semantics decision, not a code fix.

Consequence for **#358**: its premise — "the Python role gate over-suppresses" — is falsified. Measured on
`python-issue-119-same-role`: `embedding pass complete pair_count=0`, `candidate_pairs=0`,
`visible=0 hidden=0`. Nothing was suppressed because nothing was ever formed; the gate never runs. A probe
fixture with enough overlap to clear 0.80 produced `visible=1 hidden=0 same_behavior=1` — the gate is
healthy. #358 needs retitling; the work is this harness problem.

## The other embeddings-on ignores

Moved here from `fused-score-followups.md`, where they did not belong — these are recall defects, not
fused-scoring defects.

- **#356** — two ignores in `embedding_route_invariance`, the blast-radius pins for `[REPAIR-COSINE-MERGE]`.
  `csharp-type3` publishes two `structural_only` clusters at `structural 1.0` with embeddings off and **one**
  `same_behavior` cluster with them on; `ts-mixed-band` publishes a four-file `nearly_identical` cluster off
  and **zero** clusters on. Restored cosines are changing cluster *membership* through the closure. A bucket
  must be a function of a cluster's occurrences, never of which pass reached them
  ([FUSION-CLUSTER-SIGNALS]).
- ✅ **#357 — landed.** `EmbeddingBatch::push` emitted one ANN point per *fingerprint*, so the HNSW was built
  over N identical points and `indexed_subtrees` reported the occurrence count. It now emits one point per
  distinct snippet carrying all its owners, and `vectors_by_fingerprint` fans the vector back to every owner
  so no pair loses its measured cosine (`[REPAIR-COSINE-MERGE]`). Measured on a 300-statement C# corpus
  against real `nomic-embed-text`: **305 s → 17 s wall, 2597 s → 8 s CPU**, occurrence-for-occurrence
  identical to the embeddings-off baseline. `embedding_perf` un-ignored, 3 green.

  Adjudicated afterwards: the collapse put `issue_82_embedding_context_budget` in contradiction with
  `embedding_perf` — one asserted `attempted == indexed + failed`, the other requires `indexed < attempted`
  with no failures. `[REPORTING-CONTEXT]` settles it: `indexed_subtrees` is "the count of **unique**
  successful subtree embeddings … **lower than** `attempted_subtrees` when duplicate snippets collapse", so
  the old identity was the spec's negation and was only ever green because the code violated the spec. The
  three fields were also in two different units with nothing to reconcile them, which made
  `indexed/attempted` read as a coverage ratio — the HTML footer literally rendered "indexed 29/52", implying
  23 lost subtrees where none were lost. `EmbeddingProvenance` now carries a fourth field,
  `succeeded_subtrees` (occurrence-level successes), so `attempted = succeeded + failed` holds in one unit and
  `indexed <= succeeded` is checkable. The test gained two assertions rather than losing one; the footer now
  reads "embedded 52/52 subtrees via 29 index points".

- ~~**#357** — duplicate subtrees are not collapsed before ANN indexing (312 attempted / 312 indexed),~~
  `embedding_perf`. The collapse must expand pairs correctly so no pair loses its measured cosine (#351);
  `EmbeddingBatch::push` documents why every fingerprint sharing a snippet must receive the vector.
- **#358** — the Python role gate over-suppresses: a same-role, behaviour-equivalent function pair never
  surfaces, `python_issue_119`. Rule out a #356 ANN-bridge interaction before blaming the gate.

## Also carried

- ✅ `deslop-lsp::embedding_failure_progress` no longer hangs (#370, landed). The stall was never a missing
  terminal frame: the LSP test harness piped the child's stderr and held the read end open without ever
  reading it, so the pipe filled, the next `tracing` event blocked while holding the subscriber's stderr
  lock, and the `tower-lsp` serve loop queued behind it and stopped answering. `common::StderrDrain` now
  reads it to EOF on a background thread — 14m41s to 9.5s, and every LSP test loses the same latent
  deadlock. Un-ignoring it then exposed a live false negative: a refresh in which the provider rejected all
  851 subtrees was committed over the last good report and announced `phase = "complete", done = 851`. That
  code is quarantined under `[QUARANTINE-EMBED-REFRESH-COMPLETE]` in `live/api.rs` and the test is
  deliberately red. The terminal-phase rule is now stated in `[LIVE-EMBEDDING-CONSENT]`.
- Hidden clusters are absent from the JSON wire entirely (`clusters: [], clusters_hidden: 1`) — an AI consumer cannot reconstruct the human view or audit the hide reasons; consider putting hidden clusters with their drop reason on the wire (#344 adjacent).
- `report.rs::log_hidden_cluster` now logs the signal triple and content evidence for every hidden cluster (landed on `fused`) — keep using it to audit hides.
