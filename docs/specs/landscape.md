# Landscape of Techniques (2009 → 2026)

Ordered from cheapest/oldest to most expensive/newest.

### [TECH-PMATCH-BAKER] Parameterized matching (Baker 1995/1996)

- **Baker's `dup`** — the original Type-2 formalism: two fragments *p-match* when one consistent one-to-one substitution over their parameter symbols maps one onto the other.
- **prev-encoding** — each parameter occurrence is encoded as the distance to the previous occurrence of the same symbol, `0` for a first occurrence. Two fragments p-match iff their encodings are equal, with the structural consequence Deslop's rename evidence is built on: **a symbol seen once encodes `0` and matches any other first occurrence — it carries no binding constraint — while every repetition is a constraint the match must satisfy.** Corroboration by repetition, not mere consistency, is the proof of a deliberate rename; sibling scaffolding gets one consistent substitution for free from its own subject name.

URLs:
- [Baker 1995 — On Finding Duplication and Near-Duplication (dup / p-match, WCRE)](https://plg.uwaterloo.ca/~migod/846/papers/wcre95-baker.pdf)
- [Baker 1996 — Parameterized Pattern Matching (prev-encoding, JCSS)](https://www.sciencedirect.com/science/article/pii/S0022000096900033)

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
- **Chilowicz et al.** — *"each node of an AST is associated with a fingerprint based on a hash value (incrementally computed) of the subtree rooted at the node"* — allows exact subtree clustering + approximate extension over sibling sequences. This is effectively what Deslop is building.
- **ASPDup** — AST-sequence-based progressive duplicate detection; recent practical variant.

URLs:
- [Baxter et al., Clone Detection Using Abstract Syntax Trees, ICSM 1998 (PDF)](https://leodemoura.github.io/files/ICSM98.pdf)
- [Chilowicz et al., Syntax Tree Fingerprinting, ICPC 2009 (PDF)](https://igm.univ-mlv.fr/~chilowi/research/syntax_tree_fingerprinting/syntax_tree_fingerprinting_ICPC09.pdf)
- [Chilowicz et al. — Foundation paper (CORE PDF)](https://core.ac.uk/download/pdf/48343903.pdf)
- [Syntax tree fingerprinting (IEEE)](https://ieeexplore.ieee.org/document/5090050/)
- [Source Code Plagiarism via AST Fingerprinting, IEEE 2022](https://ieeexplore.ieee.org/document/9960266)
- [ASPDup: AST-Sequence Progressive Duplicate Detection (ACM)](https://dl.acm.org/doi/10.1145/3457913.3457938)

<a id="tech-hash-primitives"></a>

**Near-duplicate hashing primitives (the plumbing).**

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

<a id="tech-llm-hybrid"></a>

**LLM + execution / hybrid approaches (Type-4 frontier).**

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
