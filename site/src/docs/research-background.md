---
layout: layouts/docs.njk
title: Research Background — Code-clone detection algorithms
description: The research behind Deslop's structural, token, and embedding-based clone detection.
eleventyNavigation:
  key: Research
  order: 8
icon: science
docsGroup: trust
---

# Research Background

Deslop combines established clone-detection techniques: normalized syntax trees, Merkle fingerprints, sibling windows, MinHash LSH, and optional embedding search. The implementation pointers below identify where each shipped technique lives.

## Why AI changes the clone problem

Code clones were already a maintenance risk before AI coding assistants. The established concern is not that every duplicate is automatically wrong; it is that copied logic must be kept consistent across fixes, security patches, and feature changes. AI-assisted development changes the economics: the cost of generating another similar implementation drops, so repeated logic can enter the repository during ordinary feature work rather than only through deliberate copy-and-paste.

Recent research supports that risk model:

- **LLM code repetition**: [*Code Copycat Conundrum: Demystifying Repetition in LLM-based Code Generation*](https://arxiv.org/abs/2504.12608) studies 19 code LLMs and reports that repetition appears across character, statement, and block levels, including structurally redundant code. The paper also evaluates a repetition-mitigation technique in open-source and industrial settings.
- **LLM-generated clones**: [*Unveiling the potential of large language models in generating semantic and cross-language clones*](https://arxiv.org/abs/2309.06424) evaluates GPT-3's ability to generate semantic and cross-language clone variants, which is directly relevant to Type-4 and cross-language duplicate detection.
- **Commercial AI code generators**: [*An Empirical Study of Code Clones from Commercial AI Code Generators*](https://dl.acm.org/doi/10.1145/3729397) (FSE 2025) reports Type-1 and Type-2 clone rates up to 7.50% for studied commercial generators, and discusses copyright, bug propagation, and vulnerability propagation risks.
- **AI-era clone detection**: [*Are Classical Clone Detectors Good Enough For the AI Era?*](https://arxiv.org/abs/2509.25754) evaluates nine clone detectors on GPTCloneBench and traditional clone benchmarks, highlighting why normalization and semantic variation matter when clones are AI-generated.
- **Technical debt in production repositories**: [*Debt Behind the AI Boom: A Large-Scale Empirical Study of AI-Generated Code in the Wild*](https://arxiv.org/abs/2603.28592) analyzes verified AI-authored commits and tracks static-analysis issues introduced by those commits. It is broader than duplication, but it supports treating AI-generated code as a technical-debt source that needs repository-level appraisal.

## Clone taxonomy

Deslop follows the standard clone taxonomy used throughout the code-clone literature:

| Clone class | Meaning | Deslop signal |
| --- | --- | --- |
| Type-1 | Exact copied text except layout or comments | Structural hash after parsing and normalization |
| Type-2 | Same structure with renamed identifiers or changed literals | Structural hash after identifier/literal collapse |
| Type-3 | Near-miss clone with inserted, deleted, or changed statements | Sibling-window fingerprints and token MinHash LSH |
| Type-4 | Similar behavior with different syntax or structure | Optional embedding cosine similarity |

The public report buckets are implemented in `crates/deslop-core/src/buckets.rs`. The code maps signal triples to five wire labels: `identical`, `nearly_identical`, `structural_only`, `loosely_similar`, and `same_behavior`. The `structural_only` bucket marks clusters whose only positive evidence is the normalized AST shape; they are weight-demoted in the ranking by default. The `same_behavior` bucket is only reachable when the embedding signal is strong enough.

## Algorithm foundations

Each row links a research line to the shipped implementation it influenced.

| Research line | What Deslop takes from it | Status & implementation pointer |
| --- | --- | --- |
| [Baxter et al. 1998 — AST clone detection](https://ieeexplore.ieee.org/document/738528) | Parse code into syntax trees, normalize irrelevant spelling, and compare tree structure rather than raw text. | ✅ `crates/deslop-core/src/lang/shared.rs`, `lang/csharp.rs`, `lang/rust_lang.rs`, `lang/python.rs`, `lang/dart.rs` (registered through `pipeline/corpus.rs::default_parsers`) |
| Chilowicz et al. 2009 — syntax-tree fingerprinting | Hash subtrees so exact structural clones become equal fingerprints; extend coverage with sibling sequences for near-miss clones. | ✅ Bottom-up BLAKE3 Merkle in `crates/deslop-core/src/fingerprint.rs::collect_non_boilerplate_fingerprints` and width-2..8 sibling windows in `crates/deslop-core/src/sibling.rs::collect_non_boilerplate_sibling_fingerprints` |
| [SourcererCC (Sajnani et al. 2016)](https://arxiv.org/abs/1512.06448) | Token k-grams and Jaccard similarity for scalable near-miss detection. | ✅ Adapted to **normalized AST-kind k-grams** rather than raw source tokens: `crates/deslop-core/src/tokens.rs`, `crates/deslop-core/src/pipeline/signatures.rs` |
| [MinHash (Broder 1997)](https://ieeexplore.ieee.org/document/666900) | Estimates Jaccard from compact signatures. | ✅ 128-value signatures in `crates/deslop-core/src/lsh.rs::minhash_signature`; Jaccard estimated by `estimate_jaccard` |
| LSH banding (Indyk & Motwani 1998) | Bucket similar fingerprints in sub-linear time. | ✅ 32 bands × 4 rows in `crates/deslop-core/src/lsh.rs::band_collisions` |
| In Defense of MinHash Over SimHash (Shrivastava & Li 2014) | Use MinHash, not SimHash, for binarized features. | ✅ MinHash chosen; SimHash and Winnowing not used |
| Neural semantic clone detection (CodeBERT, GraphCodeBERT, UniXCoder) | Use embeddings as a recall layer for Type-4 clones. | ✅ `EmbeddingProvider` trait in `crates/deslop-core/src/embedding/provider.rs`; Ollama provider in `embedding/ollama.rs`; default model `nomic-embed-text` |
| [SSCD (Ahmed et al., Wiley 2024) — BERT + ANN at scale](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3355) | HNSW ANN over BERT-style embeddings as the Type-3/4 recall path. | ✅ `instant-distance` HNSW with deterministic seed, top-k retrieval, cosine threshold 0.80: `crates/deslop-core/src/embedding/pairs.rs` |
| [Ensemble-LLM 2025 (arXiv 2510.15480) — max/sum fusion](https://arxiv.org/abs/2510.15480) | Strong signals should not be diluted by averaging. | ✅ Candidate admission uses the bounded maximum of structural, token, and embedding signals in `crates/deslop-core/src/pair.rs::PairScore::bounded_fused`. Report rendering then applies a content-evidence gate to saturated, non-identical structural matches in `crates/deslop-core/src/buckets/gate.rs::content_gated_signals`. |
| Hybrid clone detection (no pure-RAG paper recommends pure embeddings) | Union structural + token + embedding pairs, fuse, cluster. | ✅ `crates/deslop-core/src/pair.rs::candidate_pairs`, transitive closure in `crates/deslop-core/src/cluster.rs` |
| Boilerplate filtering (mature-tool convention) | Drop import / namespace / decorator clones before fingerprinting; re-surface as low-noise hints. | ✅ `crates/deslop-core/src/boilerplate.rs` and `report_boilerplate.rs` |
| Autofix `refactor.extract` (LSP code action) | Rewrite Type-1 clusters into a single shared method. | ✅ Shipped — spec in [`docs/specs/autofix-extract.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/autofix-extract.md) |

## References

- Baxter, Yahin, Moura, Sant'Anna, and Bier, *Clone Detection Using Abstract Syntax Trees*, 1998. [PDF](https://leodemoura.github.io/files/ICSM98.pdf)
- Chilowicz, Duris, and Roussel, *Syntax Tree Fingerprinting for Source Code Similarity Detection*, 2009. [PDF](https://igm.univ-mlv.fr/~chilowi/research/syntax_tree_fingerprinting/syntax_tree_fingerprinting_ICPC09.pdf)
- Sajnani, Saini, Svajlenko, Roy, and Lopes, *SourcererCC: Scaling Code Clone Detection to Big Code*, 2016. [arXiv](https://arxiv.org/abs/1512.06448)
- Roy and Cordy, *NICAD: Accurate Detection of Near-Miss Intentional Clones Using Flexible Pretty-Printing and Code Normalization*, 2008. [ResearchGate entry](https://www.researchgate.net/publication/221219568_The_NiCad_clone_detector)
- Shrivastava and Li, *Densifying One Permutation Hashing via Rotation for Fast Near Neighbor Search*, 2014. [PDF](http://proceedings.mlr.press/v33/shrivastava14.pdf)
- Roy, Alam, Al-omari, Roy, Roy, and Schneider, *Unveiling the potential of large language models in generating semantic and cross-language clones*, 2023. [arXiv](https://arxiv.org/abs/2309.06424)
- Roy et al., *GPTCloneBench: A comprehensive benchmark of semantic clones and cross-language clones using GPT-3 model and SemanticCloneBench*, 2023. [arXiv](https://arxiv.org/abs/2308.13963)
- Eagal, Stolee, and Ore, *Analyzing the dependability of Large Language Models for code clone generation*, 2025. [Journal of Systems and Software](https://www.sciencedirect.com/science/article/pii/S0164121225002171)
- Liu et al., *Code Copycat Conundrum: Demystifying Repetition in LLM-based Code Generation*, 2025. [arXiv](https://arxiv.org/abs/2504.12608)
- Alam et al., *Are Classical Clone Detectors Good Enough For the AI Era?*, 2025. [arXiv](https://arxiv.org/abs/2509.25754)
- Wu et al., *An Empirical Study of Code Clones from Commercial AI Code Generators*, 2025. [FSE 2025 entry](https://conf.researchr.org/details/fse-2025/fse-2025-research-papers/111/An-Empirical-Study-of-Code-Clones-from-Commercial-AI-Code-Generators)
- Liu, Widyasari, Zhao, Irsan, and Lo, *Debt Behind the AI Boom: A Large-Scale Empirical Study of AI-Generated Code in the Wild*, 2026. [arXiv](https://arxiv.org/abs/2603.28592)
