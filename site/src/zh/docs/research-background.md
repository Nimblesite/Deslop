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

Type I 是除布局和注释外的精确副本；Type II 允许重命名和字面量变化；Type III 包含语句编辑；Type IV 描述不同实现中的等价行为。Deslop 当前标签及更严格的文本一致性规则见[工作原理](/zh/docs/how-it-works/)。仅形状相同的结果不属于克隆分类，也不计入重复率。

## 实现

- tree-sitter 解析和归一化寻找结构候选：`crates/deslop-core/src/lang/`。
- 子树和连续语句指纹定位重复代码：`fingerprint.rs` 和 `sibling.rs`。
- MinHash 与 LSH 检索近似候选：`lsh.rs`、`lsh/banding.rs` 和 `tokens.rs`。
- 可选嵌入分析补充语义候选：`embedding/`。
- 配对准入同时检查内容和结构：`pair/content_gate.rs`。明确的比较展示配对证据；簇不继承某一对的置信度。
- 报告按重复质量排列克隆：`report_weight.rs`。百分比计算位于 `report_metrics.rs`。

这些是检测技术，不是安全重构的证明。独立检查见[准确性文档](/zh/docs/accuracy-transparency/)。

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
