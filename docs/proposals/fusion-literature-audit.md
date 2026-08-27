# Fusion audit against the reading list

Audits every place Deslop combines two numbers into one against the papers in [reading-list.md](../specs/reading-list.md). Scope is arithmetic only — how scores fuse, how thresholds were chosen, and whether the citation behind each number says what we claim it says.

Verified against the sources on 2026-08-27. Two papers were read directly rather than quoted from our own notes: [Selecting and Combining LLMs for Scalable Code Clone Detection (arXiv 2510.15480)](https://arxiv.org/abs/2510.15480) and the SSCD preprint [arXiv 2309.02182](https://arxiv.org/abs/2309.02182).

## The advice we are being measured against

- **Ensemble by max or sum, never average** — arXiv 2510.15480 evaluated 76 models and reports "score normalization and favoring ensembling methods like maximum or sum over averaging". Best ensemble 46.91% precision vs 39.71% for the best single model. Note the two halves: max/sum **and** normalization first.
- **Weak evidence classes are less reliable** — Svajlenko & Roy 2015 (precision and recall degrade monotonically as syntactic similarity falls); Kapser & Godfrey 2008 (shape-level cloning is frequently deliberate and benign); Bellon et al. 2007 (per-type precision differs).
- **Token similarity operating point ≈ 0.7** — SourcererCC, using **overlap similarity** (shared tokens ÷ tokens in the larger fragment), not Jaccard.
- **Semantic recall comes from ANN over embeddings** — SSCD: a cosine threshold plus a topN cut on the ranked neighbour list.
- **MinHash beats SimHash for Jaccard-shaped problems** — Shrivastava & Li, PMLR.
- **Unweighted duplicated-line density is the comparable CI gate** — SonarQube metric definitions.

## Where we agree

| Claim | Where | Verdict |
|---|---|---|
| Pair fusion is the strongest single axis | `pair.rs:151` `bounded_fused` = `max(structural, token_jaccard, embedding_cos)`, non-finite axes dropped, clamped | Exactly the paper's advice. `[FUSION-STRATEGY-BOUNDED-MAX]` says it in words too: "Never their sum, never their average." |
| Dropping the sum arm (#343) | `issue_343_sum_clamp_saturation.rs` | The paper permits sum; we removed it because our axes are unit-bounded and correlated, so a sum clamps mid-band clusters to 1.0 that no axis earned. Stricter than the literature, and right for our regime. |
| Content gate | `buckets/gate.rs:189` — `fused = max(embedding_cos, shape × content_confidence)` | Max throughout. The one multiplication only ever lowers a saturated shape match; it cannot manufacture confidence. |
| Shape and content readings | `report.rs:157` `max(structural, token_jaccard)`; `gate.rs:90` `max(agreement, rename_consistency)` | Max, single definition, no re-derivation downstream. |
| Weighted duplication metric | `pipeline.md` `[METRICS-REPO-WEIGHTED]` — line weight is `max` over covering occurrences | Max, never sum, so overlapping clusters can't push a line past 1.0. Weights track evidence class per Svajlenko/Kapser/Bellon. |
| Mechanical percentage stays unweighted and stays the default gate | `pipeline.md:441` | Matches the SonarQube precedent for comparability. |
| Candidate set is a union of three passes | `[FUSION-SIGNALS-THREE-LAYER]` | Hybrid, not pure vector search — what every surveyed system does. |
| MinHash for the token axis | `lsh.rs` | Correct primitive for a Jaccard-shaped problem. |
| Baxter's `2S / (2S + L + R)` | `refactor/merge/gate.rs:29` | Used where Baxter used it — mergeability, not confidence. |
| Zhang–Shasha TED for graded structural overlap | `overlap.rs:9` — `1 − TED / max(nodes)` | Baxter's own near-miss extension is tree edit distance. |
| No fusion arithmetic outside Rust | swept the TypeScript surfaces | Clean. |

## Where we depart

Ordered by how much accuracy is at stake.

### 1. We average within each axis, across occurrence pairs

`cluster/signals.rs:100` renders each cluster signal as the **mean over every unordered pair of rendered occurrences**. `[FUSION-CLUSTER-SIGNALS]` bans averaging *discovery edges* and then mandates averaging *rendered pairs*.

This is not the averaging the paper prohibits — that one is across different scores, and we take the max there. But the argument for max applies at this level too: a cluster holding one byte-identical pair and one weak member renders a middling `structural`, and that number then multiplies the ranking weight at `report_weight.rs:141`. The best evidence in the cluster is invisible in the rendered triple, and a real duplicate ranks lower than it should.

The stated reason for the mean is honesty about the whole rendered set, which is a defensible position — but it is a house position, not a cited one, and no assertion pins the dilution it accepts.

### 2. Max without normalization

The paper's finding is "score normalization **and** favouring maximum or sum". We do the second half. Our three axes are all in [0, 1] but are not calibrated against each other: cosine 0.85 from a local embedding model, a MinHash Jaccard estimate of 0.85, and a tree alignment of 0.85 are not the same weight of evidence. Under max, the most generous axis wins by construction.

Compensations exist and are deliberate — `fused_threshold` raised to 0.85 rather than the literature's 0.7, the content gate, the compound admission gate in `[FUSION-SHARED-SUBTREE]`, `[PAIR-SIZE-COHERENCE]`. But nothing measures cross-axis comparability and no test asserts it.

Worth keeping in view: the ensemble in the paper reached **46.91% precision**. Max fusion is a recall-first move, measured in a precision-poor regime. It buys recall and it costs precision, and our threshold is the only thing paying that bill.

### 3. `candidates.embedding_min_cosine = 0.80` is cited as SSCD's operating point. It isn't.

`[FUSION-TUNING-LEVERS]` labels this **Literature — "SSCD's published operating point"**. SSCD's tabulated experimental settings are `similarity 0 / 0.95` with `topN 0 / 1 / 100`; the reported runs use 0.95 (BCB) or no similarity limit at all. 0.80 appears nowhere in the paper. `candidates.embedding_support_floor = 0.80` inherits the same unsupported provenance by derivation, and it is the line at which `[CLONE-BUCKETS-ROUTING]` row 2 lets semantic evidence carry a bucket alone.

The value may still be right. The citation is not.

### 4. `candidates.embedding_top_k = 5` is below every published topN

Marked **Unrecorded**, and the spec already concedes the rationale argues for "small, not for five". SSCD used topN 100 on BCB (large clone classes) and up to 10 on industrial data, and explicitly ties the value to how large clone classes are in the corpus. Five is below both. This is the axis that exists to find Type-3/4, so the cost of being wrong here is silent recall loss.

### 5. SourcererCC's 0.7 is overlap similarity, not Jaccard

Two of our provenance entries transfer that number to different quantities:

- `admission.fused_threshold` — "[TECH-TOKEN-SOURCERERCC] treats Jaccard ≥ 0.7 as the typical Type-3 cutoff". SourcererCC's similarity is shared tokens ÷ larger fragment. Overlap ≥ Jaccard for every pair, so we are stricter than the paper by an unmeasured margin, in the false-negative direction.
- `content_gate.support_floor` — cited as "[TECH-TOKEN-SOURCERERCC] Type-3 overlap cutoff", but applied to **raw-byte agreement**, a third quantity that no token study measured.

Our own `landscape.md:16` gets it right ("bag-of-tokens + inverted index + **overlap filter**"). `fusion.md` and `landscape.md` disagree with each other.

### 6. Four ranking levers have no provenance at all

`ranking.type4_embedding_floor` (0.90), `ranking.low_structural_type4_ceiling` (0.10), `ranking.low_structural_type4_weight` (1/10), `routing.proven_identical_token_floor` (0.99) — all **Unrecorded**, plus the table of unnamed inline literals in `buckets.rs` and `report_render.rs`. The spec's own rule: unrecorded is "a tracked gap, not a resting state".

### 7. `rename_consistency_discount = 0.9` is a house rule

Derived from wanting the answer to land above `fused_threshold` while reserving 1.0 for byte proof. Baker's p-match theory treats a corroborated bijective rename as *proven* Type-2 duplication, and Type-2 is the band where benchmark precision is highest. A 10% haircut on proven evidence is a presentation choice; nothing in the literature asks for it. Low risk — worth labelling accurately rather than changing.

## Spec and code disagree — reported, not fixed

Both concern the ranking formula. Per the documentation rule these are reported and left alone.

1. **Bytes vs LOC.** `pipeline.md:207` specifies `weight = clone_node_count × (cluster_size − 1) × log2(1 + total_spanned_loc)`. `cluster.rs:444` computes it over `spanned_bytes`. Bytes and lines order clusters differently.
2. **The visible re-rank drops a whole term.** The same paragraph says visible ordering uses that formula with non-hidden occupancy. `report_weight.rs:160` computes `nodes × (visible − 1)` — no log term — then multiplies by category, structural-only, and fused confidence. The rendered ranking users actually see is not the specified formula.

## What I'd do next, in order

1. **Test first for the pair mean.** Fixture: a cluster with one byte-identical pair and one weak member. Assert the rendered `structural`, the bucket, and the rank position. Watch it fail under the mean, then decide max vs mean with the number in front of you.
2. **Fix the two spec/code mismatches** — decide which side is the truth, change the other.
3. **Re-label provenance** for `embedding_min_cosine`, `embedding_support_floor`, `fused_threshold`, and `content_gate.support_floor`. Defect or derived, not literature.
4. **Sweep `embedding_top_k`** against a corpus with large clone classes before it stays at 5.
5. **Say out loud, in `[FUSION-STRATEGY-BOUNDED-MAX]`, that we take the max of uncalibrated axes** and name what pays for it. Right now the spec argues the max is conservative; it is conservative about *manufacturing* confidence, and generous about *admitting* a pair.
