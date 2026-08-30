# Fused signals, pair admission, and evidence reporting

Deslop combines structural, token, and embedding analysis. The surveyed systems in [landscape.md](landscape.md) and [reading-list.md](reading-list.md) likewise combine representations rather than relying on vector search alone.

### [FUSED-SIGNALS-THREE-LAYER] Deslop is hybrid by design

The pipeline fuses three signals:

1. **Structural (AST fingerprinting)** — Merkle-hash every tree-sitter subtree after normalization. Catches Type-1, Type-2, most Type-3. Fast, deterministic, gives exact byte ranges. (Chilowicz 2009 + Baxter 1998.)
2. **Token LSH (MinHash over normalized k-grams)** — catches Type-3 cases where structure diverged but token bag is close. Fast, deterministic. (SourcererCC 2016.)
3. **Learned embeddings (local, via Ollama)** — catches Type-3/Type-4 the structural passes miss. Used both as a **recall expander** (find candidates the hash-based passes didn't cluster) and as a **re-ranker** (promote semantically-similar AST clusters in the final score). (SSCD 2024, ensemble-LLM 2025.)

The structural and token layers always run. The embedding layer is opt-in: `--embeddings` defaults to `off`, because it needs a reachable local Ollama and the shipped CLI must produce a report on a machine that has none. `auto` uses embeddings when the provider answers and warns when it does not; `required` hard-fails instead. Leaving it off is a measurable recall loss on Type-3/4 — the research does not support it as a permanent posture, only as a default that never blocks a first run.

### [FUSED-EMBED-PROVIDER] Embedding layer — concrete choices

- **Runtime-selected provider and model.** `--embedding-provider` and `--embedding-model` select factories registered through `ProviderRegistry::production`; only `ollama` is registered today, and another provider needs no transport special case. The deterministic BLAKE3 test stub exists only behind `test-support`, is never production-registered, and is excluded from the VSIX by a packaging gate.
- **Defaults.** The provider is local `ollama`; the model is `nomic-embed-text` (`DEFAULT_OLLAMA_MODEL`). `nomic-embed-code` is the code-tuned alternative; CodeT5+110M and UniXCoder may be exposed through future providers. Hosted providers require deliberate user configuration.
- **ANN index.** Use HNSW through `usearch` or `instant-distance`; SSCD validated HNSW at 250 MLOC.
- **One cosine definition.** `embedding::cosine_similarity` accumulates dot product and norms in `f64` over raw `f32` components, performs no intermediate normalisation, and clamps to `[0,1]`. Byte-identical snippets share a vector and render exactly `1.0` (gh #372; `issue_372_identical_snippet_cosine.rs`); HNSW's `f32` distance discovers candidates but is never rendered ([FUSED-CLUSTER-SIGNALS]).
- **Ensemble by maximum.** Structural and token axes are correlated views of one normalised tree, so the fused score takes the strongest normalised axis, never a sum or average (gh #343).
- **Cache key.** `(file_content_hash, provider_id, model_id, model_version)` isolates embedding invalidation from structural and LSH caches and is reused by incremental mode.
- **Granularity.** Embed AST subtrees above the minimum-node threshold, not whole files, retaining byte ranges and reducing k-NN size.
- **Provider-owned input budget.** Oversized subtrees count in `failed_subtrees` and are not dispatched because Ollama silently truncates. `EmbeddingProvider::max_input_chars` therefore reflects the model; `OllamaProvider` reads `model_info["<arch>.context_length"]` from `POST /api/show`, uses a conservative three characters per token, and falls back to `DEFAULT_MAX_INPUT_CHARS` (6,000). This avoids the upstream fixed cap that dropped 14,723 of 175,160 subtrees in gh #286.
- **Approximate but reproducible discovery.** Record provider/model identity and version in cache and report metadata, fix ANN seed and `ef_construction`, and rank the union of all candidates; a missed ANN neighbor can reduce recall but cannot alter an existing cluster.

### [FUSED-STRATEGY-BOUNDED-MAX] Fused strategy (how the three signals combine)

The ID records the strategy this section originally specified; the **sum arm was removed by gh #343** (pinned by `issue_343_sum_clamp_saturation.rs`; `PairScore::bounded_fused` is the only fused combination) because the axes are correlated views of one normalised tree and their sum clamps mid-band clusters to a confidence of 1.0 that no single axis earned and no byte-identical pair backs. The strategy in force:

1. Compute a candidate set of clone pairs as the **union** of: structural-hash matches, LSH bucket collisions, and top-k embedding neighbors per subtree.
2. For each candidate pair, compute the evidence available before rescue in `[0,1]`: exact Merkle-hash evidence `H`, `token_jaccard`, and `embedding_cos`. Graded structural overlap is measured lazily only for rescue-eligible pairs and later for report election.
3. The pre-rescue fused pair score is the **strongest available axis** — `max(H, token_jaccard, embedding_cos)`, bounded to `[0,1]` (`PairScore::bounded_fused`). Never their sum, never their average.
4. **Admission is decided pair by pair.** A pair must pass the size-coherence and applicable LSH-only guards, then either clear its pair-specific fused threshold or satisfy the cross-file shared-subtree rescue, including token, node-count, and raw-content corroboration. There is no group-level judgement and no averaging over a group. Clusters are the transitive closure of admitted pairs.
5. Rank clusters by summed duplicated mass ([pipeline.md §RANK-MASS-SUM](pipeline.md#rank-mass-sum)) for "worst offenders first."

This way, a Type-1 clone scores ≈1 on all three signals, a Type-2 ≈1 on structural+embedding and ~high on LSH, a Type-3 may score high on LSH+embedding and medium on structural, and a Type-4 scores primarily on embedding. Every type lands in the report; scores explain *why*, and the fused confidence never exceeds the best of them. The axes are uncalibrated — a cosine 0.85, a Jaccard 0.85 and an alignment 0.85 are not the same weight of evidence, and under max the most generous axis wins; `fused_threshold` at 0.85 (above the literature's 0.7) pays that bill.

### [FUSED-SCOPE] `fused` is a pair quantity

**`fused` never refers to the whole cluster. It exists at the level of the pair only.** A cluster-wide fused is impossible by construction: averaging one across the member pairs is the mean that mispriced proven copies (gh #458), and summing ratios in `[0,1]` is meaningless.

`fused` is the pair's pre-rescue admission score: `PairScore::bounded_fused` — the strongest of exact Merkle evidence, `token_jaccard`, and `embedding_cos`, bounded to `[0,1]` ([FUSED-STRATEGY-BOUNDED-MAX]). It is compared pair by pair with the pair-specific threshold; the separately specified shared-subtree route may admit a below-threshold pair without changing that fused value. A cluster renders its bucket, the elected pair's remeasured axes ([FUSED-CLUSTER-SIGNALS]), and its content evidence ([FUSED-CONTENT-GATE]) — never a fused number. There is no cluster-level fused, no rendered confidence derived from it, and no cluster gate that compares one to the pair bar.

#### [FUSED-THRESHOLD] The pair admission bar

`admission.fused_threshold` (default 0.85) is the pair admission bar; provenance in [FUSED-TUNING-LEVERS]. It is per-pair data (`CandidatePair::fused_min_score`), not a global constant — a cross-language candidate with no structural anchor lowers it to `candidates.cross_language_min_jaccard`. Every threshold in these specs is a configurable default, never a hard-coded constant; the config surface lives in [exclusion.md](exclusion.md) and the migration in `unhardcode-tuning-plan.md`.

### [FUSED-SHARED-SUBTREE] `structural` is measured subtree overlap, not Merkle equality

`structural` is the **best-achievable ordered subtree overlap** between two occurrences: `1 - TED / max(nodes)`, where `TED` is the Zhang–Shasha tree edit distance over normalised node kinds with unit insert/delete/relabel costs (`overlap.rs`). Merkle-equal occurrences short-circuit to `1.0`, so every previously-`1.0` cluster is unchanged; what changes is the other end, which used to be a literal `0.0`.

This replaces the false-negative literal `0.0` (gh #408): one inserted statement rehashes every ancestor even when most nested statements remain equal. Across the five `*-type3` fixtures, enclosing pairs measure 0.84–0.91 while whole-method token Jaccard measures only 0.74–0.85. Overlap must preserve order and nesting; a bag of subtree hashes cannot distinguish a copy from unrelated functions using the same statement vocabulary.

Endpoints above `overlap.rs::ALIGNMENT_MAX_NODES` (768 nodes) use a conservative estimate that pairs disjoint identical subtrees from left to right on both sides:

- **Preserve order.** Swapped subtrees cannot both survive an ordered alignment (`the_fallback_never_credits_mass_no_ordered_alignment_can_reach`).
- **Convert matched mass to guaranteed shared mass.** Pairing `m` node pairs guarantees `max(2m − min(n₁,n₂), 0)`, accounting for unmatched nodes on both sides.

Forward-only pairing prevents nested double-counting (`the_fallback_never_credits_a_nested_right_subtree_twice`). The result may suppress a rescue but cannot manufacture one (`the_large_tree_fallback_never_exceeds_the_alignment`); `endpoints_past_the_alignment_cap_still_measure_as_shared` pins recall for a genuine large near-copy.

The cap counts normalised-tree nodes, including operator leaves from [PIPELINE-NORMALIZE-AST-OPERATOR](pipeline.md). It is set above the 558-node `ts-mixed-band` recall fixture; lowering it is a performance change pinned by `the_alignment_cap_is_the_documented_operating_point`.

**Rescue is a cross-file, pairwise compound gate.** A below-threshold pair requires the configured overlap, token Jaccard, endpoint node-count, endpoint-size-ratio, and raw-content-agreement floors; no axis admits alone, and rescue does not change the pre-rescue fused score. Measure overlap only for otherwise-dropped pairs already carrying token and node-count corroboration.

**Routing.** [CLONE-BUCKETS-ROUTING] row 4b sends overlap corroborated by token or embedding evidence to `nearly_identical` using the admission floors. The obsolete `structural ≤ 0.01` comparison is removed; clusters below the overlap floor retain the anchor-free demotion guard.

Because a nested window can exclude differing code, [PIPELINE-CLUSTER-SUBSUME] compares grades only between non-nested views; enclosure decides nested views within a credibility tier. `type3_enclosing_method.rs` pins all five languages.

### [FUSED-SHARED-SUBTREE-MEMO] Overlap is memoised by ordered Merkle hash pair

Memoise once per ordered endpoint-Merkle-hash pair, not per byte-range pair; equal hashes pin equal normalised structures, so this changes cost but not values or admission. `a_fleet_of_identical_windows_costs_one_alignment` pins 36 byte-range pairs to one alignment. Memoise exact and bound results separately because bounds answer only rescue admission.

### [FUSED-SHARED-SUBTREE-BOUND] The kind-multiset bound refuses hopeless alignments

Shared mass is at most `min(smaller_total, kind-multiset intersection)`. `rescue_overlap` skips quadratic alignment when `bound/larger` is below `admission.shared_subtree_min_overlap`; the bound never undercuts exact overlap, so admission is unchanged. Pins: `the_kind_multiset_bound_never_undercuts_the_alignment`, `the_rescue_path_agrees_with_the_exact_measure_on_admission`, and `a_pair_the_bound_refuses_never_pays_for_an_alignment`. Reports always use exact overlap.

### [FUSED-SHARED-SUBTREE-BOUND-ORDER] The order bound refuses alignments the multiset bound would allow

The multiset bound cannot see scrambled order. A bit-parallel Allison–Dix longest-common-subsequence calculation over post-order kind sequences supplies a never-looser upper bound at every endpoint size, including above the alignment cap, and runs whenever the multiset bound cannot refuse the pair. It removes 22% of rescue alignments on the Flutter slice without changing admissions. Pins cover machine-word boundaries (`the_bit_parallel_row_matches_the_textbook_table`), 3,600 generated pairs against Zhang–Shasha (`the_bound_never_understates_what_the_alignment_measures`), and scrambled order (`scrambled_order_is_bounded_far_below_the_shared_multiset`).

### [FUSED-CLUSTER-SIGNALS] A cluster displays one admitted pair's measured evidence

A rendered cluster's signal triple is the measurement of **one admitted pair** — the strongest — never a mean over pairs. Baker's p-match is a per-pair predicate: a group qualifies because its pairs pass, and there is no group-level "average match" to display (Baker 1995, "On Finding Duplication and Near-Duplication in Large Software Systems"). Displayed evidence must therefore attach to the pair that earned it: the report names the elected pair (wire field `signal_source`), so every displayed number is traceable to the exact two occurrences that produced it (gh #458).

**Which pair is elected.** Of the admitted pairs — those that cleared the admission gate — the cluster elects the one with the highest fused confidence (its strongest single axis, bounded to [0,1]); ties resolve to the earliest pair in corpus order, making the election deterministic across runs (gh #301). All three axes render from that one pair, together: a per-axis best drawn from different pairs would display a "super-pair" no actual pair measured. The 2025 ensemble study fuses scores attached to clone-candidate pairs, but electing one traceable pair for all three heterogeneous Deslop axes is Deslop's own reporting rule.

**The axes.** Per pair: `structural` is the measured shared-subtree overlap ([FUSED-SHARED-SUBTREE]) — `1.0` for Merkle-equal occurrences, the graded alignment otherwise; `token_jaccard` is the MinHash Jaccard estimate between the two signatures; `embedding_cos` is the cosine of the two vectors under the crate's single cosine definition ([FUSED-EMBED-PROVIDER]), so byte-identical occurrences — which share one vector — render exactly `1.0` (gh #372). A pair missing a signal input (no vector: embeddings off, oversized input, provider failure) renders `0.0` on that axis, the embeddings-off convention — absence never masquerades as a measured value.

**Only admitted pairs count.** Closure-only pairs — equal-hash combinations that never cleared admission — contribute nothing: they are artifacts of discovery topology (structural star buckets, ANN top-k fan-out, LSH band width), not of the rendered occurrences, and giving them a vote lets the deviant drag the verdict (Engler et al., "Bugs as Deviant Behavior", SOSP 2001: the majority outranks the deviant). gh #458 pinned: a byte-identical pair inside a lookalike cluster renders `1.0/1.0` and keeps its act-now bucket, while the lookalikes do not manufacture an identical verdict.

**The mean is dead.** The former per-pair mean over the closure component is removed. Under it, restored embedding evidence diluted a byte-identical file pair to `structural = 0.36` and routed it `same_behavior` instead of `identical` (gh #343 corpus, pinned by `issue_343_sum_clamp_saturation.rs`). The measured triple still feeds the cross-cluster subsumption pass, which compares structural values: diluted signals let contained artifact clusters escape collapse.

**For AI.** Election: `max over admitted pairs of (bounded_fused, Reverse(left), Reverse(right))` where bounded_fused = max(structural, token_jaccard, embedding_cos) clamped to [0,1], and left/right are corpus indices, so the lowest-index pair wins a fused tie. Rendered `PairScore` = the elected pair's own (structural, token_jaccard, embedding_cos), each `unwrap_or(0.0)` when the input is absent. `source_pair` = the elected pair's corpus indices; the wire field `ReportCluster.signal_source` holds their positions into `ReportCluster.occurrences`; `None` (all 0.0, no source) when every admitted pair's endpoint was collapsed by the same-file collapse (#339). Test pins: `the_rendered_triple_is_one_admitted_pairs_own_axes`, `non_admitted_pairs_never_contribute_to_the_rendered_signals`, `the_source_pair_election_is_deterministic`, `when_every_admitted_pair_skips_there_is_no_source_pair` (unit); `a_byte_identical_pair_reads_the_same_in_every_cluster` (E2E).

### [FUSED-CONTENT-GATE] Content agreement gates shape-identical confidence

`structural_sim` and `token_jaccard` are both computed from the *normalised* representation (identifiers and literals collapsed), so on any exact shape match they agree by construction: before gh #343 quarantined the sum their total saturated the clamp, and even under the bounded max a shape match still reads ≈1.0 while saying nothing about what the code actually said (gh #331, #336). The gate restores an independent member by measuring what normalisation erased:

1. For each cluster, walk each member's normalised subtree and hash the **raw source bytes** of every collapsed leaf, keeping the leaf's population (identifier vs literal position).
2. Measure two independent populations **for the elected pair** — the same pair [FUSED-CLUSTER-SIGNALS] elects for the shape axes, so every number on a cluster's signal row describes the same two occurrences — both in `[0, 1]`:
   - `agreement` — fraction of all collapsed positions whose raw bytes match, identifiers and literals pooled. Byte-identical members score 1.0; lightly-edited copies stay high; framework-mandated scaffolding (every name differs) and data tables (every literal differs) fall low.
   - `rename_consistency` — the Type-2 discriminator, [TECH-PMATCH-BAKER] quantified: the lesser of literal consistency (fraction of literal positions unchanged **or echoing an elected substitution**; vacuously 1.0 with none) and rename-mapping coverage, scaled by the smooth anchor-mass weight `anchors / (anchors + content_gate.rename_evidence_half_mass)`, where anchors are the consistent literal positions plus the explained identifier positions. A literal *echo* ([REPAIR-RENAME-LITERAL-ECHO], #409) is a substituted literal position whose raw bytes transform into the partner's bytes exactly by one bijection-explained identifier substitution — `"OrderService"` → `"UserService"` renamed alongside its symbol is the rename done thoroughly, not evidence against it — and the echo corroborates that substitution the way a repeated identifier occurrence would, so completing a rename can never score below leaving it half-finished (`rename_literal_monotonicity.rs`). Coverage classifies each identifier position exactly as Baker's prev-encoding constrains it: raw-byte identity is a fixed-symbol match, explained by the position itself; a substitution is explained when it is bidirectionally modal *among the substituted pairs* — fixed symbols and parameters are disjoint alphabets, and collapsed leaves carry no role, so a homonym byte-string (a preserved property name that also names a renamed local) must not let one role veto the other in a single modal election — and corroborated by at least `content_gate.rename_corroboration_min` occurrences; positions the bijection cannot explain are constrained-but-unexplained and count against coverage; a *consistent substitution seen once* is an unconstrained first occurrence (`prev = 0` matches any other first occurrence) and belongs to neither numerator nor denominator — a renamed one-shot declaration name is not evidence against the clone. Zero without positional alignment. Consistency alone cannot tell a rename from sibling scaffolding that also substitutes names consistently — the anchors carry that burden, and they must *weigh* the proof, never gate it: the deleted `rename_evidence_min_literals` cliff zeroed every pair below four literal anchors, pricing a maximal one-literal Type-2 rename to `0.0588` — an agent-surface false negative (`type2_rename_anchor_floor.rs`). **A certified rename carries no doubt left for the mass term to price** (gh #410). When the lesser of literal consistency and coverage is exactly 1.0, every aligned literal is preserved or echoed and every constrained identifier position is byte-identical or a corroborated bijection substitution: the mapping is total, contradiction-free and literal-preserving, and the only doubt the anchor mass still prices is coincidence. Coincidence is discharged by mass, so the discount is dropped exactly where the mass term already vouches for the pair on its own — where `anchors / (anchors + content_gate.rename_evidence_half_mass)` reaches `content_gate.support_floor`, i.e. at ten anchors. There the weight is 1.0 and `rename_consistency` reads 1.0. Certification therefore never promotes a cluster the mass discount would have demoted; it only stops charging a proven rename for evidence it is not missing. Below that bar, and for any pair carrying a single contradiction, the smooth discount applies unchanged, so an anchor-poor forwarding scaffold (subject name twice plus one collaborator, mass 3, weight 3/7) stays below every routing floor. Because completing a rename can only raise consistency and add anchors, certification can only switch on — the [REPAIR-RENAME-LITERAL-ECHO] monotonicity property is preserved. Without it the axis was capped at `rename_consistency_discount × anchors / (anchors + 4)`, so a certified rename could never read 1.0 and **no Type-2 rename cleared the act-now routing floor in any language**. Neither population is ever pooled with the other, and neither is averaged across a cluster's members: both are the elected pair's own measurement, the one rule [FUSED-CLUSTER-SIGNALS] states for every axis. Pooling them demoted textbook Type-2 clones to `structural_only` — a maximal rename scores low `agreement` and high `rename_consistency`, so the mean describes neither.

3. **Routing uses `support = max(agreement, rename_consistency)`** (either population may vouch; never their mean). The promotion floor depends on cluster spread: a cross-file cluster uses `content_gate.support_floor` (0.70), while a single-file cluster uses `content_gate.promote_floor` (0.85). This preserves recall for copies split across files while keeping real-world in-class sibling families such as #197 (0.72–0.80) demoted. Semantic support at or above `candidates.embedding_support_floor` leaves the legacy signal verdict unchanged. Otherwise, a gate-eligible cluster at or above its spread-dependent floor routes `nearly_identical`; one below it joins the demoted [RANK-STRUCTURAL-ONLY] routing, surfaced as `structural_only` or hidden as cross-file scaffolding according to the routing table.
4. **Token-signal correction.** A cluster whose members all carry **one** Merkle hash has normalised k-gram sets that are equal by construction; for such clusters routed `identical` / `nearly_identical` a lower rendered `token_jaccard` is a fallback-signature artifact and is corrected to 1.0 (the GH #232 argument). `structural_only` keeps its unscored signal — absent token support is that bucket's defining signature.

The correction is scoped by that digest equality, tested directly on the members, and by nothing else (gh #431). No reading of `structural` can stand in for it: since [FUSED-SHARED-SUBTREE] the axis grades subtree *overlap*, so it saturates by ratio as well as by hash equality, and every value below saturation means the subtrees provably differ. Scoping the correction to `content_gate.structural_saturation_floor` — a near-miss **routing** tolerance — published `token_jaccard = 1.0`, and the `shape` reading derived from it, across the whole `[0.99, 1.0)` band on no evidence. Routing tolerance is not proof of identity. Pinned by `crates/deslop/tests/content_gate_signal_honesty.rs`.
5. **Ranking reads none of this.** The report weight is the **sum** of duplicated mass — see [pipeline.md §RANK-MASS-SUM](pipeline.md#rank-mass-sum), which owns the formula — never a confidence factor, with no fused tie-break: at equal mass, cluster id makes the order total. Content evidence answers the binary question — is this a clone, and which bucket — never how heavily it weighs.

`token_jaccard` itself stays rename-invariant (normalised k-grams); the gate adds evidence rather than redefining an existing signal.

**The token echo is shape evidence too.** The LSH pass hashes k-grams of the same normalised kinds the structural pass hashes, so a near-total `token_jaccard` (≥ `content_gate.saturating_token_floor`, the near-identical routing line) saturates on shape matches exactly as `structural` does — the surviving flutter/flutter #331 cluster read `structural=0.62, token_jaccard=0.98` because transitive closure mixed structural and LSH pairs. The gate therefore fires on *either* saturating signal.

**The gate stops at the anchor-free route.** Row 4 of [taxonomy.md §CLONE-BUCKETS-ROUTING](taxonomy.md#clone-buckets-routing) is deliberately outside it. Both populations below assume the members align position for position, and `structural ≤ 0.01` says the shapes differ — so against a genuine Type-3 clone whose identifiers are all renamed and whose bodies differ by one statement (`csharp-type3`), agreement collapses to the literals (0.19) and rename consistency to 0.00, because the extra statement destroys the alignment the rename proof needs. Gating row 4 here would demote the renamed near-miss, the most valuable clone class there is. Row 4 is routed on cluster *spread* instead — see the taxonomy row. Shape-mismatched members have no positional alignment, so their agreement is the key-set Jaccard of their content keys — a genuine Type-3 near-miss shares nearly all of them; renamed scaffolding shares few. The verbatim guard is proportional and exclusive: one *token-identical family* — members sharing both the same normalised-subtree digest and the same collapsed-leaf keys — must hold a strict majority of the cluster (above `content_gate.verbatim_member_share_floor`). A verbatim pair among a couple of lookalikes (#104, share 2/3) still vouches for its cluster; two copied example widgets inside a 453-member framework family (0.4%) do not; and two *disjoint* identical pairs, each at exactly one half, vouch for nothing, because neither is a majority and the members they disagree with are the whole rest of the cluster. `data`-category clusters are exempt from the structural-only ranking demotion — their weight belongs to the `[ranking] data_clones` policy ([RANK-CATEGORY]) so `data_clone_weight = 1.0` can still restore a table the gate routed to the structural-only bucket.

### [FUSED-ALGEBRA] Every calculation, as algebra

The whole arithmetic surface in one place — one formula per block, the English right under it. Each block carries the spec id that owns it; if prose and algebra ever disagree, both are wrong — fix them together.

**Symbols.** `H` exact Merkle-hash evidence used during candidate admission, `S` measured structural overlap used by rescue and report election, `J` token_jaccard, `E` embedding_cos, `A` content agreement, `R` rename_consistency, `n` node count, `p` a candidate pair, `c` a cluster, `ℓ` a source line. `H` and `S` are deliberately distinct: a non-equal candidate starts admission with `H = 0`; its more expensive graded overlap `S` is measured only on the rescue path and later for the elected report pair.

**Pair admission** — [FUSED-STRATEGY-BOUNDED-MAX] [FUSED-THRESHOLD] [FUSED-SHARED-SUBTREE]

Structural similarity is the shared-node credit of the bigger tree. The aligner uses Zhang–Shasha's tree-edit-distance recurrence (keyroot decomposition over post-order sequences of normalised node kinds, unit insert/delete/relabel costs — Zhang & Shasha 1989). Deslop defines shared credit as `max(nodes) − TED` and normalises it by the larger tree. Baxter et al. 1998 supports subtree hashing and near-miss comparison, but not this TED composition: Baxter used a leaf-ignoring hash and `2S/(2S+L+R)`. Combining exact Merkle hashes with Zhang–Shasha TED is Deslop's design.

$$
\mathrm{shared}(a,b) = \max\bigl(n(a), n(b)\bigr) - \mathrm{TED}(a,b) \qquad
S(a,b) = \frac{\mathrm{shared}(a,b)}{\max(n(a),\, n(b))} = 1 - \frac{\mathrm{TED}(a,b)}{\max(n(a),\, n(b))}
$$

Merkle-equal pairs score 1.0 without paying for the walk; past the alignment cap a credited shared-node count — a sound lower bound on the aligned value — answers instead, and the share clamps to `[0, 1]`.

Exact structural evidence available before that walk is the Merkle-equality indicator:

$$
H(a,b) = \mathbf{1}\!\left[\operatorname{merkle}(a)=\operatorname{merkle}(b)\right]
$$

Embedding similarity is cosine over the raw vectors, accumulated in `f64` and clamped because negative cosine is treated as no positive clone evidence. A zero-norm vector returns zero; a non-zero vector compared with itself returns exactly one.

$$
E(a,b) =
\begin{cases}
0 & \text{if } \lVert v_a\rVert_2\lVert v_b\rVert_2 = 0 \\
\operatorname{clamp}\!\left(\dfrac{v_a \cdot v_b}{\lVert v_a\rVert_2\lVert v_b\rVert_2},\ 0,\ 1\right) & \text{otherwise}
\end{cases}
$$

The admission score takes the strongest signal available before rescue and clamps to `[0, 1]`. Max is a Deslop design choice, informed by the 2025 ensemble study's empirical preference for normalised max/sum variants over averaging; that study combines LLM scores and does not itself validate these heterogeneous structural, token, and embedding axes.

$$
f_{\mathrm{admit}}(p) = \operatorname{clamp}(\max(H(p),\, J(p),\, E(p)),\ 0,\ 1)
$$

The bar a pair must clear depends on the pair. Cross-language pairs without an exact structural anchor (`H = 0`) use the lower configured cross-language floor. Everything else uses `fused_threshold` (default 0.85), a Deslop operating point derived from its corpus. SourcererCC's 0.70 experiment is directional context only: it requires token-bag intersection of at least `0.70 × max(block sizes)`, over a different representation and similarity function.

$$
t(p) = \begin{cases} \text{cross\_language\_min\_jaccard} & \text{if cross-language}(p) \land H(p) = 0 \\ \text{fused\_threshold (default 0.85)} & \text{otherwise} \end{cases}
$$

`J` estimates the Jaccard of the two k-gram sets of normalised node kinds (Jaccard 1912). Broder's min-wise identity makes that estimable by hashing: for an ideal min-wise independent family, the probability that two minima agree is exactly the Jaccard. Deslop's BLAKE3-XOF slots are a deterministic practical approximation to that family, not a proof of exact min-wise independence.

$$
J(G_a, G_b) = \frac{|G_a \cap G_b|}{|G_a \cup G_b|} \qquad\qquad
\Pr_{h \in \mathcal{H}}\!\left[\arg\min_{x \in G_a} h(x) = \arg\min_{y \in G_b} h(y)\right] = J(G_a,G_b)
$$

The shipped estimator averages agreement over the `m = representation.minhash_signature_len` blake3-hashed slots (`lsh::estimate_jaccard`); LSH banding follows the standard `BANDS × ROWS_PER_BAND` collision curve.

$$
\hat{J}(G_a, G_b) = \frac{1}{m} \sum_{i=1}^{m} \mathbf{1}\bigl[\sigma_{G_a}(i) = \sigma_{G_b}(i)\bigr]
$$

With `b` bands and `r` rows per band (`m = br`), the idealised probability that a pair of true Jaccard similarity `s` collides in at least one band is:

$$
P_{\mathrm{candidate}}(s) = 1 - \left(1-s^r\right)^b
$$

A pair is admitted when its pre-rescue score clears its threshold or the shared-subtree rescue fires, subject to the size-coherence and LSH-only guards. The rescue is cross-file only and requires its own raw-content agreement; those conditions are load-bearing parts of the implementation, not optional prose.

$$
\begin{aligned}
\mathrm{rescue}(p) \iff {}& \operatorname{cross\_file}(p)
\land f_{\mathrm{admit}}(p) < t(p) \\
&\land S(p) \ge \text{shared\_subtree\_min\_overlap}
\land J(p) \ge \text{shared\_subtree\_min\_jaccard} \\
&\land \min(n_l,n_r) \ge \text{shared\_subtree\_min\_node\_count}
\land A(p) \ge \text{content\_support\_floor}
\end{aligned}
$$

When exact structural and embedding evidence are absent, and rescue did not fire, MinHash alone can carry the pair only above its pair-specific LSH-only floors. Explicit cross-language mode waives the ordinary node-count floor by raising the stored guard value to that floor; it does not falsify either endpoint's measured `n`. This is a rejection guard, not a rescue.

$$
\begin{aligned}
\mathrm{lsh\_ok}(p) \iff {}&
\bigl(H(p)=0 \land E(p)=0 \land \neg\mathrm{rescue}(p)\bigr)
\implies
\bigl(J(p) \ge \text{lsh\_only\_min\_jaccard}
\land n_{\mathrm{lsh}}(p) \ge \text{lsh\_only\_min\_node\_count}\bigr), \\
n_{\mathrm{lsh}}(p) = {}&
\begin{cases}
\max\!\bigl(\min(n_l,n_r),\text{lsh\_only\_min\_node\_count}\bigr) & \text{if explicit cross-language}(p)\land H(p)=0 \\
\min(n_l,n_r) & \text{otherwise}
\end{cases}
\end{aligned}
$$

Pairs without an exact structural anchor must also have coherent endpoint sizes. Putting all gates together:

$$
\begin{aligned}
\mathrm{size\_ok}(p) &\iff H(p)=1 \lor \max(n_l,n_r) \le
\text{max\_endpoint\_node\_ratio}\,\min(n_l,n_r) \\
\mathrm{admit}(p) &\iff \mathrm{size\_ok}(p)
\land \mathrm{lsh\_ok}(p)
\land \bigl(f_{\mathrm{admit}}(p) \ge t(p) \lor \mathrm{rescue}(p)\bigr)
\end{aligned}
$$

**Election** — [FUSED-CLUSTER-SIGNALS]

Every admitted pair in a cluster is remeasured on all available axes; the cluster elects the strongest measured pair. This election key uses graded `S`, unlike the pre-rescue admission score's exact-hash `H`. Ties break by the earlier left position, then the earlier right.

$$
q(p) = \operatorname{clamp}(\max(S(p),J(p),E(p)),0,1) \qquad
p^*(c) = \arg\max_{p\in\operatorname{admitted}(c)} \bigl(q(p),-\operatorname{left}(p),-\operatorname{right}(p)\bigr)
$$

The report shows the elected pair's three signals and cites its positions as the evidence source. Nothing else in the cluster is quoted.

$$
\mathrm{rendered}(c) = (S, J, E) \text{ of } p^*(c) \qquad \text{signal\_source = positions of } p^*(c)
$$

**Content evidence** (the elected pair) — [FUSED-CONTENT-GATE]

Agreement compares collapsed-leaf keys. With equal position counts it is a positional match share over authored content plus every disagreement; a matching operator is excluded because the shape axes already counted it. With unequal counts it falls back to set Jaccard after removing shared non-authored keys from both numerator and denominator. The positional branch is an accuracy ratio, not a Jaccard index.

$$
\begin{aligned}
M_{ab} &= \{i : k_{a,i}\ne k_{b,i}\ \lor\ k_{a,i}\text{ is authored content}\} \\
F_{ab} &= \{k\in K_a\cap K_b : k\text{ is non-authored}\} \\
m_a &= \text{number of frontier positions in }a, \qquad m_b = \text{number of frontier positions in }b \\
A(a,b) &=
\begin{cases}
\dfrac{|\{i\in M_{ab}:k_{a,i}=k_{b,i}\}|}{|M_{ab}|} & \text{if } m_a=m_b \\[10pt]
\dfrac{|K_a\cap K_b|-|F_{ab}|}{|K_a\cup K_b|-|F_{ab}|} & \text{otherwise}
\end{cases}
\end{aligned}
$$

Either zero denominator yields `1.0`: there is no authored content on which the pair disagrees.

Rename mass discounts anchor-poor evidence smoothly. The configured `rename_evidence_half_mass` is the anchor count at which this weight equals one half.

$$
w_{\text{mass}} = \frac{\text{anchors}}{\text{anchors} + \text{rename\_evidence\_half\_mass}}
$$

Evidence is certified only when it is airtight: literal consistency and coverage both perfect, and the mass clears the support floor.

$$
\mathrm{certified} \iff \min(\text{literal\_consistency},\, \text{coverage}) = 1.0 \land w_{\text{mass}} \ge \text{support\_floor}
$$

Certified evidence weighs at full strength; everything else keeps its asymptotic mass weight. The axis carries no discount — routing reads `R` undiscounted, and the residual-doubt discount's only consumer was the retired rendered confidence.

$$
w = \begin{cases} 1.0 & \text{if certified} \\ w_{\text{mass}} & \text{otherwise} \end{cases}
$$

Rename consistency is the weaker of consistency and coverage, scaled by the weight above.

$$
R = \min(\text{literal\_consistency},\, \text{coverage}) \times w
$$

Support is whichever population vouches harder — matched lines or rename consistency. Never a mean, never pooled: averaging would let two lukewarm signals impersonate one strong one.

$$
\mathrm{support} = \max(A, R) \qquad \text{(either population may vouch; never a mean, never pooled)}
$$

**Routing** — [FUSED-CONTENT-GATE] [CLONE-BUCKETS-ROUTING]

The gate applies only after the anchor-free near-miss route and only to a `nearly_identical` or `structural_only` candidate with saturating shape evidence. Semantic support preserves the incoming verdict. Otherwise the promotion floor is 0.70 across files and 0.85 within one file; evidence below that floor enters the demoted routing.

$$
\begin{aligned}
p(c) &= \begin{cases}
\text{support\_floor} & \text{if } c\text{ spans multiple files} \\
\text{promote\_floor} & \text{otherwise}
\end{cases} \\
E(c) \ge \text{embedding\_support\_floor} &\implies \text{preserve incoming routing} \\
E(c) < \text{embedding\_support\_floor} \land \mathrm{support}(c) \ge p(c)
&\implies \text{nearly\_identical} \\
E(c) < \text{embedding\_support\_floor} \land \mathrm{support}(c) < p(c)
&\implies \text{demoted routing}
\end{aligned}
$$

When every member of a bucket shares one Merkle hash, the tokens are identical by construction — so the rendered Jaccard is corrected to 1.0, suppressing MinHash estimation noise.

$$
\text{all members one Merkle hash} \land \text{bucket} \in \{\text{identical},\ \text{nearly\_identical}\} \implies \text{rendered } J = 1.0 \qquad \text{(token correction)}
$$

**Weight and order** — [RANK-MASS-SUM] [RANK-CATEGORY] [RANK-STRUCTURAL-ONLY]

Weight is size times duplication: more canonical nodes weigh more, each extra occurrence adds the size again, and the category and structural-only multipliers scale the result. A cluster with fewer than two visible occurrences weighs nothing — singletons are not duplication.

$$
\mathrm{weight}(c) = \text{canonical\_nodes}(c) \times (\mathrm{visible}(c) - 1) \times \text{category\_multiplier} \times \text{structural\_only\_multiplier} \qquad \text{(0 when visible < 2)}
$$

Clusters sort heaviest first; the governing [RANK-MASS-SUM] specification says ties break by cluster id ascending so the report order is stable.

$$
\text{order} = \text{weight descending, then cluster id ascending}
$$

*A duplicate is a duplicate: content evidence decides whether a cluster is reported and which bucket it wears — never how heavily it weighs.*

**Repo metrics** — [METRICS-REPO] [METRICS-REPO-WEIGHTED]

The headline number is unweighted duplicated-line density. It uses the same ratio form as SonarQube's gate, but the tools' analysed-line and clone projections differ, so the values are not interchangeable measurements.

$$
\text{duplication\_percent} = \begin{cases}
0 & \text{if analysed\_loc}=0 \\
\operatorname{clamp}\!\left(\dfrac{100 \times \text{duplicated\_loc}}{\text{analysed\_loc}},\ 0,\ 100\right) & \text{otherwise}
\end{cases}
$$

A line's weight is the heaviest bucket of any cluster covering it — a line duplicated by a critical cluster counts as critical.

$$
\text{line\_weight}(\ell) = \begin{cases}
0 & \text{if no cluster covers }\ell \\
\max\limits_{c\text{ covers }\ell}\bigl(\text{bucket\_weight}(c) \times \text{category\_weight}(c)\bigr) & \text{otherwise}
\end{cases}
$$

The weighted variant sums those per-line weights. Specified for completeness, not shipped.

$$
\text{weighted\_percent} = \begin{cases}
0 & \text{if analysed\_loc}=0 \\
\operatorname{clamp}\!\left(\dfrac{100 \times \sum_{\ell} \text{line\_weight}(\ell)}{\text{analysed\_loc}},\ 0,\ 100\right) & \text{otherwise}
\end{cases}
\qquad \text{(specified, not shipped)}
$$

Every configured bucket and category weight lies in `[0,1]`, so weighting can only retain or discount a mechanically duplicated line. It cannot increase the headline density.

$$
0 \le \text{weighted\_percent} \le \text{duplication\_percent} \le 100 \qquad \text{(invariant)}
$$

**Provenance.** The literature behind the formula lines (links in [reading-list.md](reading-list.md)); quoted where the claim is verbatim:

| Decision | Source | Verified claim |
|---|---|---|
| Fingerprinted structural axis | Chilowicz et al. 2009 | *"each node of an AST is associated with a fingerprint based on a hash value (incrementally computed) of the subtree rooted at the node"* |
| Exact hash and near-miss precedent | Baxter et al. 1998 | hashes AST subtrees; its near-miss method uses a leaf-ignoring hash and `2S/(2S+L+R)`, not tree edit distance |
| The `TED` recurrence (unit costs, post-order keyroot decomposition) | Zhang & Shasha 1989 | the textbook ordered-tree edit distance the aligner implements; `shared = max − TED` and its normalisation are Deslop definitions layered on that distance |
| The Jaccard target and the min-wise identity behind `Ĵ` | Jaccard 1912; Broder 1997 | J = \|A∩B\|/\|A∪B\|; the minimum-hash agreement probability equals the Jaccard |
| Normalised max as a combination option | arXiv:2510.15480 | evaluates score combination for two LLM outputs and finds normalised max/sum variants outperform averaging in its datasets; applying max to Deslop's heterogeneous axes and to content support is a Deslop design choice |
| `fused_threshold` context | SourcererCC (Sajnani et al. 2016) | its evaluated 0.70 setting requires `\|X∩Y\|/max(\|X\|,\|Y\|) ≥ 0.70` over token bags; Deslop's 0.85 applies to a bounded maximum over different axes and is not a calibrated translation |
| Routing separates evidence classes — no confidence scaling of findings | Svajlenko & Roy 2015 | BigCloneBench syntactic-similarity bands (VST3 ≥ 0.90, ST3 0.70–0.90, MT3 0.50–0.70, WT3/4 < 0.50); tool precision and recall degrade monotonically as similarity falls, and `promote_floor` 0.85 sits inside the ST3 band |
| Shape-only repetition demoted, never a failing verdict | Kapser & Godfrey 2008 | shape-level repetition is the weakest ground for a failing verdict |
| Repo percentage ratio | SonarQube metrics | SonarQube defines duplicated-line density as `duplicated_lines / lines × 100`; Deslop uses the same ratio form over its own analysed-line and clone projections |

Content-evidence arithmetic (`A`, `R`, the asymptotic weight, the routing floors) and the mass-sum weight are **derived or defect fixes**, not literature — their provenance rows in [FUSED-TUNING-LEVERS] say so.

Every numeric constant above is a configurable default, never a hard-coded value — provenance in [FUSED-TUNING-LEVERS], surface in [exclusion.md](exclusion.md), migration in `unhardcode-tuning-plan.md`.

### [FUSED-TUNING-LEVERS] Every threshold is a configuration item with a recorded provenance

A number is a **lever** when changing it changes which clusters are reported, which bucket they land in, or how they rank. Every lever is named, defaulted to the value compiled today, range- and invariant-validated at load ([EXCLUSION-CONFIG] `[tuning]`), and declared in the report that its value produced ([CONFIG-TUNING-DECLARED]).

**Unhardcoding is behaviour-preserving.** A run with no `[tuning]` section, no `--tune` flag, and no editor override produces a byte-identical report to the pre-migration build on every fixture and every corpus repository. Changing a *default* is a separate change with its own failing test, its own provenance entry, and its own corpus measurement — widening one during the migration is how an unhardcoding refactor becomes an undetected recall loss.

**Provenance is part of the spec.** A threshold with no recorded justification is an unfalsifiable claim, so each default carries one of four kinds: **literature** (a published operating point, cited by its [TECH-*] id), **defect** (an observed false positive or negative, cited by issue — it says what the value must *not* admit, which beats a curve), **derived** (follows from the fused algebra or another lever, with the derivation stated), or **unrecorded** (a tracked gap, not a resting state — each earns a citation, a defect, or a measured sweep).

| Key | Site | Default | Provenance |
| --- | --- | --- | --- |
| `admission.fused_threshold` | `pair.rs:31` | 0.85 | **Derived.** Under bounded max one axis alone can carry a pair, so the bar on that axis rises to compensate. SourcererCC's 0.70 token-bag intersection-over-larger-block setting is directional context, not an equivalent threshold for Deslop's heterogeneous axes. Not an ROC sweep. |
| `admission.lsh_only_min_jaccard` | `pair.rs:36` | 0.90 | **Defect.** Not a similarity threshold — a guard. LSH-only pairs have no structural anchor, and tiny `using`/`namespace` sibling windows hit Jaccard ≈ 1.0 by accident, then merge into a mega-cluster through transitive closure. |
| `admission.lsh_only_min_node_count` | `pair.rs:43` | 40 | **Defect.** The same defect's other half, applied at both endpoints: an 18-node k-gram set is mostly grammar scaffolding, so tens of thousands of such subtrees agree by accident. |
| `admission.max_endpoint_node_ratio` | `pair.rs:61` | 4 | **Defect** (#368). [PAIR-SIZE-COHERENCE] — an embedding-only pair scored a 19-node parameter list against a 274-node arithmetic chain at cosine 1.00. Deliberately loose; fires only where the pair is self-contradictory. |
| `admission.shared_subtree_min_overlap` | `pair.rs` | 0.75 | **Defect** (#408). Measured: the five genuine `*-type3` whole-method near-miss pairs score 0.84–0.91 overlap, so the floor sits below every one of them with margin, while requiring that three quarters of the larger tree align. Never admits alone — `shared_subtree_min_jaccard` must corroborate. |
| `admission.shared_subtree_min_jaccard` | `pair.rs` | 0.65 | **Defect** (#408). The corroboration floor, deliberately *below* `lsh_only_min_jaccard`: a one-statement Type-3 insertion measures 0.74–0.85 exact whole-method Jaccard precisely because the inserted statement dilutes the k-gram set. Above 0.85 it would re-close the recall hole it exists to open. |
| `admission.shared_subtree_min_node_count` | `pair.rs` | 30 | **Defect** (#408). Below `lsh_only_min_node_count` because this route carries structural corroboration that LSH-only pairs lack, and above grammar scaffolding: the smallest genuine fixture method (`python-type3`'s `aggregate`) is 31 nodes. |
| `candidates.cross_language_min_jaccard` | `pair.rs:66` | 0.10 | **Derived.** Cross-language AST vocabularies differ and the mode is opt-in ([CONFIG-CROSS-LANGUAGE]), so the floor sits below the same-language LSH-only floor. |
| `candidates.embedding_min_cosine` | `embedding/pairs.rs:27` | 0.80 | **Derived** (provenance audit). A candidate-set gate only — `fused_threshold` decides admission downstream. SSCD tabulates `0 / 0.95`; 0.80 is Deslop's own operating point, not a published one. |
| `candidates.embedding_top_k` | `embedding/pairs.rs:16` | 5 | **Unrecorded.** The stated rationale — recall comes from the union, not the ANN fan-out — argues for *small*, not for *five*. |
| `candidates.embedding_exact_pair_limit` | `embedding/pairs.rs:22` | 256 | **Unrecorded.** |
| `content_gate.support_floor` | `buckets.rs:237` | 0.7 | **Derived** (#341, provenance audit). SourcererCC's 0.7 is token overlap similarity; here it prices raw-byte agreement. Value kept; literature label dropped. |
| `content_gate.promote_floor` | `buckets.rs:248` | 0.85 | **Derived** (#341). The act-now routing grade for `support`; bounded below by a defect — the #197 REST settings family measures 0.72–0.80 and must keep its demoted verdict. |
| `content_gate.structural_only_max_support` | `buckets.rs:215` | 0.05 | **Defect.** #197's acceptance criterion (`token_jaccard = 0.00`, `embedding_cos = 0.00`) plus tolerance for MinHash collision noise. It is a ceiling below which a signal counts as *absent*, and is never a support floor — `route_shape_identical` read it as one, so a cosine of 0.05 overruled the measured content evidence and the gate's verdict followed whether the embedding pass ran (#356). |
| `candidates.embedding_support_floor` | `pair.rs:91` | 0.80 | **Derived** (#356). The cosine at which a measured `embedding_cos` is the embedding pass *vouching for* a cluster rather than merely having measured it — the ANN candidate gate's own operating point, and the line [CLONE-BUCKETS-ROUTING] row 2 lets semantic evidence carry a bucket alone. The [FUSED-CONTENT-GATE] escape is judged against it. |
| `content_gate.saturating_token_floor` | `buckets.rs:291` | 0.95 | **Defect** (#368). The surviving flutter/flutter #331 cluster read `structural = 0.62, token_jaccard = 0.98` — the token layer echoing shape, not reporting content. |
| `content_gate.rename_consistency_discount` | `buckets/gate.rs:143` | 0.9 | **Derived** (#346), a house rule and not literature. The certified-rename separator for the *retired* rendered confidence — the only consumer is the shape × content multiply's `content_confidence = max(A, discount × R)` (`gate.rs:200`); routing support reads `R` undiscounted (`content_support`). Dies with the cluster-fused rollout. |
| `content_gate.rename_corroboration_min` | `content.rs` | 2 | **Literature.** [TECH-PMATCH-BAKER] prev-encoding: a parameter symbol's first occurrence matches anything and constrains nothing; only repetition carries binding proof. |
| `content_gate.rename_evidence_half_mass` | `content/rename.rs` | 4 | **Defect.** Replaces the `rename_evidence_min_literals = 4` cliff (#346), which zeroed sub-floor rename evidence and priced a maximal one-literal Type-2 rename to `0.0588` (`type2_rename_anchor_floor.rs`). Same operating point, now a half-saturation mass: a forwarding echo's single substitution (mass 2, weight 1/3) stays below every routing floor while a 16-anchor maximal rename clears them all. The weight is an asymptote, so it applies only while doubt remains: a rename certified contradiction-free at or above `content_gate.support_floor` of mass weighs 1.0 (#410, above). |
| `content_gate.verbatim_member_share_floor` | `content.rs:54` | 0.5 | **Defect** (#341, tightened #346). A strict majority — the share must *exceed* it. #104's verbatim pair among lookalikes (share ≥ 2/3) must stay visible; two byte-identical widgets inside 453 framework declarations (≈ 0.004) must not vouch for the family; and two disjoint identical pairs at exactly 0.5 must not certify each other. |
| `content_gate.literal_table_min_fraction` | `buckets.rs:257` | 0.8 | **Derived** (#341), value unswept. "Overwhelmingly literal" is the stated criterion for [CLONE-NOISE-LITERAL-TABLE]; 0.8 is where it was set, not where it was measured. |
| `content_gate.literal_table_min_literals` | `content.rs:36` | 8 | **Derived** (#341), value unswept. A data table is a run of values, so a two-element tuple return must not reach the classifier — the argument fixes the direction, not the number. |
| `ranking.type4_embedding_floor` | `cluster.rs:397` | 0.90 | **Unrecorded.** |
| `ranking.low_structural_type4_ceiling` | `cluster.rs:395` | 0.10 | **Unrecorded.** |
| `ranking.low_structural_type4_weight` | `cluster.rs:399`–`401` | 1/10 | **Unrecorded.** |
| `routing.proven_identical_token_floor` | `report_render.rs:236` | 0.99 | **Unrecorded.** |

`[ranking] data_clone_weight` (0.15) and `structural_only_weight` (0.15) are levers by this definition and are **already configuration** ([RANK-CATEGORY], [RANK-STRUCTURAL-ONLY]); they keep their existing section rather than moving.

**Unnamed levers.** These fail the naming requirement today — they are inline literals in comparisons, so no test can assert them and no spec can reference them. Naming each one is a prerequisite for configuring it:

| Site | Literals | Governs |
| --- | --- | --- |
| `buckets.rs:357` | `0.99`, `0.99` | `routing.identical_*` — the `Identical` line |
| `buckets.rs:359` | `0.80`, `0.50` | `routing.same_behavior_*` — the `SameBehavior` line |
| `buckets.rs:363`–`364` | `0.99`, `0.20`, `0.95` | `routing.nearly_identical_*` — the `NearlyIdentical` line |
| `buckets.rs:225`, `:282`, `:342`; `report_render.rs:297` | `0.99` | `routing.shape_identical_floor` — one concept written out four times, so a change to one is a silent divergence |
| `report.rs:371`–`374` | `0.10`, `0.80`, `10`, `500` | `suppression.embedding_mega_*` — embedding-dominant mega-cluster suppression |
| `refactor/merge/gate.rs:20`, `:24`, `:27`, `:31` | `20`, `6`, `6`, `0.95` | [AUTOFIX-MERGE] eligibility |
| `refactor/merge/naming.rs:12` | `4` | [AUTOFIX-MERGE] parameter ceiling |

**Representation parameters** — `min_nodes` (30), `kgram_width` (5), `minhash_signature_len` (128), `lsh_bands` (32), `sibling_max_window_width` (8), `max_ast_depth` (500), `embedding_chars_per_token` (3) — are levers too, but they change what is hashed or dispatched, so they are cache-keyed ([CONFIG-TUNING-CACHE]) rather than free to vary per run.

**Not levers, and never configuration.** `MIN_REPORTABLE_MEMBERS = 2` (`cluster.rs:63`) is definitional — a cluster of one is not duplication. `HNSW_SEED` (`embedding/pairs.rs:31`) is determinism ([PIPELINE-DETERMINISM]); a configurable seed makes runs irreproducible. `F64_MAX_EXACT_INTEGER*` and `F64_TWO_POW_32` (`cluster.rs:388`–`393`) are IEEE-754 facts. `MAGIC` (`fpcache.rs:32`) and `MANIFEST_VERSION` (`version_contract.rs:10`) are format identity. Presentation and transport limits — `LIVE_WIRE_OCCURRENCE_CAP`, `SNIPPET_PREVIEW_LINES`, `CHANNEL_CAPACITY`, `BROADCAST_CAPACITY`, `MIN_CLUSTER_ID_PREFIX_LEN`, and the debouncer's `QUIET_MS` / `CAP_MS` — change what a surface shows or how promptly, never which clusters exist; if ever exposed they belong to a `[live]` or `[report]` section.

### [REMOVE-STUB] Test-only stub provider must never ship
The deterministic BLAKE3 stub embedding provider named in [FUSED-EMBED-PROVIDER] exists purely so E2E tests can exercise the embedding path without a live model. It lives behind the `test-support` Cargo feature, is **never** registered in `ProviderRegistry::production`, and is barred from the shipped VSIX by a packaging gate. `[REMOVE-STUB]` tags the code sites that enforce this boundary so a grep proves the stub cannot leak into a release; any new stub-touching code must carry the tag and stay test-only.
