---
layout: layouts/docs.njk
title: 研究背景 — 代码克隆检测算法
description: 了解 Deslop 的结构、词元、MinHash LSH 与嵌入式代码克隆检测所依据的研究，并查阅原始论文。
eleventyNavigation:
  key: 研究
  order: 8
icon: science
docsGroup: trust
lang: zh
---

# 研究背景

Deslop 组合了成熟的克隆检测技术：归一化语法树、Merkle 指纹、兄弟窗口、MinHash LSH 与可选的嵌入搜索。下表中的实现指针标明了每项已交付技术所在的位置。

## 为什么 AI 改变了克隆问题

在 AI 编码助手出现之前，代码克隆就已经是一种维护风险。既有的担忧并非每一处重复都自动算错；而是被复制的逻辑必须在各类修复、安全补丁和功能变更中保持一致。AI 辅助开发改变了经济账：再生成一个相似实现的成本下降了，因此重复逻辑可能在日常功能开发中进入仓库，而不再只是通过刻意的复制粘贴。

近期研究支持这种风险模型：

- **LLM 代码重复**：[*Code Copycat Conundrum: Demystifying Repetition in LLM-based Code Generation*](https://arxiv.org/abs/2504.12608) 研究了 19 个代码 LLM，报告称重复出现在字符、语句和块级别，包括结构上冗余的代码。该论文还在开源和工业环境中评估了一种重复缓解技术。
- **LLM 生成的克隆**：[*Unveiling the potential of large language models in generating semantic and cross-language clones*](https://arxiv.org/abs/2309.06424) 评估了 GPT-3 生成语义克隆和跨语言克隆变体的能力，这与 Type-4 及跨语言重复检测直接相关。
- **商用 AI 代码生成器**：[*An Empirical Study of Code Clones from Commercial AI Code Generators*](https://dl.acm.org/doi/10.1145/3729397)（FSE 2025）报告称，所研究的商用生成器的 Type-1 和 Type-2 克隆率高达 7.50%，并讨论了版权、缺陷传播和漏洞传播的风险。
- **AI 时代的克隆检测**：[*Are Classical Clone Detectors Good Enough For the AI Era?*](https://arxiv.org/abs/2509.25754) 在 GPTCloneBench 及传统克隆基准上评估了九种克隆检测器，凸显了当克隆由 AI 生成时归一化与语义变化为何至关重要。
- **生产仓库中的技术债**：[*Debt Behind the AI Boom: A Large-Scale Empirical Study of AI-Generated Code in the Wild*](https://arxiv.org/abs/2603.28592) 分析了经核实的 AI 编写的提交，并跟踪这些提交引入的静态分析问题。它的范围比重复更广，但支持将 AI 生成的代码视为需要在仓库层面进行评估的技术债来源。

## 克隆分类法

Deslop 遵循代码克隆文献中通用的标准克隆分类法：

| 克隆类别 | 含义 | Deslop 信号 |
| --- | --- | --- |
| Type-1 | 除布局或注释外完全复制的文本 | 解析与归一化后的结构哈希 |
| Type-2 | 结构相同，但标识符被重命名或字面量被改动 | 标识符/字面量折叠后的结构哈希 |
| Type-3 | 插入、删除或改动了语句的近似克隆 | 兄弟窗口指纹与词元 MinHash LSH |
| Type-4 | 行为相似但语法或结构不同 | 可选的嵌入余弦相似度 |

公开报告的分桶在 `crates/deslop-core/src/buckets.rs` 中实现。代码将信号三元组映射到五个线协议标签：`identical`、`nearly_identical`、`structural_only`、`loosely_similar` 和 `same_behavior`。`structural_only` 桶标记那些唯一证据是归一化 AST 形状的簇；它们默认在排名中降权。`same_behavior` 桶只有在嵌入信号足够强时才可达。

## 算法基础

每一行都将一条研究脉络与受其影响的已交付实现对应起来。

| 研究脉络 | Deslop 从中采纳了什么 | 状态与实现指针 |
| --- | --- | --- |
| [Baxter et al. 1998 — AST 克隆检测](https://ieeexplore.ieee.org/document/738528) | 将代码解析为语法树，归一化无关的拼写，并比较树结构而非原始文本。 | ✅ `crates/deslop-core/src/lang/shared.rs`、`lang/csharp.rs`、`lang/rust_lang.rs`、`lang/python.rs`、`lang/dart.rs`（通过 `pipeline/corpus.rs::default_parsers` 注册） |
| Chilowicz et al. 2009 — 语法树指纹 | 对子树进行哈希，使精确的结构克隆产生相等的指纹；通过兄弟序列扩展覆盖近似克隆。 | ✅ `crates/deslop-core/src/fingerprint.rs::collect_non_boilerplate_fingerprints` 中的自底向上 BLAKE3 Merkle，以及 `crates/deslop-core/src/sibling.rs::collect_non_boilerplate_sibling_fingerprints` 中宽度 2..8 的兄弟窗口 |
| [SourcererCC (Sajnani et al. 2016)](https://arxiv.org/abs/1512.06448) | 用词元 k-gram 与 Jaccard 相似度进行可扩展的近似检测。 | ✅ 调整为基于**归一化 AST 种类 k-gram**而非原始源词元：`crates/deslop-core/src/tokens.rs`、`crates/deslop-core/src/pipeline/signatures.rs` |
| [MinHash (Broder 1997)](https://ieeexplore.ieee.org/document/666900) | 从紧凑签名估计 Jaccard。 | ✅ `crates/deslop-core/src/lsh.rs::minhash_signature` 中的 128 值签名；由 `estimate_jaccard` 估计 Jaccard |
| LSH 分带 (Indyk & Motwani 1998) | 以亚线性时间将相似指纹分桶。 | ✅ `crates/deslop-core/src/lsh.rs::band_collisions` 中的 32 带 × 4 行 |
| In Defense of MinHash Over SimHash (Shrivastava & Li 2014) | 对二值化特征使用 MinHash 而非 SimHash。 | ✅ 选用 MinHash；未使用 SimHash 和 Winnowing |
| 神经语义克隆检测 (CodeBERT、GraphCodeBERT、UniXCoder) | 将嵌入用作 Type-4 克隆的召回层。 | ✅ `crates/deslop-core/src/embedding/provider.rs` 中的 `EmbeddingProvider` trait；`embedding/ollama.rs` 中的 Ollama 提供方；默认模型 `nomic-embed-text` |
| [SSCD (Ahmed et al., Wiley 2024) — 规模化的 BERT + ANN](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3355) | 将基于 BERT 式嵌入的 HNSW ANN 作为 Type-3/4 召回路径。 | ✅ 带确定性种子的 `instant-distance` HNSW、top-k 检索、余弦阈值 0.80：`crates/deslop-core/src/embedding/pairs.rs` |
| [Ensemble-LLM 2025 (arXiv 2510.15480) — max/sum 融合](https://arxiv.org/abs/2510.15480) | 强信号不应被平均值稀释。 | ✅ 候选准入在 `crates/deslop-core/src/pair.rs::PairScore::bounded_fused` 中取结构、词元与嵌入信号的有界最大值。报告渲染随后在 `crates/deslop-core/src/buckets/gate.rs::content_gated_signals` 中，对结构信号饱和但内容并不相同的匹配应用内容证据门控。 |
| 混合克隆检测（没有纯 RAG 论文推荐使用纯嵌入） | 对结构、词元与嵌入配对取并集，然后融合并聚簇。 | ✅ `crates/deslop-core/src/pair.rs::candidate_pairs`，`crates/deslop-core/src/cluster.rs` 中的传递闭包 |
| 样板过滤（成熟工具的惯例） | 在指纹化前丢弃 import / namespace / decorator 克隆；以低噪声提示的形式重新呈现。 | ✅ `crates/deslop-core/src/boilerplate.rs` 与 `report_boilerplate.rs` |
| Autofix `refactor.extract`（LSP 代码动作） | 将 Type-1 簇重写为单一的共享方法。 | ✅ 已交付 — 规范见 [`docs/specs/autofix-extract.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/autofix-extract.md) |

## 参考文献

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
