# CodeDedup — Research & Spec

This doc captures formal research on code clone / duplication detection that informs CodeDedup's design. **Primary goal:** pick techniques that are (a) fast enough for a CLI to run on a whole repo, (b) accurate across Type-1 → Type-3 clones (and Type-4 where feasible), and (c) compatible with a future **long-running MCP/LSP** mode — incremental, per-file, byte-range-addressable, and cheap to keep live under a file watcher.

### [PRINCIPLES-AUDIENCE-AGENT] Audience for the report: AI coding agents

The report is not just for humans scanning a terminal — **the primary consumer is an AI coding agent using CodeDedup as a tool**. Design choices follow from that:

- Structured output is the product. JSON is the canonical format; the text renderer is a pretty-printer over the same data. Never emit information in text that isn't also in JSON.
- Every cluster carries enough context for an agent to act without re-reading the whole repo: exact byte ranges, file paths, a canonical representative snippet, the reason signals fired (structural / LSH / embedding with scores), and a suggested refactor hint where one is reliably inferrable (e.g. "extract as shared function," "move to module X," "both call sites are in the same crate").
- The schema is stable, versioned (`report_schema_version`), and strictly-typed so agents can parse without heuristics. Breaking changes bump the version; additive changes don't.
- No ANSI colour codes, no unicode box-drawing, no paging — the agent needs a clean stream. The `text` format is ASCII-only and line-oriented.
- Per-cluster entries include a short natural-language `summary` field written for an agent reader ("3 near-identical copies of a 42-node `switch` block across `Foo.cs:120-180`, `Bar.cs:55-112`, and `Baz.cs:200-260`; structural=1.0, token_jaccard=0.97, embedding_cos=0.91 — safe to extract"). This is a synthesised description, not a log, and it's computed from the same signals the score uses.

See [OUTPUT-SCHEMA-JSON] for the JSON schema. The report format is a first-class interface — changes go through the same review bar as the ranking formula.

### [PRINCIPLES-LONG-RUNNING-DAEMON] Long-running mode (MCP/LSP) as a load-bearing constraint

CodeDedup v1 is a batch CLI, but the architecture must not foreclose a future daemon mode:

- **Library core.** `codededup-core` owns the pipeline. The CLI is a thin shell. An MCP/LSP binary is just a second shell over the same crate.
- **Incremental first, batch second.** Every pipeline stage (parse, fingerprint, LSH, embedding) is keyed by `(file_content_hash, model_id, model_version)` and cached. A batch run is "incremental starting from empty cache." A watcher-driven update is "incremental starting from the previous cache." There is no separate batch code path.
- **Report is a materialized view over the cache, not a one-shot render.** Clusters are computed from the cached per-file fingerprints; re-rendering after a file change re-runs only cluster recomputation on the affected fingerprints.
- **File-watcher-driven incremental updates are a v2 feature — not v1.** v1 produces correct reports cheaply *because* the cache keys already support "this file didn't change, skip it." v2 wires a `notify`-based watcher to `codededup-core` and calls the existing incremental update path. v1 must ship with the cache keys and the incremental update function in place, even if the only caller is `main`.
- **Byte ranges, not line numbers, are the source of truth** everywhere in the core. Line numbers are derived at render time. LSPs need byte offsets; computing them retroactively would be a rewrite.
- **No process-global mutable state outside `src/state.rs`.** A daemon keeps multiple analyses live in one process — anything that assumes "one run, then exit" will bite later.

---

## Clone Type Taxonomy

### [CLONE-TYPE-TAXONOMY] Ground rules

- **Type-1** — identical code, ignoring whitespace/comments.
- **Type-2** — identical up to renaming of identifiers/literals/types.
- **Type-3** — Type-2 + added/removed/modified statements ("near-miss" clones).
- **Type-4** — semantically equivalent, syntactically different (same behavior, different structure/algorithm).

Recent work reframes Type-4 specifically as *"code segments deliver identical functionality through syntactically distinct implementations, such as differing algorithmic approaches or data structure choices that yield substantially varied program structures."* ([PMC — Semantic code clone detection via hybrid IR + BiLSTM, 2025](https://pmc.ncbi.nlm.nih.gov/articles/PMC12818651/))

**Implication for CodeDedup:** Types 1–3 are the sweet spot for a deterministic static tool. Type-4 is expensive, noisy, and only reliably solved today with LLMs + execution-based validation — treat it as out-of-scope for v1.

---

## Landscape of Techniques (2009 → 2026)

Ordered from cheapest/oldest to most expensive/newest.

### [TECH-TOKEN-SOURCERERCC] Token-based (SourcererCC, CCFinder, NiCad)

- **SourcererCC**: bag-of-tokens + inverted index + overlap filter. Scales to **250 MLOC on a workstation**, targets Type-1/2/3. Still the scalability benchmark.
- **NiCad**: pretty-printed, normalized token sequences compared via **LCS**. Higher precision than SourcererCC but slower (34 min vs 13 min on the same corpus in one benchmark).
- Simple token-based approaches **remain competitive with tree-based detectors on runtime and simplicity**, per recent evals.

URLs:
- [SourcererCC: Scaling Code Clone Detection to Big-Code (Semantic Scholar)](https://www.semanticscholar.org/paper/SourcererCC:-Scaling-Code-Clone-Detection-to-Sajnani-Saini/e1abe96610cb3bc989e727f0b59cebedb14260f1)
- [The NiCad Clone Detector (ResearchGate)](https://www.researchgate.net/publication/221219568_The_NiCad_clone_detector)
- [TACC vs SourcererCC/NiCad, ICSE 2023 (PDF)](https://wu-yueming.github.io/Files/ICSE2023_TACC.pdf)
- [Scalable clone detection via adaptive prefix filtering (PDF)](https://damevski.github.io/files/nishi_scalable_2017_preprint.pdf)

### [TECH-AST-FINGERPRINT] AST fingerprinting (Baxter 1998, Chilowicz 2009)

- **Baxter et al.** — seminal work: hash AST subtrees, cluster by hash, then extend to near-miss via tree edit distance.
- **Chilowicz et al.** — *"each node of an AST is associated with a fingerprint based on a hash value (incrementally computed) of the subtree rooted at the node"* — allows exact subtree clustering + approximate extension over sibling sequences. This is effectively what CodeDedup is building.
- **ASPDup** — AST-sequence-based progressive duplicate detection; recent practical variant.

URLs:
- [Baxter et al., Clone Detection Using Abstract Syntax Trees, ICSM 1998 (PDF)](https://leodemoura.github.io/files/ICSM98.pdf)
- [Chilowicz et al., Syntax Tree Fingerprinting, ICPC 2009 (PDF)](https://igm.univ-mlv.fr/~chilowi/research/syntax_tree_fingerprinting/syntax_tree_fingerprinting_ICPC09.pdf)
- [Chilowicz et al. — Foundation paper (CORE PDF)](https://core.ac.uk/download/pdf/48343903.pdf)
- [Syntax tree fingerprinting (IEEE)](https://ieeexplore.ieee.org/document/5090050/)
- [Source Code Plagiarism via AST Fingerprinting, IEEE 2022](https://ieeexplore.ieee.org/document/9960266)
- [ASPDup: AST-Sequence Progressive Duplicate Detection (ACM)](https://dl.acm.org/doi/10.1145/3457913.3457938)

### [TECH-HASH-PRIMITIVES] Near-duplicate hashing primitives (the plumbing)

These are the building blocks used inside token-based and AST-based detectors for fast approximate matching at scale:

- **Winnowing** (Schleimer/Wilkerson/Aiken, SIGMOD 2003) — the MOSS algorithm. Select fingerprints from k-gram hashes using a local window minimum. Guarantees that any match ≥ window length is detected.
- **MinHash** (Broder, 1997) — estimates Jaccard similarity of sets; near-optimal for set-overlap clone detection (tokens, k-grams).
- **SimHash** (Charikar, 2002) — cosine-similarity-preserving hash for weighted feature vectors.
- **LSH** (Indyk & Motwani, 1998) — umbrella framework; used to bucket similar fingerprints in sub-linear time.

Empirically, **MinHash outperforms SimHash** on binarized feature datasets.

URLs:
- [MinHash (Wikipedia)](https://en.wikipedia.org/wiki/MinHash)
- [SimHash (Wikipedia)](https://en.wikipedia.org/wiki/SimHash)
- [Locality-Sensitive Hashing (Wikipedia)](https://en.wikipedia.org/wiki/Locality-sensitive_hashing)
- [In Defense of MinHash Over SimHash (PMLR)](http://proceedings.mlr.press/v33/shrivastava14.pdf)

### [TECH-EMBED-NEURAL] Neural embeddings (CodeBERT, GraphCodeBERT, UniXCoder, CodeT5+)

- On **BigCloneBench**: CodeBERT F1 = 0.965, GraphCodeBERT F1 = 0.971, UniXCoder F1 ≈ 0.918 (lower variance).
- **SSCD** (Ahmed et al., 2024) — BERT-derived embeddings + **approximate nearest neighbor (ANN) search** for large-scale Type-3/4 recall. Beats SourcererCC and SAGA at industrial scale. **This is the most relevant "RAG-style" prior art for us** — embeddings indexed in a vector store, queried by k-NN.
- **Selecting & Combining LLMs for Scalable Clone Detection** (arXiv 2510.15480, 2025) — evaluated 76 LLMs; **CodeT5+110M, CuBERT, SPTCode** were top performers. Ensembling via max/sum (not avg) gave an additional lift: 46.91% precision on commercial data vs 39.71% for best single model (vs 19% for CodeBERT).
- **Small embedding sizes and small tokenizer vocabularies are advantageous**; large embeddings *hurt* recall.

URLs:
- [CodeBERT / GraphCodeBERT clonedetection (GitHub)](https://github.com/microsoft/CodeBERT/tree/master/GraphCodeBERT/clonedetection)
- [CodeXGLUE BigCloneBench benchmark (GitHub)](https://github.com/microsoft/CodeXGLUE/blob/main/Code-Code/Clone-detection-BigCloneBench/README.md)
- [SSCD: Nearest-neighbor BERT-based scalable clone detection, Wiley 2024](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3355) *(Cloudflare-gated; DOI 10.1002/spe.3355)*
- [Selecting and Combining LLMs for Scalable Clone Detection (arXiv 2510.15480)](https://arxiv.org/abs/2510.15480)
- [Improving Similarity Detection via GraphCodeBERT + extra features (arXiv 2408.08903)](https://arxiv.org/html/2408.08903)
- [Evaluating Small-Scale Code Models for Clone Detection (arXiv 2506.10995)](https://arxiv.org/pdf/2506.10995)
- [CloReCo: Benchmarking Platform for Clone Detection, 2025 (PDF)](https://www.scitepress.org/Papers/2025/136449/136449.pdf)
- [Generalizability of Clone Detection on CodeBERT, ASE 2022 (ACM)](https://dl.acm.org/doi/abs/10.1145/3551349.3561165)

### [TECH-LLM-HYBRID] LLM + execution / hybrid approaches (Type-4 frontier)

- **HyClone** (arXiv 2508.01357, 2025) — two-stage: (1) LLM filters obvious non-clones, (2) LLM-generated test inputs drive **cross-execution validation** of remaining pairs. Targets Python Type-4.
- **Rator** (Springer Cybersecurity 2025) — tree-encoding by **node degrees-of-freedom** → vector per subtree → similarity features → ML classifier. F1 = 0.99 on BigCloneBench, 0.91 on Google Code Jam, **93× faster than ASTNN**. Also provides *fine-grained* localization (Top-2/Top-3 ranking of the specific clone subtree).
- **Empirical study of LLM-based clone detection** (arXiv 2511.01176, 2025) — LLMs score highly on CodeNet (o3-mini F1 = 0.943) but **drop significantly on BigCloneBench**, revealing dataset bias. Response consistency is high (>90%).
- LLMs with simple prompting: **LLaMA excels at syntactic clones, struggles with semantic clones**.

URLs:
- [HyClone (arXiv 2508.01357)](https://arxiv.org/abs/2508.01357)
- [Rator (Springer, 2025)](https://link.springer.com/article/10.1186/s42400-025-00456-4)
- [Empirical Study of LLM-Based Clone Detection (arXiv 2511.01176)](https://arxiv.org/abs/2511.01176)
- [SCOTT: Semantic Mining from Graph and Text (ScienceDirect)](https://www.sciencedirect.com/science/article/abs/pii/S0957417426009139)
- [Semantic code clone detection via hybrid IR + BiLSTM (PLOS ONE, 2025)](https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0340971) · [PMC mirror](https://pmc.ncbi.nlm.nih.gov/articles/PMC12818651/)
- [Cross-Lingual LLM Clone Detection Struggles (arXiv 2408.04430)](https://www.arxiv.org/pdf/2408.04430)
- [LLM-based Post Hoc Explainer for Clone Detection (arXiv 2509.22978)](https://arxiv.org/html/2509.22978)
- [Nuanced Clone Detection via LLM Code Revision + AST GAT (ResearchGate)](https://www.researchgate.net/publication/397279385_Nuanced_Code_Clone_Detection_through_LLM-based_Code_Revision_and_AST_Graph_Modeling)
- [A Survey of Software Clone Detection from Security Perspective (Semantic Scholar)](https://www.semanticscholar.org/paper/A-Survey-of-Software-Clone-Detection-From-Security-Zhang-Sakurai/c834d313a2dca5747245c895b1a7c53e503ca8f6)
- [Multilingual Clone Detector Benchmarking (arXiv 2409.06176)](https://arxiv.org/pdf/2409.06176)
- [Clone Detection & Similarity Assessment boundaries, MDPI 2025](https://www.mdpi.com/2504-2289/9/2/41)

---

## Pipeline design — hybrid, not pure-RAG

### [FUSION-POLICY-HYBRID] The state of the art is HYBRID, not pure-RAG

**The research is unambiguous: the state of the art is HYBRID, not pure-RAG.** CodeDedup is hybrid.

### [FUSION-POLICY-NO-PURE-RAG] No paper recommends pure embeddings / pure RAG

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

### [FUSION-RATIONALE-AGAINST-PURE-RAG] Why pure-RAG loses
- **Empirical Study of LLM-Based Clone Detection** (arXiv 2511.01176, 2025): LLMs hit F1 0.943 on CodeNet but **drop significantly on BigCloneBench**. Pure-learned approaches are dataset-brittle. Structural signals anchor them.
- **Cross-Lingual LLM Clone Detection Struggles** (arXiv 2408.04430): *"Embedding models enable the training of classifiers that outperform LLMs by ~1–20 percentage points"* — and those classifiers take structural features as input.
- **Smaller embedding sizes beat larger ones** for clone detection. This directly contradicts the "bigger vector DB = better" intuition of pure-RAG.
- **Reports must cite exact byte ranges** (LSP requirement per CLAUDE.md). Pure embedding similarity gives you "these two fragments are similar" but not "this specific subtree of fragment A matches this specific subtree of fragment B." AST fingerprinting gives that natively; Rator showed it's also achievable from tree encoding with Top-2/Top-3 localization.

### [FUSION-SIGNALS-THREE-LAYER] CodeDedup is hybrid by design

The pipeline fuses three signals:

1. **Structural (AST fingerprinting)** — Merkle-hash every tree-sitter subtree after normalization. Catches Type-1, Type-2, most Type-3. Fast, deterministic, gives exact byte ranges. (Chilowicz 2009 + Baxter 1998.)
2. **Token LSH (MinHash over normalized k-grams)** — catches Type-3 cases where structure diverged but token bag is close. Fast, deterministic. (SourcererCC 2016.)
3. **Learned embeddings (local, via Ollama)** — catches Type-3/Type-4 the structural passes miss. Used both as a **recall expander** (find candidates the hash-based passes didn't cluster) and as a **re-ranker** (promote semantically-similar AST clusters in the final score). (SSCD 2024, ensemble-LLM 2025.)

All three run by default. The research doesn't support shipping without embeddings — it's a measurable accuracy loss on Type-3/4.

### [FUSION-EMBED-PROVIDER] Embedding layer — concrete choices

- **Provider and model are not hard-coded.** Both are CLI flags (`--embedding-provider`, `--embedding-model`), backed by config file and env var fallbacks. The core crate exposes an `EmbeddingProvider` trait; providers are selected at runtime by name. v1 ships with an `ollama` provider; a local `onnx` provider and a stub/null provider are on the roadmap. The research picks a *default*, not a lock-in.
- **Default provider + model (overridable).** Provider defaults to `ollama` (local, no network) and model defaults to `nomic-embed-code` — these are recommended starting points from the 2025 ensemble paper's finding that *"smaller embedding sizes, smaller tokenizer vocabularies and tailored datasets are advantageous"*. CodeT5+110M and UniXCoder are alternate top performers cited in the literature; either should be selectable via `--embedding-model` once exposed through a provider.
- **Local-only is a policy, not a hard requirement of the architecture.** The default stack never touches the network, but the trait doesn't forbid a hosted provider. A user configuring `--embedding-provider=hosted-foo` opts into that tradeoff deliberately; we don't enable it for them.
- **ANN index: HNSW.** Use `usearch` or `instant-distance` (pure Rust, no C deps). SSCD validated HNSW at 250 MLOC.
- **Ensemble by max/sum, never average.** The 2025 ensemble paper is specific: averaging *hurts*; max and sum help. Score normalization is mandatory before combining.
- **Cache by `(file_content_hash, provider_id, model_id, model_version)`.** Re-runs are free; switching providers or models invalidates only the embedding layer and leaves structural/LSH caches intact. LSP incremental mode reuses the same cache unchanged.
- **Index granularity: AST subtrees above min-node threshold**, not whole files. We already have those subtrees from the structural pass — embed them directly. This keeps embeddings byte-range-addressable and dramatically reduces the N in k-NN.
- **Determinism caveat.** Embedding + ANN is approximate. Mitigate by: (a) recording `provider_id`, `model_id`, and `model_version` in the `.codededup-cache` header and the report, (b) using deterministic ANN parameters (fixed seed, fixed ef_construction), (c) final ranking is still computed over the *union* of structural + LSH + embedding candidates, so a missed ANN neighbor only loses recall, never changes existing cluster content.

### [FUSION-STRATEGY-MAX-SUM] Fusion strategy (how the three signals combine)

Per the ensemble-LLM 2025 findings (max/sum with normalization):

1. Compute a candidate set of clone pairs as the **union** of: structural-hash matches, LSH bucket collisions, and top-k embedding neighbors per subtree.
2. For each candidate pair, compute three scores in [0,1]: `structural_sim`, `token_jaccard`, `embedding_cos`.
3. Final pair score = **max-normalized sum** of the three (not average).
4. Cluster pairs by transitive closure above a threshold.
5. Weight each cluster by the ranking formula in §4 for "worst offenders first."

This way, a Type-1 clone scores ≈1 on all three signals, a Type-2 ≈1 on structural+embedding and ~high on LSH, a Type-3 may score high on LSH+embedding and medium on structural, and a Type-4 scores primarily on embedding. Every type lands in the report; scores explain *why*.

---

## Pipeline stages (v1, hybrid by default)

### [PIPELINE-LANG-TRAIT] Language plugin trait
The single extension point. Implementations live in `codededup-core::lang::<name>`. Each implementation provides: (a) tree-sitter grammar factory, (b) file-extension filter, (c) per-language node-kind normalization rules that collapse identifier / literal / trivia nodes into their structural kind. The trait output type (`NormalizedNode`) is identical across languages so downstream stages are language-agnostic. v1 ships with three plug-ins: `csharp` (`tree-sitter-c-sharp`), `rust` (`tree-sitter-rust`), and `python` (`tree-sitter-python`). Adding a language = one `LanguageParser` impl + pinning the grammar version in `Cargo.toml`. Shared walking / interning plumbing lives in `lang::shared` so every language module is just a `normalise_kind` match plus boilerplate.

### [PIPELINE-DISCOVER-FILES] File discovery
Walk the target path with the `ignore` crate, respecting `.gitignore` and Git's standard ignore rules. Filter by the set of file extensions contributed by registered `LanguageParser`s. Additionally drop paths matching `[EXCLUSION-CONFIG]` `exclude` patterns — those files are never parsed. Every surviving path is registered with [STATE-FILE-REGISTRY] and downstream code traffics in `FileId`, never `Path`.

### [PIPELINE-NORMALIZE-AST] AST normalization
For each file, parse with the selected language's tree-sitter grammar and walk the resulting tree bottom-up, producing `NormalizedNode { kind: &'static str, children: Vec<Self>, byte_range, file_id }`. Identifier / literal / comment / whitespace nodes are collapsed to their structural kind so Type-2 clones (renamed identifiers) hash identically. Byte ranges are preserved and are the source of truth for any later rendering — line numbers are derived.

### [PIPELINE-FINGERPRINT-MERKLE] Structural fingerprint (Merkle)
Bottom-up Merkle hash over `NormalizedNode`. Each node's hash combines its own `kind` string with the ordered hashes of its children using `blake3`. Each node stores `(hash, subtree_node_count, byte_range, file_id)`. Nodes whose subtree size is below `--min-nodes` are excluded from clustering per [DECISION-MIN-NODES].

### [PIPELINE-CLUSTER-EXACT] Exact subtree clustering
Group `NormalizedNode` fingerprints by `hash`. Every bucket with ≥ 2 entries is a candidate clone cluster. Covers Type-1 and normalized Type-2 deterministically in O(n).

### [PIPELINE-RANK-WORST-FIRST] Ranking: worst offenders first
`weight = clone_node_count × (cluster_size − 1) × log2(1 + total_spanned_loc)`. Clusters are sorted by weight descending. A cluster with one member (no duplication) scores zero by construction. Later stages multiply in the fusion score from [FUSION-STRATEGY-MAX-SUM].

### [STATE-FILE-REGISTRY] File registry (the only global state)
`codededup-core::state::FileRegistry` maps `FileId ↔ PathBuf`. This is the *only* place mutable state associated with a pipeline run may live. Instances are per-run (not process-global) so a future long-running daemon can keep multiple analyses side-by-side.

### [OUTPUT-SCHEMA-JSON] Canonical JSON schema
JSON is the canonical report format ([PRINCIPLES-AUDIENCE-AGENT]). Text and HTML are derived from it — nothing lives in two places. Text is terse and AI-readable (ASCII, line-oriented, no colour). HTML is single-file, inline-CSS, human-readable, and embeds the same `schema_doc` and `action_hints` the JSON carries so a human opening the file cold understands what they are looking at.

Top level at `report_schema_version = 2`:

- `report_schema_version: u32` — bumped on breaking change.
- `tool_version: String` — producer binary version.
- `min_nodes: u32` — subtree size floor used for the run.
- `files_analysed: usize` — count of files actually parsed.
- `clusters_hidden: usize` — clusters that existed but were suppressed from `clusters` because every occurrence matched a [EXCLUSION-CONFIG] `report_hide` pattern. Surfaces the volume of ignored duplication without leaking the content.
- `schema_doc: &'static str` — markdown explaining every field, signal, threshold, ranking formula, byte-range convention, and clone taxonomy. Shipped via `include_str!` so it cannot drift from the schema.
- `action_hints: Vec<ActionHint>` — short playbook entries ("high structural + high jaccard → extract shared function", etc.) agents can consult before deciding how to act.
- `clusters: Vec<ReportCluster>` — ranked worst-offenders-first per [PIPELINE-RANK-WORST-FIRST].

`ReportCluster`:

- `id`, `weight`, `size`, `canonical_node_count`, `signals { structural, token_jaccard, embedding_cos, fused }`, `summary` — as in v1.
- `interpretation: String` (new in v2) — one-line synthesis computed from the signal combination ("Type-1 exact clone, safe to extract", "Type-3 near-miss, review before merging", "Low-information LSH-only match, treat as hint"). Derived, so rendering is deterministic.
- `occurrences: Vec<ReportOccurrence>` — each with `path`, `start_byte`, `end_byte`, and `hidden: bool` (true when the occurrence matched a `report_hide` pattern per [EXCLUSION-CONFIG]).

`--from-report <file.json>` skips analysis and re-renders the text + HTML views from a canonical JSON report. Keeps the rendering pipeline testable in isolation and makes re-formatting a cached report free.

The default invocation writes all three formats to disk (`codededup-report.{json,txt,html}` in CWD, or `<path>.{json,txt,html}` when `--output <path>` is given). `--nojson`, `--notext`, `--nohtml` suppress individual formats; at least one must remain enabled.

### [EXCLUSION-CONFIG] Exclusion configuration
A single opt-in configuration file — `.codededup.toml` in the scan root, or `--config <path>` — controls two orthogonal exclusion tiers. Motivating case: generated code. We want to know when hand-written code duplicates a generated file, but we do not want the generated file itself to dominate the top of the report.

**Tiers.**

- `exclude` — matching files are dropped in [PIPELINE-DISCOVER-FILES] before parsing. They are not counted in `files_analysed`, never fingerprinted, never embedded, and cannot appear in any cluster. Use for third-party vendored code you do not want analysed at all.
- `report_hide` — matching files **are analysed** and can contribute to clustering, but each occurrence is flagged `hidden = true` at render time. A cluster where **every** occurrence is hidden is dropped from the rendered `clusters` list and counted under `clusters_hidden`. A cluster with at least one non-hidden occurrence is kept intact so the user sees "regular code duplicates generated code." This is the default tier for generated output like `*.g.cs`, `*.generated.cs`, OpenAPI clients, protobuf output.

**File format.** TOML. Parsed via the `toml` crate. Minimal, familiar, diffable:

```toml
[defaults]
exclude = ["vendor/**", "third_party/**"]
report_hide = ["**/*.generated.cs", "**/*.g.cs"]

[language.csharp]
report_hide = ["**/Migrations/**/*.cs"]

[language.rust]
report_hide = ["**/target/**"]
```

**Pattern semantics.** `ignore::gitignore` syntax. Same engine as [PIPELINE-DISCOVER-FILES] so patterns behave identically to `.gitignore`. Paths are matched relative to the scan root.

**Merge rule.** Per-language sections **extend** `[defaults]`, they do not replace it. A `.rs` file is checked against `defaults.report_hide ∪ language.rust.report_hide`. Keeps the config declarative — you never have to repeat shared patterns in every language block.

**No config ⇒ no exclusions.** Current behaviour is preserved. Absence of `.codededup.toml` is not an error and is not warned on.

**`report_hide` membership is a rendering decision, not an analysis one.** Hidden files still participate in fingerprinting, LSH, and (later) embedding. The `hidden: bool` per occurrence is the only surface-level signal of the policy, so downstream consumers that want the unfiltered view can ignore `clusters_hidden` and inspect `occurrences[].hidden` directly.



1. **Parser:** tree-sitter per language (C#, Rust, Python) — already mandated by CLAUDE.md.
2. **Normalization:** strip identifiers, literals, comments, whitespace. Keep operators, keywords, and structural node kinds. Per-language rules; identical output format across languages so downstream layers are language-agnostic.
3. **Structural fingerprint:** bottom-up Merkle hash of every AST subtree with ≥ N nodes (configurable, default ~30). Each node stores `(hash, size, byte_range, file_id)`.
4. **Exact-clone clustering:** group subtrees by hash. O(n) after hashing. Covers Type-1 and normalized Type-2.
5. **Near-clone extension:** for each exact cluster, extend matches over adjacent sibling subtrees (Chilowicz's approach). Catches a chunk of Type-3 without tree-edit-distance.
6. **Token MinHash/LSH pass:** normalized k-grams → MinHash → LSH buckets. Catches Type-3 where structure diverged but token bag is close. Deterministic.
7. **Embedding pass (pluggable provider, local-by-default):** embed every AST subtree from step 3 through the configured `EmbeddingProvider`. Provider (`--embedding-provider`, default `ollama`) and model (`--embedding-model`, default `nomic-embed-code`) are runtime-selectable — never hard-coded. Index via HNSW (`usearch` or `instant-distance`, both pure Rust). For each subtree, retrieve top-k neighbors above a cosine threshold. Catches Type-3 and Type-4 the prior passes miss. **First-class pipeline stage, not optional.**
8. **Candidate union + fusion:** union the pairs produced by steps 4, 6, 7. For each pair compute `(structural_sim, token_jaccard, embedding_cos)`, normalize, combine via **max-normalized sum** (per ensemble-LLM 2025). Cluster by transitive closure above a threshold.
9. **Ranking score:** `weight = clone_node_count × (cluster_size − 1) × log(total_spanned_loc) × fusion_score`. Sort descending. `cluster_size − 1` ensures singletons score zero.
10. **Output (agent-first):** JSON is canonical; text is a pretty-printer over the same struct. Stable schema with `report_schema_version`. Each cluster: exact byte ranges, file paths, canonical representative snippet, per-signal scores (structural / LSH / embedding), a short agent-oriented `summary`, and a refactor hint where reliably inferrable. ASCII-only text format; no colour codes, no paging. See "Audience for the report" above.
11. **Incremental cache:** `(file_content_hash, provider_id, model_id, model_version) → (parse_tree, subtree_fingerprints, embeddings)`. Re-runs with unchanged files skip all inference. v1 uses this to make batch runs cheap; v2 uses the same keys for a watcher-driven update loop. Switching embedding provider/model invalidates only the embedding layer, not the structural/LSH caches.
12. **Library vs binary split:** `codededup-core` owns the pipeline. `codededup` binary is a thin shell. An MCP/LSP daemon binary is a later sibling shell over the same crate — no pipeline code moves.
13. **Incremental update entry point from day one:** `codededup-core` exposes `update_files(changed: &[FileId]) -> ReportDelta` as a first-class API, even though v1's only caller is `main`. This is the function the future file watcher will call.
14. **Embedding disable flag:** `--embeddings={auto,required,off}`. `off` runs the deterministic two-pass pipeline only; the report header notes reduced Type-4 recall. `auto` (default) uses embeddings when the configured provider is reachable and falls back with a `tracing::warn!` otherwise. `required` hard-fails if the provider is unreachable.
15. **Provider/model pinning:** `provider_id`, `model_id`, and `model_version` are written into the cache header and into every report. Changing any of them invalidates the embedding layer deterministically and explicitly.

---

## Decisions with fallback rules

### [DECISION-MIN-NODES] Minimum subtree size

Default `--min-nodes` = **30**. Subtrees below this threshold are excluded from fingerprinting, clustering, and embedding. Rationale: smaller subtrees (`return x;`, single-statement blocks) are noise and dominate the report. If the top-50 clusters on a real C# corpus are dominated by trivial fragments, raise the default to 40 before the next release. If large real duplicates are being missed, lower to 20. The flag is always user-overridable. Never ship a default below 15 or above 60.

### [DECISION-CROSS-LANGUAGE] Cross-language clones

Out of scope for v1. The normalization format is identical across languages so that a future cross-language pass can compare fingerprints directly without rework. Do not add heuristics, mappings, or type-system bridges until cross-language is an explicit feature goal.

### [DECISION-TYPE3-TWO-PASS] Type-3 recall via AST sibling-extension + token LSH

Ship both passes. Sibling-extension runs first because it is cheaper and produces byte-range-accurate matches. Token LSH runs second and surfaces Type-3 candidates whose structure diverged too far for sibling-extension. Fallback rule: if the LSH pass contributes fewer than 5% additional clusters on three consecutive representative corpora (measured in a calibration run), mark it as a future removal candidate and raise the issue — do not silently disable it.

---

## Reading list

### [READ-LIST-DEDUPED] Deduplicated reading list

Canonical:
- [Baxter et al. 1998 — AST clone detection](https://leodemoura.github.io/files/ICSM98.pdf)
- [Chilowicz et al. 2009 — Syntax tree fingerprinting](https://igm.univ-mlv.fr/~chilowi/research/syntax_tree_fingerprinting/syntax_tree_fingerprinting_ICPC09.pdf)
- [SourcererCC — Scaling clone detection (Semantic Scholar)](https://www.semanticscholar.org/paper/SourcererCC:-Scaling-Code-Clone-Detection-to-Sajnani-Saini/e1abe96610cb3bc989e727f0b59cebedb14260f1)
- [NiCad clone detector](https://www.researchgate.net/publication/221219568_The_NiCad_clone_detector)

Recent (2024–2026):
- [SSCD — BERT + ANN scalable clone detection (Wiley 2024, gated)](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3355)
- [Selecting & Combining LLMs for Clone Detection (arXiv 2510.15480)](https://arxiv.org/abs/2510.15480)
- [HyClone — LLM + execution validation (arXiv 2508.01357)](https://arxiv.org/abs/2508.01357)
- [Rator — tree encoding via node DoF (Springer 2025)](https://link.springer.com/article/10.1186/s42400-025-00456-4)
- [Empirical Study of LLM-Based Clone Detection (arXiv 2511.01176)](https://arxiv.org/abs/2511.01176)
- [Evaluating Small-Scale Code Models (arXiv 2506.10995)](https://arxiv.org/pdf/2506.10995)
- [CloReCo benchmarking platform, 2025](https://www.scitepress.org/Papers/2025/136449/136449.pdf)
- [Hybrid IR + BiLSTM semantic clone detection, PLOS ONE 2025](https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0340971)
- [Multilingual Clone Detector Benchmark (arXiv 2409.06176)](https://arxiv.org/pdf/2409.06176)

Primitives:
- [MinHash](https://en.wikipedia.org/wiki/MinHash) · [SimHash](https://en.wikipedia.org/wiki/SimHash) · [LSH](https://en.wikipedia.org/wiki/Locality-sensitive_hashing)
- [In Defense of MinHash Over SimHash (PMLR)](http://proceedings.mlr.press/v33/shrivastava14.pdf)

Surveys:
- [A Survey of Software Clone Detection from Security Perspective](https://www.semanticscholar.org/paper/A-Survey-of-Software-Clone-Detection-From-Security-Zhang-Sakurai/c834d313a2dca5747245c895b1a7c53e503ca8f6)
- [Survey of Clone Detection Techniques, Types I–IV](https://www.semanticscholar.org/paper/The-Survey-of-the-Code-Clone-Detection-Techniques-(-Kaur-Sharma/f5600f495f863fd9f62ed29873d509939cd09ca0)
