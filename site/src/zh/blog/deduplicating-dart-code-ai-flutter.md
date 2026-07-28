---
layout: layouts/blog.njk
title: "当 AI 编写你的 Flutter 应用时，如何为 Dart 代码去重"
date: 2026-06-05
author: Christian Findlay
tags:
  - posts
  - dart
  - flutter
  - ai-generated-code
  - duplicate-code
category: engineering
description: "AI 编写 Dart 的速度超过了你审查它的速度，而 Flutter 冗长的 widget 树又把重复隐藏了起来。本文讲解如何检测并去重克隆的 Dart 代码。"
excerpt: "编码智能体生成 Flutter widget、仓库和状态管理类的速度，没有任何审查者跟得上。Flutter 的冗长把这些副本藏了起来。本文讲解该检查什么、为什么基于行的工具会漏掉它，以及如何从结构层面为 Dart 去重。"
heroImage: "/assets/img/blog/deduplicating-dart-code-ai-flutter-header.png"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "展示 Flutter widget 树、克隆的 Dart 卡片以及 find-similar 门禁的头图。"
ogImage: "/assets/img/blog/deduplicating-dart-code-ai-flutter-og.png"
ogImageWidth: "1200"
ogImageHeight: "630"
ogImageAlt: "Deslop —— 当 AI 编写你的 Flutter 应用时为 Dart 代码去重。实时 LSP 与 MCP 重复代码服务器，按最严重者优先排序。"
lang: zh
---

如果你搜索的是 "Flutter code duplication"、"deduplicate Dart code" 或 "AI-generated Flutter technical debt"，简短的答案是：AI 不需要写出*错误*的 Dart 才能伤害一个 Flutter 代码库。它只需要把同一个 widget、同一个仓库或同一条校验规则写两遍——以两种略有差异的形态——而且速度快到没人能察觉。

Flutter 在隐藏这一点上格外擅长。widget 树天生就冗长，所以一个四十行的 `build` 方法，即便与三个文件之外的另一个方法有 80% 完全相同，也不会在审查中显眼。diff 看起来就是普通的 Flutter。应用能跑。而代码库现在有了两份实现，未来需要同样的修复。

完整的算法说明请参阅[研究背景](/zh/docs/research-background/)。本文是针对 Dart 与 Flutter 的版本。

## AI 提升的是代码被复制的速度，而非复制时的审慎程度

吞吐量的变化如今是被实测出来的，而非道听途说。GitClear 的 [2025 年 AI Copilot 代码质量研究](https://www.gitclear.com/ai_assistant_code_quality_2025_research)分析了 2.11 亿行变更代码，发现复制粘贴的行数从 2021 年的 8.3% 升至 2024 年的 12.3%，五行及以上的重复代码块增长了八倍。2024 年，复制粘贴的代码有记录以来首次超过了*移动*的代码——而"移动"的代码（即有人把逻辑整合成可复用之物的信号）跌至 10% 以下，同比下降 44%。

整个行业的论述都是一致的。LeadDev 关于 [AI 生成代码如何加速技术债务](https://leaddev.com/technical-direction/how-ai-generated-code-accelerates-technical-debt)的报道引用了 API 布道者 Kin Lane 的话："在我 35 年的技术生涯中，我想我从未见过在如此短的时间内产生如此多的技术债务。"研究方向也表示赞同——[Code Copycat Conundrum](https://arxiv.org/abs/2504.12608) 从字符、语句和代码块层面测量了 LLM 生成代码中的重复。

这一切都不意味着 AI 写出的 Dart 是糟糕的。它意味着 AI 写出的 Dart 应当接受与手写 Dart 同样的仓库级评估——只是要更快，因为它到来得更快。

## 为什么 Flutter 代码库比大多数代码库更容易重复

Flutter 有几个结构性特征，使得 AI 生成的重复更容易产生、也更难被发现：

- **widget 树很冗长。** 编码智能体会乐于把一张卡片、一个列表项或一个表单字段直接内联进 `build` 方法，而不是抽取出可复用的 widget。十个屏幕之后，你就有了十个几乎相同的卡片布局，每一个都被轻微调整过。
- **`build` 方法默认就很长。** 在 Flutter 中，长是正常的，因此一个又长又部分重复的方法不会触发通常那种"这个文件太大了"的直觉。
- **状态管理被混用。** AI 工具常常在同一个项目里混合 `Provider`、`Riverpod` 和 `Bloc` 的模式，对同一个状态问题产出几种形态各异的解决方案。
- **那些重复的层本就是有意重复的。** 仓库、数据映射器、`copyWith` 方法、重试包装器以及 `*_test.dart` 的 setup 代码块，全都遵循可预测的形态——正是 LLM 在一个又一个任务中会复现的那些形态。

Flutter 对这一切都有很好的答案。你可以抽取自定义 widget，把共享样式提升到 [`ThemeData`](https://docs.flutter.dev/cookbook/design/themes) 中，[用 hooks 减少 widget 样板代码](https://medium.com/@remirousselet/flutter-reducing-widgets-boilerplate-3e635f10685e)，或者对样板层依赖[代码生成](https://codewithandrea.com/articles/dart-flutter-code-generation/)。问题不在于缺工具。问题在于*察觉*——在第二份副本落地之前，意识到两段 Dart 是同一个抽象。

## 一次 Dart 重复代码检查应当查找什么

一次有用的检查不会止步于精确的行匹配。它应当找出四个层级的相似：

1. **精确重复代码** —— 同样的 Dart 被复制，只改动了格式或注释。
2. **重命名的重复代码** —— 同样的结构，标识符不同：一个 `CustomerCard` widget 被克隆成 `AccountCard`，`customerId` 换成了 `accountId`。
3. **近似重复代码** —— 逻辑大体相同，但有语句被插入、删除或重新排序：同一个表单校验多了一个分支。
4. **行为相同、代码不同** —— 两个 widget 或函数用不同的语法解决同一个问题（一个 `for` 循环对比一个 `map().toList()`）。

经典的克隆检测研究把这些称为 Type-1 到 Type-4——用 Deslop 的术语来说，就是完全相同的代码、几乎相同的代码、松散相似的代码，以及行为相同、代码不同。同样的分类法也适用于 Dart——文献中已有针对 Dart 的基于 token 和基于结构的克隆检测器，包括一个[专为 Dart 语言构建的基于 token 的克隆检测器](https://techniumscience.com/index.php/technium/article/view/6732)，以及 [Out of Step](https://www.sciencedirect.com/science/article/pii/S0167642324000352) 中跨语言的结构性工作，后者报告在 Dart 和 Kotlin 特性集上的相似度检测率超过 80%。

## 为什么对 Dart 而言行匹配不够

基于行的工具能抓住字面上的复制粘贴。但只要智能体改变了表层形态——而它无时无刻不在这么做——它们就会立刻失明：

- `CustomerCard` 变成 `AccountCard`，每个标识符都被重命名。
- 一个辅助方法被复制到另一个类里并重新缩进。
- `setState` 逻辑被改写成一个做同样事情的 `Riverpod` notifier。
- 同一条校验规则以不同的分支顺序被重建。

这就是为什么 Deslop 从解析后的语法树出发，而非从文本出发。它用 **tree-sitter** 解析每一个 `.dart` 文件，剥离标识符名和字面量名，使重命名的副本仍能匹配，对树的*结构*生成指纹，用兄弟窗口和 MinHash 把网撒向近似重复，并可选地为行为相同的匹配加入嵌入（向量嵌入）。简而言之：它先比较结构，从不比较行。完整的审计轨迹见[工作原理](/zh/docs/how-it-works/)。

## 预防胜于清理：动笔之前先发问

事后去重是每个静态分析器都已经在做的事。Deslop 的优势在于**实时位于智能体的内循环之中**，从而让第二份副本永远不落地。

如果你通过编码智能体（Claude Code、Cursor、Copilot、Codex、Continue）来推进 Flutter 工作，就把它接上 Deslop 的 `find-similar` 工具，**在它编写新的 widget、仓库、映射器或测试 setup 之前**调用。如果已经存在一个高相似度的匹配，智能体就会复用那个规范实现，而不是去撰写第二份副本。质量不再是周期性的审计，而是变成一个持续的循环——这正是 [Dart/Flutter 工具社区借助 MCP 驱动代码质量](https://dcm.dev/blog/2025/08/25/agentic-code-quality-dcm-mcp/)所走的同一个方向。配置方法见 [AI 集成](/zh/docs/ai-integration/)。

## 排序：最严重的重复排在第一行

一份有 200 条无序发现的重复代码报告，不过是又一个待办积压。Deslop 按影响对簇排序——克隆大小 × 克隆数量 × 跨越行数——所以第一项是收益最高的目标，而非字母序最靠前的那个。

这对智能体而言比对人类更重要。智能体不需要一整面墙的克隆数据；它需要一个小而结构化的答案：哪个簇最重要、字节范围在哪里、为什么被标记，以及信号来自结构、token 还是嵌入。这就是为什么 Deslop 以 JSON 为先，也是为什么会有 LSP 和 MCP 这两个表面——AI 制造重复 Dart 的速度很快，所以反馈必须紧挨着编辑而生。

## 当你发现重复的 Dart 时该怎么办

不要把每一个克隆都当成 bug。把它当成一个决策。

- **抽取** —— 当这些副本明显是同一个抽象，并且会一起变更时：把重复的布局提取成一个自定义 widget，把共享样式提升到 `ThemeData` 中。
- **复用** —— 当其中一份实现已经是更好的事实来源，其他应当调用它时。
- **接受** —— 当重复是有意为之时：生成的代码、测试夹具、平台垫片，或者今天看起来相似、但预期会分道扬镳的两条路径。

错误不在于接受重复。错误在于*意外地*接受它，因为没人去度量它。

## 常见问题

### Deslop 支持 Dart 吗？

支持。Dart 如今是一等公民语言，与 C#、Rust、Python、JavaScript、TypeScript、PHP、F# 和 Go 一样用 tree-sitter 解析。Java 与 C/C++ 在路线图上。

### 重复代码总是技术债务吗？

不是。有些重复是有意的，而且比取代它的抽象更划算。Deslop 把证据呈现出来；它并不强迫你重构。

### 我能不能直接让 LLM 审查它自己写的 Flutter 代码有没有重复？

有时候可以。但一份确定性的报告要容易审计得多——它指向文件、字节范围、信号评分和 schema 字段。智能体读取它，*然后*制定重构计划。

### 这只是 AI 的问题吗？

不是。克隆检测的文献比现代 LLM 早了几十年，而 Flutter 从 1.0 起就一直很冗长。AI 之所以重要，是因为它提升了重复 Dart 出现的速度——而且，根据 GitClear 的数据，也提升了它事后极少被整合的程度。

AI 生成的 Dart 并不会自动成为糟糕的 Dart。但如果它产出同一个 widget 的速度比你的团队审查它的速度还快，那么 Flutter 的维护账单就是真实存在的。趁代码还新鲜时，就度量它。
