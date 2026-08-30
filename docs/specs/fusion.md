# Pipeline design — hybrid, not pure-RAG

Deslop combines structural, token, and embedding analysis. The surveyed systems in [landscape.md](landscape.md) and [reading-list.md](reading-list.md) likewise combine representations rather than relying on vector search alone.

### [FUSION-SIGNALS-THREE-LAYER] Deslop is hybrid by design

The pipeline fuses three signals:

1. **Structural (AST fingerprinting)** — Merkle-hash every tree-sitter subtree after normalization. Catches Type-1, Type-2, most Type-3. Fast, deterministic, gives exact byte ranges. (Chilowicz 2009 + Baxter 1998.)
2. **Token LSH (MinHash over normalized k-grams)** — catches Type-3 cases where structure diverged but token bag is close. Fast, deterministic. (SourcererCC 2016.)
3. **Learned embeddings (local, via Ollama)** — catches Type-3/Type-4 the structural passes miss. Used both as a **recall expander** (find candidates the hash-based passes didn't cluster) and as a **re-ranker** (promote semantically-similar AST clusters in the final score). (SSCD 2024, ensemble-LLM 2025.)

The structural and token layers always run. The embedding layer is opt-in: `--embeddings` defaults to `off`, because it needs a reachable local Ollama and the shipped CLI must produce a report on a machine that has none. `auto` uses embeddings when the provider answers and warns when it does not; `required` hard-fails instead. Leaving it off is a measurable recall loss on Type-3/4 — the research does not support it as a permanent posture, only as a default that never blocks a first run.

### [FUSION-EMBED-PROVIDER] Embedding layer — concrete choices

- **Provider and model are not hard-coded.** Both are CLI flags (`--embedding-provider`, `--embedding-model`). The core crate exposes an `EmbeddingProvider` trait; production providers are selected at runtime by name through `ProviderRegistry::production`, which registers `ollama` only today. A local `onnx` provider is on the roadmap and slots in by registering another factory — no transport-specific special-casing. A deterministic BLAKE3 stub provider exists purely as test infrastructure: it lives behind the `test-support` Cargo feature, is never registered in production, and is barred from the shipped VSIX by a packaging gate. The research picks a *default*, not a lock-in.
- **Default provider + model (overridable).** Provider defaults to `ollama` (local, no network) and model to `nomic-embed-text` (`DEFAULT_OLLAMA_MODEL`), the embedding model an Ollama install is most likely to already carry. `nomic-embed-code` is the code-tuned alternative and is selected with `--embedding-model`; both follow the 2025 ensemble paper's finding that *"smaller embedding sizes, smaller tokenizer vocabularies and tailored datasets are advantageous"*. CodeT5+110M and UniXCoder are alternate top performers cited in the literature; either should be selectable via `--embedding-model` once exposed through a provider.
- **Local-only is a policy, not a hard requirement of the architecture.** The default stack never touches the network, but the trait doesn't forbid a hosted provider. A user configuring `--embedding-provider=hosted-foo` opts into that tradeoff deliberately; we don't enable it for them.
- **ANN index: HNSW.** Use `usearch` or `instant-distance` (pure Rust, no C deps). SSCD validated HNSW at 250 MLOC.
- **One cosine definition, accumulated in `f64`.** Exact-path pair admission and every rendered `embedding_cos` use the crate's single cosine function (`embedding::cosine_similarity`): dot product and norms accumulated in `f64` over the raw `f32` components, no intermediate normalised vector, result clamped to `[0, 1]`. Byte-identical snippets share one vector, so their rendered cosine is exactly `1.0` — `f32` accumulation grew error with vector width and reported `0.999998` (gh #372, pinned by `issue_372_identical_snippet_cosine.rs` and the unit tests beside the function). The HNSW pass measures `f32` cosine distance only to discover candidates; discovery-edge cosines are never rendered ([FUSION-CLUSTER-SIGNALS]).
- **Ensemble by max, never sum or average.** The 2025 ensemble paper's max/sum finding assumes independent members; Deslop's structural and token axes are two views of one normalised tree, so summing them manufactures confidence neither carries alone (gh #343). Fusion takes the strongest single axis; score normalization is mandatory before combining.
- **Cache by `(file_content_hash, provider_id, model_id, model_version)`.** Re-runs are free; switching providers or models invalidates only the embedding layer and leaves structural/LSH caches intact. LSP incremental mode reuses the same cache unchanged.
- **Index granularity: AST subtrees above min-node threshold**, not whole files. We already have those subtrees from the structural pass — embed them directly. This keeps embeddings byte-range-addressable and dramatically reduces the N in k-NN.
- **The per-input character budget belongs to the provider, never to the pipeline.** A subtree longer than the budget is counted in `failed_subtrees` and never dispatched, because Ollama truncates silently (`truncate: true`) and a truncated vector misrepresents the code it claims to describe. The budget is therefore a property of the model behind the provider — `nomic-embed-text` carries a 2,048-token context, `mxbai-embed-large` 512, `qwen3-embedding` 32k — and is read from `EmbeddingProvider::max_input_chars`. `OllamaProvider` derives it at construction from the model's own `model_info["<arch>.context_length"]` via `POST /api/show`, converted at a deliberately conservative 3 chars/token, falling back to `DEFAULT_MAX_INPUT_CHARS` (6,000) when the endpoint or field is unavailable. A single pipeline-wide constant cannot be correct for two models an order of magnitude apart: gh #286 reported 14,723 of 175,160 subtrees (8.4%) dropped, at the large end where re-derived duplication is most expensive to miss, and no model swap could have recovered them while the cap sat upstream of the provider.
- **Determinism caveat.** Embedding + ANN is approximate. Mitigate by: (a) recording `provider_id`, `model_id`, and `model_version` in the `.deslop/cache` header and the report, (b) using deterministic ANN parameters (fixed seed, fixed ef_construction), (c) final ranking is still computed over the *union* of structural + LSH + embedding candidates, so a missed ANN neighbor only loses recall, never changes existing cluster content.

### [FUSION-STRATEGY-BOUNDED-MAX] Fusion strategy (how the three signals combine)

The ID records the strategy this section originally specified; the **sum arm was removed by gh #343** (pinned by `issue_343_sum_clamp_saturation.rs`; `PairScore::bounded_fused` is the only fusion) because the axes are correlated views of one normalised tree and their sum clamps mid-band clusters to a confidence of 1.0 that no single axis earned and no byte-identical pair backs. The strategy in force:

1. Compute a candidate set of clone pairs as the **union** of: structural-hash matches, LSH bucket collisions, and top-k embedding neighbors per subtree.
2. For each candidate pair, compute three scores in [0,1]: `structural_sim`, `token_jaccard`, `embedding_cos`.
3. Final pair score = the **strongest single axis** — `max(structural_sim, token_jaccard, embedding_cos)`, bounded to [0,1] (`PairScore::bounded_fused`). Never their sum, never their average.
4. **Admission is decided pair by pair.** Each pair either clears
   `admission.fused_threshold` ([FUSION-TUNING-LEVERS]) or it does not; there is
   no group-level judgement and no averaging over a group. Clusters are the
   transitive closure of the pairs that cleared it.
5. Rank clusters by summed duplicated mass ([pipeline.md §RANK-MASS-SUM](pipeline.md#rank-mass-sum)) for "worst offenders first."

This way, a Type-1 clone scores ≈1 on all three signals, a Type-2 ≈1 on structural+embedding and ~high on LSH, a Type-3 may score high on LSH+embedding and medium on structural, and a Type-4 scores primarily on embedding. Every type lands in the report; scores explain *why*, and the fused confidence never exceeds the best of them. The axes are uncalibrated — a cosine 0.85, a Jaccard 0.85 and an alignment 0.85 are not the same weight of evidence, and under max the most generous axis wins; `fused_threshold` at 0.85 (above the literature's 0.7) pays that bill.

### [FUSION-SCOPE] `fused` is a pair quantity

**`fused` never refers to the whole cluster. It exists at the level of the pair only.** A cluster-wide fused is impossible by construction: averaging one across the member pairs is the mean that mispriced proven copies (gh #458), and summing ratios in `[0,1]` is meaningless.

`fused` is the pair's admission score: `PairScore::bounded_fused` — the strongest single axis of `structural`, `token_jaccard`, `embedding_cos`, bounded to `[0,1]` ([FUSION-STRATEGY-BOUNDED-MAX]). It decides admission, pair by pair, against `admission.fused_threshold` — nothing else. A cluster renders its bucket (the verdict), the elected pair's measured axes ([FUSION-CLUSTER-SIGNALS]), and its content evidence ([FUSION-CONTENT-GATE]) — never a fused number. There is no cluster-level fused, no rendered confidence derived from it, and no gate that compares one to the pair bar.

#### [FUSED-THRESHOLD] The pair admission bar

`admission.fused_threshold` (default 0.85) is the pair admission bar; provenance in [FUSION-TUNING-LEVERS]. It is per-pair data (`CandidatePair::fused_min_score`), not a global constant — a cross-language candidate with no structural anchor lowers it to `candidates.cross_language_min_jaccard`. Every threshold in these specs is a configurable default, never a hard-coded constant; the config surface lives in [exclusion.md](exclusion.md) and the migration in `unhardcode-tuning-plan.md`.

### [FUSION-SHARED-SUBTREE] `structural` is measured subtree overlap, not Merkle equality

`structural` is the **best-achievable ordered subtree overlap** between two occurrences: `1 - TED / max(nodes)`, where `TED` is the Zhang–Shasha tree edit distance over normalised node kinds with unit insert/delete/relabel costs (`overlap.rs`). Merkle-equal occurrences short-circuit to `1.0`, so every previously-`1.0` cluster is unchanged; what changes is the other end, which used to be a literal `0.0`.

That literal was a false negative by construction (gh #408). A single inserted statement rehashes every ancestor Merkle node, so a textbook Type-3 near-miss — full identifier rename plus one extra statement — scored `structural = 0.0` on the enclosing method while the unchanged statements *inside* it stayed Merkle-identical. The fragments were reported and the method was not, in **every** language: the pipeline held the evidence and discarded it. Measured on the five `*-type3` fixtures the enclosing pairs score 0.84–0.91, while their exact whole-method token Jaccard is only 0.74–0.85 — below `admission.fused_threshold`, which is why token evidence alone could never rescue them.

Overlap is an alignment, never a bag of matching subtree hashes. The discriminating information is the *order and nesting* of the matches, which is exactly what a multiset discards: two unrelated functions built from the same statement vocabulary share the same hashes as a genuine copy.

Endpoints above `overlap.rs::ALIGNMENT_MAX_NODES` (768 nodes) are too large to align, so their shared mass is estimated instead. The estimate walks both endpoints left to right, pairing identical subtrees, and never looks backwards — so the spans it pairs are disjoint and in the same order on both sides. Two rules make it safe, and both are load-bearing:

- **The pairing must respect order.** An alignment is ordered, so two subtrees matched in swapped order cannot both be kept: one has to be deleted and reinserted. A pairing chosen without regard to order is not an alignment at all, and crediting it reports shared mass no alignment reaches. Pinned by `the_fallback_never_credits_mass_no_ordered_alignment_can_reach` — two files holding the same two functions in swapped order, where an unordered pairing claimed 47 shared nodes against the alignment's 32.
- **Matched mass is not shared mass.** `structural` charges for everything left *un*matched on both sides; counting matched pairs alone ignores that charge. Pairing `m` node pairs costs at most `n₁ + n₂ − 2m` edits, so the shared mass it guarantees is `2m − min(n₁, n₂)` — and nothing at all when the match is small next to the endpoints, which is exactly what the alignment would have said.

Walking forward is also what stops a subtree nested inside an already-paired one from being counted twice (`the_fallback_never_credits_a_nested_right_subtree_twice`). The result is a conservative lower bound: it can suppress a rescue, but it cannot manufacture one (`the_large_tree_fallback_never_exceeds_the_alignment`), and a genuine large near-copy still clears the floor through it (`endpoints_past_the_alignment_cap_still_measure_as_shared`).

The cap counts nodes of the *normalised* tree, so [PIPELINE-NORMALIZE-AST-OPERATOR](pipeline.md) moved what it reaches without the number changing: operator tokens survive as leaves, and an operator-dense expression counts around half as many nodes again. At 512 that pulled `ts-mixed-band`'s ninety-term expression — 558 nodes, a consistent rename plus one redundant paren, exactly the case this section exists to rescue — onto the conservative bound, which scored it under the admission floor and reported **nothing**. The cap must reach the largest endpoint the admission path is expected to rescue, and is set above the largest pinned such case with room to spare rather than trimmed to it. Moving it is a performance decision, pinned deliberately by `the_alignment_cap_is_the_documented_operating_point`, whose companion assertion fails if the cap ever falls back below that measured case.

**Admission is a compound gate over two independently measured axes, not sum fusion, and it is applied pair by pair.** A pair below `admission.fused_threshold` is admitted only when overlap ≥ `admission.shared_subtree_min_overlap` **and** `token_jaccard` ≥ `admission.shared_subtree_min_jaccard` **and** both endpoints clear `admission.shared_subtree_min_node_count`. Neither axis admits alone — normalisation makes scaffolding Merkle-identical across unrelated files, so shape must be corroborated by tokens. This gate changes what is *measured*, never how the pair score is combined. Overlap is measured only on pairs that would otherwise be dropped yet carry the token corroboration, so the cost is bounded away from the ~596K-candidate admission set that [FUSION-CONTENT-GATE] deliberately avoids.

**Routing gains one row, and one comparison is retired.** [CLONE-BUCKETS-ROUTING] row 4b routes high overlap corroborated by an independent axis — the token axis at `admission.shared_subtree_min_jaccard`, **or** the embedding axis at `candidates.embedding_support_floor` — to `nearly_identical`, using the same floors that admitted the pair — so the pipeline can never admit a shared-subtree near-miss the renderer then hides. Row 4's old `structural ≤ 0.01` leg is gone: it predates the measurement, when any non-zero value meant a Merkle anchor, and additional shape evidence must never *hide* a cluster the token axis already carries. Clusters below the overlap floor keep the anchor-free demotion guard unchanged.

Because the value is now graded, **it is no longer comparable across two views of different scope**. A window nested inside a near-miss scores higher exactly to the extent that it excludes what differs, so [PIPELINE-CLUSTER-SUBSUME] compares grades only between views that do not nest; where one encloses the other, enclosure decides within a credibility tier. Pinned by `type3_enclosing_method.rs` in all five languages.

### [FUSION-SHARED-SUBTREE-MEMO] Overlap is memoised by ordered Merkle hash pair

One measurement per *structure pair*, never per byte-range pair. The memo key is the ordered pair of the two endpoints' Merkle hashes — the same premise the `1.0` short-circuit already stands on: hash equality pins the entire normalised structure, so every byte-offset copy of one structural pair shares the key, and the alignment runs once however many byte-range combinations LSH produced. The memoised value is exact — admission decisions and stored overlap values are identical to unmemoised measurement, only the alignment count changes. On the corpus that motivated it, 793,076 byte-range pairs collapsed onto a fraction as many structure pairs; without the memo the rescue never finished. Pinned by `a_fleet_of_identical_windows_costs_one_alignment` (36 byte-range pairs, exactly 1 alignment). Exact and bound results ([FUSION-SHARED-SUBTREE-BOUND]) are memoised separately, because a bound answers only the rescue question.

### [FUSION-SHARED-SUBTREE-BOUND] The kind-multiset bound refuses hopeless alignments

Shared mass never exceeds `min(smaller_total, kind-multiset intersection)`: any edit script maps a set of node pairs, and only kind-preserving pairs contribute to `larger − TED`. The rescue path (`rescue_overlap`) computes this bound first; when `bound / larger` is already below `admission.shared_subtree_min_overlap`, the quadratic alignment is skipped and the bound itself is returned. Sound because the bound never undercuts the alignment (`the_kind_multiset_bound_never_undercuts_the_alignment`), so a value below the floor proves the exact value is too — the admission decision is identical by construction (`the_rescue_path_agrees_with_the_exact_measure_on_admission`), and a refused pair pays for no alignment (`a_pair_the_bound_refuses_never_pays_for_an_alignment`). The bound applies only to the rescue's admission question; cluster signal measurement ([FUSION-CLUSTER-SIGNALS]) always uses the exact `overlap`, because a rendered `structural` value below the floor must still be the measured one.

### [FUSION-SHARED-SUBTREE-BOUND-ORDER] The order bound refuses alignments the multiset bound would allow

The kind-multiset bound counts *how many* nodes of each kind the two endpoints share. It cannot see where those nodes are, so two endpoints holding the same kinds in scrambled order look, to it, like a perfect match — and it pays for a full alignment to find out otherwise.

Order is available for free. An alignment maps one node before another on the left exactly when it maps their partners in the same order on the right, so the nodes it can match are a common *subsequence* of the two post-order kind sequences. The longest common subsequence is therefore a second upper bound on shared mass, never looser than the multiset one and usually far tighter. It is computed sixty-four positions per machine word (Allison–Dix), which makes it microseconds against the alignment's milliseconds, and the rescue runs it whenever the multiset bound fails to refuse a pair.

It applies at every endpoint size. Above the alignment cap the pair is answered by a lower bound on the alignment ([FUSION-SHARED-SUBTREE]), and this is an upper bound on the same quantity, so it dominates that too.

Soundness is what matters here, because a bound that ever *under*-states shared mass silently drops a real cross-file duplicate. It is pinned against a textbook longest-common-subsequence table at and around every machine-word boundary (`the_bit_parallel_row_matches_the_textbook_table`), against the real Zhang–Shasha result over 3,600 generated tree pairs (`the_bound_never_understates_what_the_alignment_measures`), and shown to be strictly tighter than the multiset bound on scrambled order (`scrambled_order_is_bounded_far_below_the_shared_multiset`). On the Flutter framework slice it removes 22% of the rescue's alignments while admitting exactly the same pairs.

### [FUSION-CLUSTER-SIGNALS] A cluster displays one admitted pair's measured evidence

A rendered cluster's signal triple is the measurement of **one admitted pair** — the strongest — never a mean over pairs. Baker's p-match is a per-pair predicate: a group qualifies because its pairs pass, and there is no group-level "average match" to display (Baker 1995, "On Finding Duplication and Near-Duplication in Large Software Systems"). Displayed evidence must therefore attach to the pair that earned it: the report names the elected pair (wire field `signal_source`), so every displayed number is traceable to the exact two occurrences that produced it (gh #458).

**Which pair is elected.** Of the admitted pairs — those that cleared the admission gate — the cluster elects the one with the highest fused confidence (its strongest single axis, bounded to [0,1]); ties resolve to the earliest pair in corpus order, making the election deterministic across runs (gh #301). All three axes render from that one pair, together: a per-axis best drawn from different pairs would display a "super-pair" no actual pair measured. Ensemble fusion research is explicit that the unit is the pair and the combination is max or sum, never average (arXiv:2510.15480, 2025); the same logic says the *pair*, not each axis, is the unit.

**The axes.** Per pair: `structural` is the measured shared-subtree overlap ([FUSION-SHARED-SUBTREE]) — `1.0` for Merkle-equal occurrences, the graded alignment otherwise; `token_jaccard` is the MinHash Jaccard estimate between the two signatures; `embedding_cos` is the cosine of the two vectors under the crate's single cosine definition ([FUSION-EMBED-PROVIDER]), so byte-identical occurrences — which share one vector — render exactly `1.0` (gh #372). A pair missing a signal input (no vector: embeddings off, oversized input, provider failure) renders `0.0` on that axis, the embeddings-off convention — absence never masquerades as a measured value.

**Only admitted pairs count.** Closure-only pairs — equal-hash combinations that never cleared admission — contribute nothing: they are artifacts of discovery topology (structural star buckets, ANN top-k fan-out, LSH band width), not of the rendered occurrences, and giving them a vote lets the deviant drag the verdict (Engler et al., "Bugs as Deviant Behavior", SOSP 2001: the majority outranks the deviant). gh #458 pinned: a byte-identical pair inside a lookalike cluster renders `1.0/1.0` and keeps its act-now bucket, while the lookalikes do not manufacture an identical verdict.

**The mean is dead.** The former per-pair mean over the closure component is removed. Under it, restored embedding evidence diluted a byte-identical file pair to `structural = 0.36` and routed it `same_behavior` instead of `identical` (gh #343 corpus, pinned by `issue_343_sum_clamp_saturation.rs`). The measured triple still feeds the cross-cluster subsumption pass, which compares structural values: diluted signals let contained artifact clusters escape collapse.

**For AI.** Election: `max over admitted pairs of (bounded_fused, Reverse(left), Reverse(right))` where bounded_fused = max(structural, token_jaccard, embedding_cos) clamped to [0,1], and left/right are corpus indices, so the lowest-index pair wins a fused tie. Rendered `PairScore` = the elected pair's own (structural, token_jaccard, embedding_cos), each `unwrap_or(0.0)` when the input is absent. `source_pair` = the elected pair's corpus indices; the wire field `ReportCluster.signal_source` holds their positions into `ReportCluster.occurrences`; `None` (all 0.0, no source) when every admitted pair's endpoint was collapsed by the same-file collapse (#339). Test pins: `the_rendered_triple_is_one_admitted_pairs_own_axes`, `non_admitted_pairs_never_contribute_to_the_rendered_signals`, `the_source_pair_election_is_deterministic`, `when_every_admitted_pair_skips_there_is_no_source_pair` (unit); `a_byte_identical_pair_reads_the_same_in_every_cluster` (E2E).

### [FUSION-CONTENT-GATE] Content agreement gates shape-identical confidence

`structural_sim` and `token_jaccard` are both computed from the *normalised*
representation (identifiers and literals collapsed), so on any exact shape
match they agree by construction: before gh #343 quarantined the sum their
total saturated the clamp, and even under the bounded max a shape match still
reads ≈1.0 while saying nothing about what the code actually said (gh #331,
#336). The gate restores an independent member by measuring what normalisation
erased:

1. For each cluster, walk each member's normalised subtree and hash the **raw
   source bytes** of every collapsed leaf, keeping the leaf's population
   (identifier vs literal position).
2. Measure two independent populations **for the elected pair** — the same
   pair [FUSION-CLUSTER-SIGNALS] elects for the shape axes, so every number
   on a cluster's signal row describes the same two occurrences — both in
   `[0, 1]`:
   - `agreement` — fraction of all collapsed positions whose raw bytes match,
     identifiers and literals pooled. Byte-identical members score 1.0;
     lightly-edited copies stay high; framework-mandated scaffolding (every
     name differs) and data tables (every literal differs) fall low.
   - `rename_consistency` — the Type-2 discriminator, [TECH-PMATCH-BAKER]
     quantified: the lesser of literal consistency (fraction of literal
     positions unchanged **or echoing an elected substitution**; vacuously
     1.0 with none) and rename-mapping coverage, scaled by the smooth
     anchor-mass weight `anchors / (anchors +
     content_gate.rename_evidence_half_mass)`, where anchors are the
     consistent literal positions plus the explained identifier positions.
     A literal *echo* ([REPAIR-RENAME-LITERAL-ECHO], #409) is a substituted literal position whose raw
     bytes transform into the partner's bytes exactly by one
     bijection-explained identifier substitution — `"OrderService"` →
     `"UserService"` renamed alongside its symbol is the rename done
     thoroughly, not evidence against it — and the echo corroborates that
     substitution the way a repeated identifier occurrence would, so
     completing a rename can never score below leaving it half-finished
     (`rename_literal_monotonicity.rs`).
     Coverage classifies each identifier position exactly as Baker's
     prev-encoding constrains it: raw-byte identity is a fixed-symbol
     match, explained by the position itself; a substitution is explained
     when it is bidirectionally modal *among the substituted pairs* —
     fixed symbols and parameters are disjoint alphabets, and collapsed
     leaves carry no role, so a homonym byte-string (a preserved property
     name that also names a renamed local) must not let one role veto the
     other in a single modal election — and corroborated by at least
     `content_gate.rename_corroboration_min` occurrences; positions the
     bijection cannot explain are constrained-but-unexplained and count
     against coverage; a *consistent substitution seen once* is an
     unconstrained first occurrence (`prev = 0` matches any other first
     occurrence) and belongs to neither numerator nor denominator — a
     renamed one-shot declaration name is not evidence against the clone.
     Zero without positional alignment. Consistency alone cannot tell a
     rename from sibling scaffolding that also substitutes names
     consistently — the anchors carry that burden, and they must *weigh*
     the proof, never gate it: the deleted
     `rename_evidence_min_literals` cliff zeroed every pair below four
     literal anchors, pricing a maximal one-literal Type-2 rename to
     `0.0588` — an agent-surface false negative
     (`type2_rename_anchor_floor.rs`).
     **A certified rename carries no doubt left for the mass term to
     price** (gh #410). When the lesser of literal consistency and
     coverage is exactly 1.0, every aligned literal is preserved or
     echoed and every constrained identifier position is byte-identical
     or a corroborated bijection substitution: the mapping is total,
     contradiction-free and literal-preserving, and the only doubt the
     anchor mass still prices is coincidence. Coincidence is discharged
     by mass, so the discount is dropped exactly where the mass term
     already vouches for the pair on its own — where
     `anchors / (anchors + content_gate.rename_evidence_half_mass)`
     reaches `content_gate.support_floor`, i.e. at ten anchors. There
     the weight is 1.0 and `rename_consistency` reads 1.0. Certification
     therefore never promotes a cluster the mass discount would have
     demoted; it only stops charging a proven rename for evidence it is
     not missing. Below that bar, and for any pair carrying a single
     contradiction, the smooth discount applies unchanged, so an
     anchor-poor forwarding scaffold (subject name twice plus one
     collaborator, mass 3, weight 3/7) stays below every routing floor.
     Because completing a rename can only raise consistency and add
     anchors, certification can only switch on — the
     [REPAIR-RENAME-LITERAL-ECHO] monotonicity property is preserved.
     Without it the axis was capped at
     `rename_consistency_discount × anchors / (anchors + 4)`, so a
     certified rename could never read 1.0 and **no Type-2
     rename cleared the act-now routing floor in any language**.
   Neither population is ever pooled with the other, and neither is averaged
   across a cluster's members: both are the elected pair's own measurement,
   the one rule [FUSION-CLUSTER-SIGNALS] states for every axis. Pooling them
   demoted textbook Type-2 clones to `structural_only` — a maximal rename
   scores low `agreement` and high `rename_consistency`, so the mean describes
   neither.

3. **Routing — three zones over `support = max(agreement,
   rename_consistency)`** (either population may vouch; never their mean).
   Below `content_gate.support_floor` (the Type-3 line; provenance in
   [FUSION-TUNING-LEVERS]), and with no semantic support (`embedding_cos` below
   `candidates.embedding_support_floor`, the line at which the embedding pass
   vouches for a cluster rather than merely having measured it), the cluster
   joins the [RANK-STRUCTURAL-ONLY] routing — surfaced honestly or hidden as cross-file
   scaffolding, and demoted in ranking. At or above
   `content_gate.promote_floor` (act-now grade) the cluster is a proven
   clone — a byte-agreeing near-miss
   or a consistent maximal rename — and routes `nearly_identical` even when
   the token layer lost its signature to the fingerprint-scoped fallback.
   Between the two, the legacy signal routing stands: real-world sibling
   families (the #197 REST settings surface measures 0.72–0.80) keep their
   demoted verdict.
4. **Token-signal correction.** A cluster whose members all carry **one**
   Merkle hash has normalised k-gram sets that are equal by construction;
   for such clusters routed `identical` / `nearly_identical` a lower rendered
   `token_jaccard` is a fallback-signature artifact and is corrected to 1.0
   (the GH #232 argument). `structural_only` keeps its unscored signal —
   absent token support is that bucket's defining signature.

   The correction is scoped by that digest equality, tested directly on the
   members, and by nothing else (gh #431). No reading of `structural` can
   stand in for it: since [FUSION-SHARED-SUBTREE] the axis grades subtree
   *overlap*, so it saturates by ratio as well as by hash equality, and every
   value below saturation means the subtrees provably differ. Scoping the
   correction to `content_gate.structural_saturation_floor` — a near-miss
   **routing** tolerance — published `token_jaccard = 1.0`, and the `shape`
   reading derived from it, across the whole `[0.99, 1.0)` band on no
   evidence. Routing tolerance is not
   proof of identity. Pinned by
   `crates/deslop/tests/content_gate_signal_honesty.rs`.
5. **Ranking reads none of this.** The report weight is the **sum** of
   duplicated mass — see [pipeline.md §RANK-MASS-SUM](pipeline.md#rank-mass-sum),
   which owns the formula — never a confidence factor, with no fused
   tie-break: at equal mass, cluster id makes the order total. Content
   evidence answers the binary question — is this a clone, and which bucket
   — never how heavily it weighs.

`token_jaccard` itself stays rename-invariant (normalised k-grams); the gate
adds evidence rather than redefining an existing signal.

**The token echo is shape evidence too.** The LSH pass hashes k-grams of the
same normalised kinds the structural pass hashes, so a near-total
`token_jaccard` (≥ `content_gate.saturating_token_floor`, the near-identical
routing line) saturates on shape
matches exactly as `structural` does — the surviving flutter/flutter #331
cluster read `structural=0.62, token_jaccard=0.98` because
transitive closure mixed structural and LSH pairs. The gate therefore fires on
*either* saturating signal.

**The gate stops at the anchor-free route.** Row 4 of
[taxonomy.md §CLONE-BUCKETS-ROUTING](taxonomy.md#clone-buckets-routing) is
deliberately outside it. Both populations below assume the members align
position for position, and `structural ≤ 0.01` says the shapes differ — so
against a genuine Type-3 clone whose identifiers are all renamed and whose
bodies differ by one statement (`csharp-type3`), agreement collapses to the
literals (0.19) and rename consistency to 0.00, because the extra statement
destroys the alignment the rename proof needs. Gating row 4 here would demote
the renamed near-miss, the most valuable clone class there is. Row 4 is routed
on cluster *spread* instead — see the taxonomy row. Shape-mismatched members have no positional
alignment, so their agreement is the key-set Jaccard of their content keys — a
genuine Type-3 near-miss shares nearly all of them; renamed scaffolding shares
few. The verbatim guard is proportional and
exclusive: one *token-identical family* — members sharing both the same
normalised-subtree digest and the same collapsed-leaf keys — must hold a
strict majority of the cluster (above `content_gate.verbatim_member_share_floor`).
A verbatim pair among a couple of lookalikes (#104, share 2/3) still vouches
for its cluster; two copied example widgets inside a 453-member framework
family (0.4%) do not; and two *disjoint* identical pairs, each at exactly one
half, vouch for nothing, because neither is a majority and the members they
disagree with are the whole rest of the cluster. `data`-category
clusters are exempt from the structural-only ranking demotion — their weight
belongs to the `[ranking] data_clones` policy ([RANK-CATEGORY]) so
`data_clone_weight = 1.0` can still restore a table the gate routed to the
structural-only bucket.

### [FUSION-ALGEBRA] Every calculation, as algebra

The whole arithmetic surface in one place — one formula per block, the English right under it. Each block carries the spec id that owns it; if prose and algebra ever disagree, both are wrong — fix them together.

**Symbols.** `S` structural, `J` token_jaccard, `E` embedding_cos, `A` agreement, `R` rename_consistency, `n` node count, `p` a candidate pair, `c` a cluster, `ℓ` a source line.

**Pair admission** — [FUSION-STRATEGY-BOUNDED-MAX] [FUSED-THRESHOLD] [FUSION-SHARED-SUBTREE]

Structural similarity is the shared-node share of the bigger tree. The aligner is Zhang–Shasha's tree-edit-distance recurrence (keyroot decomposition over post-order sequences of normalised node kinds, unit insert/delete/relabel costs — Zhang & Shasha 1989), and under unit costs the distance is exactly the unmatched node mass, so shared = max − TED and the two spellings agree. Hash-then-TED is the shape Baxter et al. 1998 established: exact hashes cluster, tree edit distance grades the near-miss. The normalisation is ours.

$$
\mathrm{shared}(a,b) = \max\bigl(n(a), n(b)\bigr) - \mathrm{TED}(a,b) \qquad
S(a,b) = \frac{\mathrm{shared}(a,b)}{\max(n(a),\, n(b))} = 1 - \frac{\mathrm{TED}(a,b)}{\max(n(a),\, n(b))}
$$

Merkle-equal pairs score 1.0 without paying for the walk; past the alignment cap a credited shared-node count — a sound lower bound on the aligned value — answers instead, and the share clamps to `[0, 1]`.

The fused score takes the strongest of the three signals and clamps to [0, 1]. Max — never an average — so one loud signal is never diluted by two quiet ones.

$$
\mathrm{fused}(p) = \operatorname{clamp}(\max(S(p),\, J(p),\, E(p)),\ 0,\ 1) \qquad \text{(max-or-sum, never average — arXiv:2510.15480)}
$$

The bar a pair must clear depends on the pair. Cross-language pairs with no structural signal can't use `S`, so they fall back to a token-only floor. Everything else uses the configured `fused_threshold` (default 0.85), which was derived from our corpus and clears SourcererCC's 0.7 overlap floor.

$$
t(p) = \begin{cases} \text{cross\_language\_min\_jaccard} & \text{if cross-language}(p) \land S(p) \le 0 \\ \text{fused\_threshold (default 0.85)} & \text{otherwise} \end{cases}
$$

`J` estimates the Jaccard of the two k-gram sets of normalised node kinds (Jaccard 1912). Broder's min-wise identity makes that estimable by hashing: for a min-wise independent family, the probability the two minima agree is exactly the Jaccard.

$$
J(A, B) = \frac{|A \cap B|}{|A \cup B|} \qquad\qquad \Pr_{h \in \mathcal{H}}\Bigl[\,\arg\min_{a \in A} h(a) = \arg\min_{b \in B} h(b)\Bigr] = J(A, B)
$$

The shipped estimator averages agreement over the `m = representation.minhash_signature_len` blake3-hashed slots (`lsh::estimate_jaccard`); LSH banding follows the standard `BANDS × ROWS_PER_BAND` collision curve.

$$
\hat{J}(A, B) = \frac{1}{m} \sum_{i=1}^{m} \mathbf{1}\bigl[\sigma_A(i) = \sigma_B(i)\bigr]
$$

A pair is admitted when its fused score clears the threshold, or when the shared-subtree escape fires: structural overlap and token overlap both strong, and both sides big enough to be worth counting.

$$
\begin{aligned}
\mathrm{admit}(p) \iff {}& \mathrm{fused}(p) \ge t(p) \\
&\lor \bigl( S(p) \ge \text{shared\_subtree\_min\_overlap} \land J(p) \ge \text{shared\_subtree\_min\_jaccard} \\
&\qquad \land\ n(\text{left}) \ge \text{shared\_subtree\_min\_node\_count} \land n(\text{right}) \ge \text{shared\_subtree\_min\_node\_count} \bigr)
\end{aligned}
$$

The guard rescues pure-token pairs. When both structural and embedding evidence are dead (`S` and `E` at zero), MinHash agreement alone still admits — but only above `lsh_only_min_jaccard`, and only when the smaller side has enough nodes.

$$
\mathrm{guard}(p):\ S(p) \le 0 \land E(p) \le 0 \implies J(p) \ge \text{lsh\_only\_min\_jaccard} \land \min(n(\text{left}),\, n(\text{right})) \ge \text{lsh\_only\_min\_node\_count}
$$

**Election** — [FUSION-CLUSTER-SIGNALS]

Every admitted pair in a cluster is a candidate; the cluster elects the strongest — highest fused score first, ties broken by the earlier left position, then the earlier right. Lexicographic ordering makes the winner identical on every machine, every run.

$$
p^*(c) = \max_{\text{admitted pairs}} \bigl(\mathrm{fused},\ -\text{left},\ -\text{right}\bigr) \qquad \text{(lexicographic, deterministic)}
$$

The report shows the elected pair's three signals and cites its positions as the evidence source. Nothing else in the cluster is quoted.

$$
\mathrm{rendered}(c) = (S, J, E) \text{ of } p^*(c) \qquad \text{signal\_source = positions of } p^*(c)
$$

**Content evidence** (the elected pair) — [FUSION-CONTENT-GATE]

Agreement compares the aligned collapsed-leaf keys. Position counts equal, it is the positional match share; counts differ, the shapes cannot align and it falls back to the key-set Jaccard — both branches are Jaccard comparisons ([FUSION-CONTENT-GATE]).

$$
A = \begin{cases} \dfrac{\text{matching aligned positions}}{\text{all aligned positions}} & \text{if position counts are equal} \\[8pt] \dfrac{|K_a \cap K_b|}{|K_a \cup K_b|} & \text{otherwise (key-set Jaccard)} \end{cases}
$$

Rename mass discounts anchors that could be explained away as a rename: each unit of rename evidence costs half a mass point.

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
\mathrm{support} = \max(A, R) \qquad \text{(either population may vouch; never a mean, never pooled — arXiv:2510.15480)}
$$

**Routing** — [FUSION-CONTENT-GATE] [CLONE-BUCKETS-ROUTING]

Three exits. Weak support plus weak embeddings routes structural-only — the content signals had their say and lost. Strong support promotes the cluster to nearly_identical outright. Everyone else falls through to the legacy per-signal routing.

$$
\begin{aligned}
\mathrm{support} < \text{support\_floor} \land E < \text{embedding\_support\_floor} &\implies \text{structural\_only routing} \\
\mathrm{support} \ge \text{promote\_floor} &\implies \text{nearly\_identical} \\
\text{otherwise} &\implies \text{legacy signal routing}
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

Clusters sort heaviest first; ties break by cluster id ascending so the report order is stable.

$$
\text{order} = \text{weight descending, then cluster id ascending}
$$

*A duplicate is a duplicate: content evidence decides whether a cluster is reported and which bucket it wears — never how heavily it weighs.*

**Repo metrics** — [METRICS-REPO] [METRICS-REPO-WEIGHTED]

The headline number is unweighted duplicated-line density — directly comparable to SonarQube's gate.

$$
\text{duplication\_percent} = \operatorname{clamp}\!\left(\frac{100 \times \text{duplicated\_loc}}{\text{analysed\_loc}},\ 0,\ 100\right) \qquad \text{(unweighted duplicated-line density — the SonarQube comparable gate)}
$$

A line's weight is the heaviest bucket of any cluster covering it — a line duplicated by a critical cluster counts as critical.

$$
\text{line\_weight}(\ell) = \max_{\text{clusters covering } \ell}\bigl(\text{bucket\_weight} \times \text{category\_weight}\bigr)
$$

The weighted variant sums those per-line weights. Specified for completeness, not shipped.

$$
\text{weighted\_percent} = \operatorname{clamp}\!\left(\frac{100 \times \sum_{\ell} \text{line\_weight}(\ell)}{\text{analysed\_loc}},\ 0,\ 100\right) \qquad \text{(specified, not shipped)}
$$

Weighting can only pull lines *into* the duplicated set, never out — so the weighted figure can never undercut the unweighted one.

$$
\text{weighted\_percent} \le \text{duplication\_percent} \qquad \text{(invariant)}
$$

**Provenance.** The literature behind the formula lines (links in [reading-list.md](reading-list.md)); quoted where the claim is verbatim:

| Decision | Source | Verified claim |
|---|---|---|
| Fingerprinted structural axis | Chilowicz et al. 2009 | *"each node of an AST is associated with a fingerprint based on a hash value (incrementally computed) of the subtree rooted at the node"* |
| Exact hash, TED near-miss extension | Baxter et al. 1998 | hash AST subtrees, cluster by hash, extend to near-miss via tree edit distance |
| The `TED` recurrence (unit costs, post-order keyroot decomposition) | Zhang & Shasha 1989 | the textbook ordered-tree edit distance the aligner implements; under unit costs shared = max − TED, making both spellings of `S` exact |
| The Jaccard target and the min-wise identity behind `Ĵ` | Jaccard 1912; Broder 1997 | J = \|A∩B\|/\|A∪B\|; the minimum-hash agreement probability equals the Jaccard |
| `fused` / `support` = max; the pair is the unit; never average | arXiv:2510.15480 | the unit is the pair and the combination is max or sum, never average |
| `fused_threshold` default above the overlap floor | SourcererCC (Sajnani et al. 2016) | candidate test is \|X∩Y\|/min(\|X\|,\|Y\|) ≥ 0.7 — containment of the smaller bag, not a Jaccard; 0.85 clears it with margin |
| Routing separates evidence classes — no confidence scaling of findings | Svajlenko & Roy 2015 | BigCloneBench syntactic-similarity bands (VST3 ≥ 0.90, ST3 0.70–0.90, MT3 0.50–0.70, WT3/4 < 0.50); tool precision and recall degrade monotonically as similarity falls, and `promote_floor` 0.85 sits inside the ST3 band |
| Shape-only repetition demoted, never a failing verdict | Kapser & Godfrey 2008 | shape-level repetition is the weakest ground for a failing verdict |
| Repo percentage as the comparable gate | SonarQube metrics | the industry-standard CI gate is unweighted duplicated-line density |

Content-evidence arithmetic (`A`, `R`, the asymptotic weight, the routing floors) and the mass-sum weight are **derived or defect fixes**, not literature — their provenance rows in [FUSION-TUNING-LEVERS] say so.

Every numeric constant above is a configurable default, never a hard-coded value — provenance in [FUSION-TUNING-LEVERS], surface in [exclusion.md](exclusion.md), migration in `unhardcode-tuning-plan.md`.

### [FUSION-TUNING-LEVERS] Every threshold is a configuration item with a recorded provenance

A number is a **lever** when changing it changes which clusters are reported, which bucket they land in, or how they rank. Every lever is named, defaulted to the value compiled today, range- and invariant-validated at load ([EXCLUSION-CONFIG] `[tuning]`), and declared in the report that its value produced ([CONFIG-TUNING-DECLARED]).

**Unhardcoding is behaviour-preserving.** A run with no `[tuning]` section, no `--tune` flag, and no editor override produces a byte-identical report to the pre-migration build on every fixture and every corpus repository. Changing a *default* is a separate change with its own failing test, its own provenance entry, and its own corpus measurement — widening one during the migration is how an unhardcoding refactor becomes an undetected recall loss.

**Provenance is part of the spec.** A threshold with no recorded justification is an unfalsifiable claim, so each default carries one of four kinds: **literature** (a published operating point, cited by its [TECH-*] id), **defect** (an observed false positive or negative, cited by issue — it says what the value must *not* admit, which beats a curve), **derived** (follows from the fusion algebra or another lever, with the derivation stated), or **unrecorded** (a tracked gap, not a resting state — each earns a citation, a defect, or a measured sweep).

| Key | Site | Default | Provenance |
| --- | --- | --- | --- |
| `admission.fused_threshold` | `pair.rs:31` | 0.85 | **Derived.** Under bounded max one axis alone can carry a pair, so the bar on that axis rises to compensate. SourcererCC's 0.7 is overlap similarity, not Jaccard ([TECH-TOKEN-SOURCERERCC]); Deslop sits above the stricter reading. Not an ROC sweep. |
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
| `candidates.embedding_support_floor` | `pair.rs:91` | 0.80 | **Derived** (#356). The cosine at which a measured `embedding_cos` is the embedding pass *vouching for* a cluster rather than merely having measured it — the ANN candidate gate's own operating point, and the line [CLONE-BUCKETS-ROUTING] row 2 lets semantic evidence carry a bucket alone. The [FUSION-CONTENT-GATE] escape is judged against it. |
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
The deterministic BLAKE3 stub embedding provider named in [FUSION-EMBED-PROVIDER]
exists purely so E2E tests can exercise the embedding path without a live model.
It lives behind the `test-support` Cargo feature, is **never** registered in
`ProviderRegistry::production`, and is barred from the shipped VSIX by a packaging
gate. `[REMOVE-STUB]` tags the code sites that enforce this boundary so a grep
proves the stub cannot leak into a release; any new stub-touching code must carry
the tag and stay test-only.
