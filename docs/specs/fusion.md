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
4. Cluster pairs by transitive closure above `admission.fused_threshold` ([FUSION-TUNING-LEVERS]).
5. Weight each cluster by the ranking formula in §4 for "worst offenders first."

This way, a Type-1 clone scores ≈1 on all three signals, a Type-2 ≈1 on structural+embedding and ~high on LSH, a Type-3 may score high on LSH+embedding and medium on structural, and a Type-4 scores primarily on embedding. Every type lands in the report; scores explain *why*, and the fused confidence never exceeds the best of them. Rendered confidence is defined by [FUSION-CONTENT-GATE]: for shape-saturating clusters the gate substitutes measured content evidence for this function's implicit 1.0 content factor; everywhere else the bounded max **is** the rendered value.

### [FUSION-CLUSTER-SIGNALS] Rendered cluster signals are measured, never aggregated from discovery edges

A rendered cluster's signal triple is **measured between the occurrences the report shows**: the per-signal mean over every unordered pair of rendered occurrences. Per pair: `structural` is Merkle-hash equality (1.0 or 0.0), `token_jaccard` is the MinHash Jaccard estimate between the two signatures, and `embedding_cos` is the cosine of the two vectors, computed by the crate's single cosine definition ([FUSION-EMBED-PROVIDER]): dot product and norms accumulated in `f64` over the raw `f32` components, clamped to [0,1], so byte-identical occurrences — which share one vector — render exactly `1.0` (gh #372). A pair where either signal input is missing (no vector: embeddings off, oversized input, provider failure) contributes to neither that signal's numerator nor its denominator; a signal with no measurable pair reports 0.0, matching the embeddings-off convention, with the absence explained by the report's embedding provenance.

Averaging the surviving pair scores of the transitive-closure component is prohibited. Closure admits every edge above threshold, so the edge mix is an artifact of discovery topology — structural star buckets, ANN top-k fan-out, LSH band width — not of the rendered occurrences. Under that mean, restored embedding evidence diluted a byte-identical file pair to `structural = 0.36` and routed it `same_behavior` instead of `identical` (gh #343 corpus, pinned by `issue_343_sum_clamp_saturation.rs`). The measured triple also feeds the cross-cluster subsumption pass, which compares structural values: diluted signals let contained artifact clusters escape collapse.

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
2. Measure two independent populations per member against the canonical
   member, both in `[0, 1]`:
   - `agreement` — fraction of all collapsed positions whose raw bytes match,
     identifiers and literals pooled. Byte-identical members score 1.0;
     lightly-edited copies stay high; framework-mandated scaffolding (every
     name differs) and data tables (every literal differs) fall low.
   - `rename_consistency` — the Type-2 discriminator, [TECH-PMATCH-BAKER]
     quantified: the lesser of literal preservation (fraction of literal
     positions unchanged; vacuously 1.0 with none) and rename-mapping
     coverage, scaled by the smooth anchor-mass weight `anchors / (anchors
     + content_gate.rename_evidence_half_mass)`, where anchors are the
     preserved literal positions plus the explained identifier positions.
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
     literal anchors and rendered a maximal one-literal Type-2 rename at
     `fused = 0.0588`, an agent-surface false negative
     (`type2_rename_anchor_floor.rs`).
   A maximally renamed clone of real logic scores low pooled `agreement` but
   high `rename_consistency` — every renamed name repeats, so nearly every
   position is a corroborated anchor; pooling the populations into one mean
   is what demoted textbook Type-2 clones to `structural_only`.
3. **Rendered confidence**: for shape-identical clusters not proven
   byte-equivalent, `fused = max(embedding_cos, max(structural, token_jaccard)
   × max(agreement, rename_consistency_discount × rename_consistency))`. The
   discount reflects that mapping-explained identifier positions are strictly
   weaker evidence than byte equality, keeping a proven rename in the act-now
   band while reserving `fused = 1.0` for byte-proven duplication. LSH-only and
   embedding-discovered pairs render the bounded max fusion unchanged — the
   same formula with the content factor at its implicit 1.0.
4. **Routing — three zones over `support = max(agreement,
   rename_consistency)`** (either population may vouch; never their mean).
   Below `content_gate.support_floor` (the [TECH-TOKEN-SOURCERERCC] Type-3
   overlap cutoff), and with no semantic support (`embedding_cos` below
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
5. **Token-signal correction.** A shape-identical cluster shares one Merkle
   hash, so its members' normalised k-gram sets are equal by construction;
   for clusters routed `identical` / `nearly_identical` a lower rendered
   `token_jaccard` is a fallback-signature artifact and is corrected to 1.0
   (the GH #232 argument). `structural_only` keeps its unscored signal —
   absent token support is that bucket's defining signature.
6. **Ranking.** The content-gated `fused` scales the final report weight as a
   continuous factor alongside the [RANK-CATEGORY] and
   [RANK-STRUCTURAL-ONLY] bucket multipliers: at equal geometry a byte-proven
   copy outranks a consistent rename, which outranks shape-only coincidence,
   and two same-bucket clusters rank by how much of their content agrees.

`token_jaccard` itself stays rename-invariant (normalised k-grams); the gate
adds evidence rather than redefining an existing signal.

**The token echo is shape evidence too.** The LSH pass hashes k-grams of the
same normalised kinds the structural pass hashes, so a near-total
`token_jaccard` (≥ `content_gate.saturating_token_floor`, the near-identical
routing line) saturates on shape
matches exactly as `structural` does — the surviving flutter/flutter #331
cluster read `structural=0.62, token_jaccard=0.98, fused=1.00` because
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
few. The verbatim guard is proportional
(`content_gate.verbatim_member_share_floor` of the members must participate
in byte-identical duplicates): a verbatim pair among a couple of lookalikes
(#104) still vouches for its cluster, but two copied example widgets inside a
453-member framework family (0.4%) do not. `data`-category
clusters are exempt from the structural-only ranking demotion — their weight
belongs to the `[ranking] data_clones` policy ([RANK-CATEGORY]) so
`data_clone_weight = 1.0` can still restore a table the gate routed to the
structural-only bucket.

### [FUSION-TUNING-LEVERS] Every threshold is a configuration item with a recorded provenance

A number is a **lever** when changing it changes which clusters are reported, which bucket they land in, or how they rank. Every lever is named, defaulted to the value compiled today, range- and invariant-validated at load ([EXCLUSION-CONFIG] `[tuning]`), and declared in the report that its value produced ([CONFIG-TUNING-DECLARED]).

**Unhardcoding is behaviour-preserving.** A run with no `[tuning]` section, no `--tune` flag, and no editor override produces a byte-identical report to the pre-migration build on every fixture and every corpus repository. Changing a *default* is a separate change with its own failing test, its own provenance entry, and its own corpus measurement — widening one during the migration is how an unhardcoding refactor becomes an undetected recall loss.

**Provenance is part of the spec.** A threshold with no recorded justification is an unfalsifiable claim, so each default carries one of four kinds: **literature** (a published operating point, cited by its [TECH-*] id), **defect** (an observed false positive or negative, cited by issue — it says what the value must *not* admit, which beats a curve), **derived** (follows from the fusion algebra or another lever, with the derivation stated), or **unrecorded** (a tracked gap, not a resting state — each earns a citation, a defect, or a measured sweep).

| Key | Site | Default | Provenance |
| --- | --- | --- | --- |
| `admission.fused_threshold` | `pair.rs:31` | 0.85 | **Derived.** Under bounded max one axis alone can carry a pair, so the bar on that axis rises to compensate. [TECH-TOKEN-SOURCERERCC] treats Jaccard ≥ 0.7 as the typical Type-3 cutoff; Deslop sits higher for that reason. Not an ROC sweep. |
| `admission.lsh_only_min_jaccard` | `pair.rs:36` | 0.90 | **Defect.** Not a similarity threshold — a guard. LSH-only pairs have no structural anchor, and tiny `using`/`namespace` sibling windows hit Jaccard ≈ 1.0 by accident, then merge into a mega-cluster through transitive closure. |
| `admission.lsh_only_min_node_count` | `pair.rs:43` | 40 | **Defect.** The same defect's other half, applied at both endpoints: an 18-node k-gram set is mostly grammar scaffolding, so tens of thousands of such subtrees agree by accident. |
| `admission.max_endpoint_node_ratio` | `pair.rs:61` | 4 | **Defect** (#368). [PAIR-SIZE-COHERENCE] — an embedding-only pair scored a 19-node parameter list against a 274-node arithmetic chain at cosine 1.00. Deliberately loose; fires only where the pair is self-contradictory. |
| `candidates.cross_language_min_jaccard` | `pair.rs:66` | 0.10 | **Derived.** Cross-language AST vocabularies differ and the mode is opt-in ([CONFIG-CROSS-LANGUAGE]), so the floor sits below the same-language LSH-only floor. |
| `candidates.embedding_min_cosine` | `embedding/pairs.rs:27` | 0.80 | **Literature.** SSCD's published operating point, and a candidate-set gate only — `fused_threshold` still decides admission downstream. |
| `candidates.embedding_top_k` | `embedding/pairs.rs:16` | 5 | **Unrecorded.** The stated rationale — recall comes from the union, not the ANN fan-out — argues for *small*, not for *five*. |
| `candidates.embedding_exact_pair_limit` | `embedding/pairs.rs:22` | 256 | **Unrecorded.** |
| `content_gate.support_floor` | `buckets.rs:237` | 0.7 | **Literature** (#341). [TECH-TOKEN-SOURCERERCC] Type-3 overlap cutoff. |
| `content_gate.promote_floor` | `buckets.rs:248` | 0.85 | **Derived** (#341). The act-now grade, matched to `fused_threshold`; bounded below by a defect — the #197 REST settings family measures 0.72–0.80 and must keep its demoted verdict. |
| `content_gate.structural_only_max_support` | `buckets.rs:215` | 0.05 | **Defect.** #197's acceptance criterion (`token_jaccard = 0.00`, `embedding_cos = 0.00`) plus tolerance for MinHash collision noise. It is a ceiling below which a signal counts as *absent*, and is never a support floor — `route_shape_identical` read it as one, so a cosine of 0.05 overruled the measured content evidence and the gate's verdict followed whether the embedding pass ran (#356). |
| `candidates.embedding_support_floor` | `pair.rs:91` | 0.80 | **Derived** (#356). The cosine at which a measured `embedding_cos` is the embedding pass *vouching for* a cluster rather than merely having measured it — the ANN candidate gate's own operating point, and the line [CLONE-BUCKETS-ROUTING] row 2 lets semantic evidence carry a bucket alone. The [FUSION-CONTENT-GATE] escape is judged against it. |
| `content_gate.saturating_token_floor` | `buckets.rs:291` | 0.95 | **Defect** (#368). The surviving flutter/flutter #331 cluster read `structural = 0.62, token_jaccard = 0.98` — the token layer echoing shape, not reporting content. |
| `content_gate.rename_consistency_discount` | `buckets.rs:301` | 0.9 | **Derived** (#346). Keeps a proven Type-2 rename above `fused_threshold` while reserving `fused = 1.0` for byte-proven duplication. |
| `content_gate.rename_corroboration_min` | `content.rs` | 2 | **Literature.** [TECH-PMATCH-BAKER] prev-encoding: a parameter symbol's first occurrence matches anything and constrains nothing; only repetition carries binding proof. |
| `content_gate.rename_evidence_half_mass` | `content.rs` | 4 | **Defect.** Replaces the `rename_evidence_min_literals = 4` cliff (#346), which zeroed sub-floor rename evidence and rendered a maximal one-literal Type-2 rename at `fused = 0.0588` (`type2_rename_anchor_floor.rs`). Same operating point, now a half-saturation mass: a forwarding echo's single substitution (mass 2, weight 1/3) stays below every routing floor while a 16-anchor maximal rename clears the reuse line. |
| `content_gate.verbatim_member_share_floor` | `content.rs:54` | 0.5 | **Defect** (#341, tightened #346). #104's verbatim pair among lookalikes (share ≥ 2/3) must stay visible; two byte-identical widgets inside 453 framework declarations (≈ 0.004) must not vouch for the family. |
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
