# Pipeline design — hybrid, not pure-RAG

**Research note: the state of the art is hybrid, not pure-RAG.**

**The research is unambiguous: the state of the art is HYBRID, not pure-RAG.** Deslop is hybrid.

**Research note: no paper recommends pure embeddings / pure RAG.**

**No.** Across every 2024–2026 paper surveyed, no top-performing system is pure vector search. The strongest embedding-only approach (SSCD, Wiley 2024) frames itself as *"a BERT-based clone detection approach that targets high recall of Type 3 and Type 4 clones at scale"* — i.e. a **recall layer**, explicitly paired with structural/token methods in the surrounding pipeline. Every system that reports SOTA numbers fuses at least two representations.

Concretely:

| System (year) | Structure signal | Learned signal | Extra signal | Result |
|---|---|---|---|---|
| Rator (2025) | AST subtree encoding via node degrees-of-freedom | ML classifier over similarity features | — | F1 **0.99** BigCloneBench, 93× faster than ASTNN |
| HyClone (2025) | — | LLM semantic screen | **Execution validation** via generated test inputs | SOTA on Python Type-4 |
| SCOTT (2026) | Graph | Text/embedding | Unified framework | Type-1 → Type-4 in one pipeline |
| Hybrid IR + BiLSTM (2025) | Baf + Jimple IR | BiLSTM over both | — | Strong Type-4 |
| Ensemble-LLM (arXiv 2510.15480) | — | **Multiple** embedding models combined via max/sum | Score normalization | 46.91% precision vs 39.71% best-single, vs 19% CodeBERT |
| SSCD (2024) | — | BERT embeddings + ANN | Token pre-filter | Beats SourcererCC + SAGA at scale |
| Nuanced Clone Detection (2025) | AST graph | GAT + contrastive loss | **LLM code revision** | SOTA Type-3/4 |

The pattern is consistent: **structure (AST/graph) + learned representation (embedding) + sometimes a third verification signal (execution, normalization, ensembling).**

**Research note: why pure-RAG loses.**
- **Empirical Study of LLM-Based Clone Detection** (arXiv 2511.01176, 2025): LLMs hit F1 0.943 on CodeNet but **drop significantly on BigCloneBench**. Pure-learned approaches are dataset-brittle. Structural signals anchor them.
- **Cross-Lingual LLM Clone Detection Struggles** (arXiv 2408.04430): *"Embedding models enable the training of classifiers that outperform LLMs by ~1–20 percentage points"* — and those classifiers take structural features as input.
- **Smaller embedding sizes beat larger ones** for clone detection. This directly contradicts the "bigger vector DB = better" intuition of pure-RAG.
- **Reports must cite exact byte ranges** (LSP requirement per CLAUDE.md). Pure embedding similarity gives you "these two fragments are similar" but not "this specific subtree of fragment A matches this specific subtree of fragment B." AST fingerprinting gives that natively; Rator showed it's also achievable from tree encoding with Top-2/Top-3 localization.

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
- **Ensemble by max/sum, never average.** The 2025 ensemble paper is specific: averaging *hurts*; max and sum help. Score normalization is mandatory before combining.
- **Cache by `(file_content_hash, provider_id, model_id, model_version)`.** Re-runs are free; switching providers or models invalidates only the embedding layer and leaves structural/LSH caches intact. LSP incremental mode reuses the same cache unchanged.
- **Index granularity: AST subtrees above min-node threshold**, not whole files. We already have those subtrees from the structural pass — embed them directly. This keeps embeddings byte-range-addressable and dramatically reduces the N in k-NN.
- **The per-input character budget belongs to the provider, never to the pipeline.** A subtree longer than the budget is counted in `failed_subtrees` and never dispatched, because Ollama truncates silently (`truncate: true`) and a truncated vector misrepresents the code it claims to describe. The budget is therefore a property of the model behind the provider — `nomic-embed-text` carries a 2,048-token context, `mxbai-embed-large` 512, `qwen3-embedding` 32k — and is read from `EmbeddingProvider::max_input_chars`. `OllamaProvider` derives it at construction from the model's own `model_info["<arch>.context_length"]` via `POST /api/show`, converted at a deliberately conservative 3 chars/token, falling back to `DEFAULT_MAX_INPUT_CHARS` (6,000) when the endpoint or field is unavailable. A single pipeline-wide constant cannot be correct for two models an order of magnitude apart: gh #286 reported 14,723 of 175,160 subtrees (8.4%) dropped, at the large end where re-derived duplication is most expensive to miss, and no model swap could have recovered them while the cap sat upstream of the provider.
- **Determinism caveat.** Embedding + ANN is approximate. Mitigate by: (a) recording `provider_id`, `model_id`, and `model_version` in the `.deslop/cache` header and the report, (b) using deterministic ANN parameters (fixed seed, fixed ef_construction), (c) final ranking is still computed over the *union* of structural + LSH + embedding candidates, so a missed ANN neighbor only loses recall, never changes existing cluster content.

### [FUSION-STRATEGY-MAX-SUM] Fusion strategy (how the three signals combine)

Per the ensemble-LLM 2025 findings (max/sum with normalization):

1. Compute a candidate set of clone pairs as the **union** of: structural-hash matches, LSH bucket collisions, and top-k embedding neighbors per subtree.
2. For each candidate pair, compute three scores in [0,1]: `structural_sim`, `token_jaccard`, `embedding_cos`.
3. Final pair score = **max-normalized sum** of the three (not average).
4. Cluster pairs by transitive closure above a threshold.
5. Weight each cluster by the ranking formula in §4 for "worst offenders first."

This way, a Type-1 clone scores ≈1 on all three signals, a Type-2 ≈1 on structural+embedding and ~high on LSH, a Type-3 may score high on LSH+embedding and medium on structural, and a Type-4 scores primarily on embedding. Every type lands in the report; scores explain *why*.

### [FUSION-CONTENT-GATE] Content agreement gates shape-identical confidence

The max/sum ensemble above assumes independent members, but `structural_sim`
and `token_jaccard` are both computed from the *normalised* representation
(identifiers and literals collapsed), so on any exact shape match they agree by
construction and their sum saturates the clamp: every structural cluster used
to render `fused = 1.0` regardless of what the code actually said (gh #331,
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
   embedding-discovered pairs keep the max/sum fusion unchanged.
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
