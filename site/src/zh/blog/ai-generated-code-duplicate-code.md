---
layout: layouts/blog.njk
title: "AI 生成的代码与重复代码：该检查什么"
date: 2026-04-23
author: Christian Findlay
tags:
  - posts
  - ai-generated-code
  - technical-debt
  - duplicate-code
category: engineering
description: "AI 生成的代码可能制造重复代码和技术债务。了解如何借助代码克隆检测来检查这些问题，以及 Deslop 如何审计 AI 时代的代码库。"
excerpt: "AI 生成的代码可能在代码审查发现之前就成倍增加重复逻辑。本指南讲解该检查什么、为什么重复代码会变成技术债务，以及在哪里阅读完整的 Deslop 研究背景。"
heroImage: "/assets/img/blog/ai-generated-code-duplicate-code-header.png"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "标题图，展示针对 AI 生成代码的四层重复代码检查清单。"
lang: zh
---

如果你搜索过「AI 生成代码 技术债务」「重复代码检测」或「代码克隆检测」，那么务实的答案是这样的：AI 并不需要生成有缺陷的代码，就能让一个代码库更难维护。它只需在有人注意到之前，用两种略有差异的形态把同一个想法实现两遍即可。

这就是 AI 时代的重复代码问题。代码可能能够编译。测试可能能够通过。拉取请求看上去可能也合情合理。但此时仓库里已经有了两份实现，将来需要同样的修复。

完整的实现脉络，请参阅 Deslop 文档页面：[研究背景](/zh/docs/research-background/)。本文是面向那些正在权衡该先检查什么的团队的精简版本。

## 这些搜索词都是大白话

这一领域的 Google Trends 主题推荐并不是「Type-2 克隆」或「Type-3 近似克隆」这类学术标签。它们是工程师和管理者真正会搜索的短语：

- AI 生成的代码
- 技术债务
- 重复代码
- 代码重复
- 代码克隆检测
- vibe coding（凭感觉写代码）

这些短语之所以重要，是因为它们描述的是实际运营层面的问题。一个团队通常不会问「我有 Type-3 克隆吗？」它问的是「AI 是不是刚把同一条业务规则塞进了三个地方？」

## AI 生成的代码会制造技术债务吗？

会的。这种风险并不神秘，也并非 AI 独有。几十年来人类一直在复制代码。变化的是吞吐量。

AI 编码助手能够快速产出一个看似合理、仓库形态的答案。当提示词与此前的任务相似时，答案往往也会呈现出熟悉的形态：又一个仓储类、又一个校验函数、又一个映射器、又一个重试包装器、又一个测试夹具。这在当下很有用，在日后则代价高昂。

研究方向也在朝同一个方向发展：

- [Code Copycat Conundrum](https://arxiv.org/abs/2504.12608) 研究了 LLM 生成代码在字符、语句和代码块层面的重复现象。
- [An Empirical Study of Code Clones from Commercial AI Code Generators](https://conf.researchr.org/details/fse-2025/fse-2025-research-papers/111/An-Empirical-Study-of-Code-Clones-from-Commercial-AI-Code-Generators) 报告了所研究的商用代码生成器可量化的 Type-1 和 Type-2 克隆率。
- [Debt Behind the AI Boom](https://arxiv.org/abs/2603.28592) 研究了生产仓库中由 AI 编写的提交所引入的技术债务。

这些都不意味着每一行 AI 写的代码都是糟糕的。它意味着 AI 生成的代码应当接受与人类代码相同的、仓库级别的审视，只是要更快。

## 重复代码检查应当查找什么？

一项实用的 AI 时代重复代码检查不应止步于精确的行匹配。它应当查找四个层次的相似性：

1. **精确重复代码**：复制后仅改动了格式或注释的相同代码。
2. **重命名的重复代码**：结构相同、但变量名或常量不同。
3. **近似重复代码**：逻辑大体相同，但插入、删除或重排了若干语句。
4. **行为相同、代码不同**：用不同语法解决同一问题的两份实现。

经典的代码克隆检测研究将它们称为 Type-1、Type-2、Type-3 和 Type-4 克隆。Deslop 采用了这些思想，但详细的算法说明放在了[研究背景](/zh/docs/research-background/)中，因此本文不再重复整个文档页面的内容。

## 为什么行匹配还不够

基于行的重复代码工具擅长找出显而易见的复制粘贴。当 AI 改变了表层形态时，它们就力不从心了：

- `customerId` 变成了 `accountId`。
- `foreach` 变成了推导式。
- 一个辅助函数被复制后挪进了另一个类。
- 同一条校验规则以不同的分支顺序被重写。

这正是 Deslop 从解析后的语法树而非原始文本入手的原因。它用 tree-sitter 解析每个文件，然后剥离标识符名和字面量名，使得重命名后的副本仍能匹配。它对树结构进行指纹提取，借助兄弟节点窗口和 MinHash 把网撒向近似重复，并可选择性地加入嵌入（向量嵌入）以匹配行为相同的代码。简而言之：它先比较代码结构，而不是先比较行。

完整的审计链路见[工作原理](/zh/docs/how-it-works/)和[研究背景](/zh/docs/research-background/)。

## 发现重复代码后该怎么办

不要把每个克隆都当成一个 bug。把它当成一个决策。

**提取（Extract）**：当这些副本显然是同一个抽象、并且会一起变化时。

**复用（Reuse）**：当其中一份实现已经是更好的事实来源、其余的应当调用它时。

**接受（Accept）**：当重复是有意为之时——夹具、生成代码、兼容垫片，或两条现在看起来相似、但预期会分道扬镳的路径。

错误不在于接受重复，而在于因为无人度量而在不知不觉中接受了它。

## 为什么 Deslop 要给发现项排名

一份包含 200 条无序发现项的重复代码报告，不过是又一个待办积压。Deslop 按影响对簇进行排名，使得第一项就意味着是回报最高的审查目标。

这对 AI 编码智能体很重要。智能体不需要一堵克隆数据的墙。它需要一个小而结构化的答案：

- 哪个重复簇最重要，
- 字节范围在哪里，
- 这个簇为什么被标记，
- 信号来自结构、词法相似度，还是嵌入。

这就是 Deslop 以 JSON 为先的原因，也是 LSP/MCP 路径存在的原因。AI 能够快速制造重复代码；反馈回路就必须同样贴近编辑动作。

## 常见问题

### 重复代码总是技术债务吗？

不是。有些重复代码是有意为之，比起它将要替换的那个抽象更省事。Deslop 的职责是把证据呈现出来，而不是强制重构。

### 这只是 AI 的问题吗？

不是。代码克隆方面的文献比现代 LLM 早了几十年。AI 之所以重要，是因为它能加快重复逻辑出现的速度。

### LLM 能不能自己审查自己的代码是否重复？

有时可以，但确定性的报告更便于审计。Deslop 会指向文件、字节范围、信号评分和报告 schema 字段。智能体可以读取那份报告，然后制定重构计划。

### 学术背景在哪里？

先从 [Deslop 的研究背景](/zh/docs/research-background/)开始，再顺着链接的论文继续。最相关的入口是：

- [Clone Detection Using Abstract Syntax Trees](https://leodemoura.github.io/files/ICSM98.pdf)
- [Syntax Tree Fingerprinting for Source Code Similarity Detection](https://igm.univ-mlv.fr/~chilowi/research/syntax_tree_fingerprinting/syntax_tree_fingerprinting_ICPC09.pdf)
- [SourcererCC: Scaling Code Clone Detection to Big Code](https://arxiv.org/abs/1512.06448)
- [Unveiling the potential of large language models in generating semantic and cross-language clones](https://arxiv.org/abs/2309.06424)
- [Are Classical Clone Detectors Good Enough For the AI Era?](https://arxiv.org/abs/2509.25754)

AI 生成的代码并非天然就是糟糕的代码。但如果它制造重复代码的速度快过你团队的审查速度，那么维护账单就是实实在在的。趁代码还新鲜时去度量它。
