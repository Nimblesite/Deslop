# Pipeline design — hybrid, not pure-RAG

Deslop combines structural, token, and embedding analysis. The surveyed systems in [landscape.md](landscape.md) and [reading-list.md](reading-list.md) likewise combine representations rather than relying on vector search alone.

### [FUSION-SIGNALS-THREE-LAYER] Deslop is hybrid by design

The pipeline fuses three signals:

1. **Structural (AST fingerprinting)** — Merkle-hash every tree-sitter subtree after normalization. Catches Type-1, Type-2, most Type-3. Fast, deterministic, gives exact byte ranges. (Chilowicz 2009 + Baxter 1998.)
2. **Token LSH (MinHash over normalized k-grams)** — catches Type-3 cases where structure diverged but token bag is close. Fast, deterministic. (SourcererCC 2016.)
3. **Learned embeddings (local, via Ollama)** — catches Type-3/Type-4 the structural passes miss. Used both as a **recall expander** (find candidates the hash-based passes didn't cluster) and as a **re-ranker** (promote semantically-similar AST clusters in the final score). (SSCD 2024, ensemble-LLM 2025.)

All three run by default. The research doesn't support shipping without embeddings — it's a measurable accuracy loss on Type-3/4.

### [FUSION-EMBED-PROVIDER] Embedding layer — concrete choices

- **Provider and model are not hard-coded.** Both are CLI flags (`--embedding-provider`, `--embedding-model`), backed by config file and env var fallbacks. The core crate exposes an `EmbeddingProvider` trait; production providers are selected at runtime by name through `ProviderRegistry::production`, which registers `ollama` only today. A local `onnx` provider is on the roadmap and slots in by registering another factory — no transport-specific special-casing. A deterministic BLAKE3 stub provider exists purely as test infrastructure: it lives behind the `test-support` Cargo feature, is never registered in production, and is barred from the shipped VSIX by a packaging gate. The research picks a *default*, not a lock-in.
- **Default provider + model (overridable).** Provider defaults to `ollama` (local, no network) and model defaults to `nomic-embed-code` — these are recommended starting points from the 2025 ensemble paper's finding that *"smaller embedding sizes, smaller tokenizer vocabularies and tailored datasets are advantageous"*. CodeT5+110M and UniXCoder are alternate top performers cited in the literature; either should be selectable via `--embedding-model` once exposed through a provider.
- **Local-only is a policy, not a hard requirement of the architecture.** The default stack never touches the network, but the trait doesn't forbid a hosted provider. A user configuring `--embedding-provider=hosted-foo` opts into that tradeoff deliberately; we don't enable it for them.
- **ANN index: HNSW.** Use `usearch` or `instant-distance` (pure Rust, no C deps). SSCD validated HNSW at 250 MLOC.
- **Ensemble by max, never sum or average.** The 2025 ensemble paper's max/sum finding assumes independent members; Deslop's structural and token axes are two views of one normalised tree, so summing them manufactures confidence neither carries alone (gh #343). Fusion takes the strongest single axis; score normalization is mandatory before combining.
- **Cache by `(file_content_hash, provider_id, model_id, model_version)`.** Re-runs are free; switching providers or models invalidates only the embedding layer and leaves structural/LSH caches intact. LSP incremental mode reuses the same cache unchanged.
- **Index granularity: AST subtrees above min-node threshold**, not whole files. We already have those subtrees from the structural pass — embed them directly. This keeps embeddings byte-range-addressable and dramatically reduces the N in k-NN.
- **The per-input character budget belongs to the provider, never to the pipeline.** A subtree longer than the budget is counted in `failed_subtrees` and never dispatched, because Ollama truncates silently (`truncate: true`) and a truncated vector misrepresents the code it claims to describe. The budget is therefore a property of the model behind the provider — `nomic-embed-text` carries a 2,048-token context, `mxbai-embed-large` 512, `qwen3-embedding` 32k — and is read from `EmbeddingProvider::max_input_chars`. `OllamaProvider` derives it at construction from the model's own `model_info["<arch>.context_length"]` via `POST /api/show`, converted at a deliberately conservative 3 chars/token, falling back to `DEFAULT_MAX_INPUT_CHARS` (6,000) when the endpoint or field is unavailable. A single pipeline-wide constant cannot be correct for two models an order of magnitude apart: gh #286 reported 14,723 of 175,160 subtrees (8.4%) dropped, at the large end where re-derived duplication is most expensive to miss, and no model swap could have recovered them while the cap sat upstream of the provider.
- **Determinism caveat.** Embedding + ANN is approximate. Mitigate by: (a) recording `provider_id`, `model_id`, and `model_version` in the `.deslop/cache` header and the report, (b) using deterministic ANN parameters (fixed seed, fixed ef_construction), (c) final ranking is still computed over the *union* of structural + LSH + embedding candidates, so a missed ANN neighbor only loses recall, never changes existing cluster content.

### [FUSION-STRATEGY-MAX-SUM] Fusion strategy (how the three signals combine)

The ID records the strategy this section originally specified; the **sum arm was removed by gh #343** (pinned by `issue_343_sum_clamp_saturation.rs`; `PairScore::bounded_fused` is the only fusion) because the axes are correlated views of one normalised tree and their sum clamps mid-band clusters to a confidence of 1.0 that no single axis earned and no byte-identical pair backs. The strategy in force:

1. Compute a candidate set of clone pairs as the **union** of: structural-hash matches, LSH bucket collisions, and top-k embedding neighbors per subtree.
2. For each candidate pair, compute three scores in [0,1]: `structural_sim`, `token_jaccard`, `embedding_cos`.
3. Final pair score = the **strongest single axis** — `max(structural_sim, token_jaccard, embedding_cos)`, bounded to [0,1] (`PairScore::bounded_fused`). Never their sum, never their average.
4. Cluster pairs by transitive closure above a threshold.
5. Weight each cluster by the ranking formula in §4 for "worst offenders first."

This way, a Type-1 clone scores ≈1 on all three signals, a Type-2 ≈1 on structural+embedding and ~high on LSH, a Type-3 may score high on LSH+embedding and medium on structural, and a Type-4 scores primarily on embedding. Every type lands in the report; scores explain *why*, and the fused confidence never exceeds the best of them. Rendered confidence is defined by [FUSION-CONTENT-GATE]: for shape-saturating clusters the gate substitutes measured content evidence for this function's implicit 1.0 content factor; everywhere else the bounded max **is** the rendered value.

### [FUSION-CLUSTER-SIGNALS] Rendered cluster signals are measured, never aggregated from discovery edges

A rendered cluster's signal triple is **measured between the occurrences the report shows**: the per-signal mean over every unordered pair of rendered occurrences. Per pair: `structural` is Merkle-hash equality (1.0 or 0.0), `token_jaccard` is the MinHash Jaccard estimate between the two signatures, and `embedding_cos` is the cosine of the two vectors computed by the same arithmetic the ANN pass uses ([FUSION-EMBED-PROVIDER]), including its [0,1] clamp. A pair where either signal input is missing (no vector: embeddings off, oversized input, provider failure) contributes to neither that signal's numerator nor its denominator; a signal with no measurable pair reports 0.0, matching the embeddings-off convention, with the absence explained by the report's embedding provenance.

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
   - `rename_consistency` — the Type-2 discriminator: the lesser of literal
     preservation (fraction of literal positions unchanged) and bijective
     identifier-mapping coverage (fraction of identifier positions explained
     by one consistent 1:1 substitution, modal in both directions). Zero
     without positional alignment or with fewer than 4 literal anchors —
     without anchors, a consistent mapping cannot tell a rename from sibling
     scaffolding that also substitutes names consistently.
   A maximally renamed clone of real logic scores low pooled `agreement` but
   `rename_consistency ≈ 1.0`; pooling the populations into one mean is what
   demoted textbook Type-2 clones to `structural_only`.
3. **Rendered confidence**: for shape-identical clusters not proven
   byte-equivalent, `fused = max(embedding_cos, max(structural, token_jaccard)
   × max(agreement, 0.9 × rename_consistency))`. The 0.9 discount reflects
   that mapping-explained identifier positions are strictly weaker evidence
   than byte equality, keeping a proven rename in the act-now band while
   reserving `fused = 1.0` for byte-proven duplication. LSH-only and
   embedding-discovered pairs render the bounded max fusion unchanged — the
   same formula with the content factor at its implicit 1.0.
4. **Routing — three zones over `support = max(agreement,
   rename_consistency)`** (either population may vouch; never their mean).
   Below the support floor (0.7, the [TECH-TOKEN-SOURCERERCC] Type-3 overlap
   cutoff) with no semantic support, the cluster joins the
   [RANK-STRUCTURAL-ONLY] routing — surfaced honestly or hidden as cross-file
   scaffolding, and demoted in ranking. At or above the promote bar (0.85,
   act-now grade) the cluster is a proven clone — a byte-agreeing near-miss
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
`token_jaccard` (≥ 0.95, the near-identical routing line) saturates on shape
matches exactly as `structural` does — the surviving flutter/flutter #331
cluster read `structural=0.62, token_jaccard=0.98, fused=1.00` because
transitive closure mixed structural and LSH pairs. The gate therefore fires on
*either* saturating signal. Shape-mismatched members have no positional
alignment, so their agreement is the key-set Jaccard of their content keys — a
genuine Type-3 near-miss shares nearly all of them; renamed scaffolding shares
few. The verbatim guard is proportional (≥ half the members must participate
in byte-identical duplicates): a verbatim pair among a couple of lookalikes
(#104) still vouches for its cluster, but two copied example widgets inside a
453-member framework family (0.4%) do not. `data`-category
clusters are exempt from the structural-only ranking demotion — their weight
belongs to the `[ranking] data_clones` policy ([RANK-CATEGORY]) so
`data_clone_weight = 1.0` can still restore a table the gate routed to the
structural-only bucket.

### [REMOVE-STUB] Test-only stub provider must never ship
The deterministic BLAKE3 stub embedding provider named in [FUSION-EMBED-PROVIDER]
exists purely so E2E tests can exercise the embedding path without a live model.
It lives behind the `test-support` Cargo feature, is **never** registered in
`ProviderRegistry::production`, and is barred from the shipped VSIX by a packaging
gate. `[REMOVE-STUB]` tags the code sites that enforce this boundary so a grep
proves the stub cannot leak into a release; any new stub-touching code must carry
the tag and stay test-only.
