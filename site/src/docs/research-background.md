---
layout: layouts/docs.njk
title: Research Background — Code-clone detection algorithms
description: Read the research behind Deslop's structural, token, MinHash LSH, and embedding-based code-clone detection, with links to the source papers.
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

Type I describes exact copies apart from layout and comments; Type II allows renaming and changed literals; Type III includes edited statements; Type IV describes equivalent behaviour implemented differently. Deslop's current labels and stricter text-identity rule are explained in [How It Works](/docs/how-it-works/#read-the-categories). Shape-only findings are outside this taxonomy and do not count as duplication.

## Implementation

- Tree-sitter parsing and normalization identify structural candidates: `crates/deslop-core/src/lang/`.
- Subtree and sibling-run fingerprints locate repeated code: `fingerprint.rs` and `sibling.rs`.
- MinHash and LSH retrieve near-miss candidates: `lsh.rs`, `lsh/banding.rs` and `tokens.rs`.
- Optional embeddings add semantic candidates: `embedding/`.
- Pair admission checks content as well as structure: `pair/content_gate.rs`. Explicit comparisons expose pair evidence; clusters do not inherit one pair's confidence.
- Reports rank surviving clones by duplicated mass: `report_weight.rs`. Percentage calculations live in `report_metrics.rs`.

These are detection techniques, not proofs of safe refactoring. The [accuracy documentation](/docs/accuracy-transparency/) explains the independent checks.

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
