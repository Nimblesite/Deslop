---
layout: layouts/blog.njk
title: "如何在 Flutter 项目中查找重复的 Dart 代码"
date: 2026-06-05
author: Christian Findlay
tags:
  - posts
  - dart
  - flutter
  - ai-generated-code
  - duplicate-code
category: engineering
description: "结构分析如何在 Flutter 项目中发现经过重命名和近似重复的 Dart 代码，以及编码智能体如何在写下另一份副本前先检查。"
excerpt: "Flutter 项目中常会重复组件树、仓储层、映射器与测试初始化代码。本文说明 Deslop 如何比较其结构并把影响最大的重复排在前面。"
heroImage: "/assets/img/blog/deduplicating-dart-code-ai-flutter-header.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "展示 Flutter widget 树、克隆的 Dart 卡片以及 find-similar 门禁的头图。"
ogImage: "/assets/img/blog/deduplicating-dart-code-ai-flutter-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
ogImageAlt: "Deslop —— 当 AI 编写你的 Flutter 应用时为 Dart 代码去重。实时 LSP 与 MCP 重复代码服务器，按最严重者优先排序。"
lang: zh
---

AI 辅助的 Flutter 开发可能以略有差异的形式重复同一个 widget、仓储层或校验规则。代码可以通过编译和测试，却留下多份今后需要同步修复的实现。

Flutter 的 widget 树和 `build` 方法可能很长，因此部分重复的结构不一定会在代码审查中显眼。

完整的算法说明请参阅[研究背景](/zh/docs/research-background/)。本文是针对 Dart 与 Flutter 的版本。

## AI 辅助代码与重复

GitClear 的 [2025 年 AI Copilot 代码质量研究](https://www.gitclear.com/ai_assistant_code_quality_2025_research)报告称，其研究的仓库中，复制粘贴的代码行与重复代码块均有所增加。[Code Copycat Conundrum](https://arxiv.org/abs/2504.12608)则研究了 LLM 生成代码在字符、语句和代码块层面的重复。

这并不意味着 AI 编写的 Dart 天然有问题，但在 AI 辅助开发中进行仓库级重复检查仍然有价值。

## Flutter 中常见的重复位置

常见候选包括：

- `build` 方法中的 widget 树与重复布局片段；
- 主要区别只是名称的仓储层、数据映射器与校验路径；
- `copyWith` 方法、重试包装器与重复的 `*_test.dart` 初始化代码。

## 一次 Dart 重复代码检查应当查找什么

一次有用的检查不会止步于精确的行匹配。它应当找出四个层级的相似：

1. **精确重复代码** —— 同样的 Dart 被复制，只改动了格式或注释。
2. **重命名的重复代码** —— 同样的结构，标识符不同：一个 `CustomerCard` widget 被克隆成 `AccountCard`，`customerId` 换成了 `accountId`。
3. **近似重复代码** —— 逻辑大体相同，但有语句被插入、删除或重新排序：同一个表单校验多了一个分支。
4. **行为相同、代码不同** —— 两个 widget 或函数用不同的语法解决同一个问题（一个 `for` 循环对比一个 `map().toList()`）。

经典的克隆检测研究把这些称为 Type-1 到 Type-4。Deslop 的实现与研究参考见[研究背景](/zh/docs/research-background/)。

## 为什么对 Dart 而言行匹配不够

基于行的工具能抓住字面上的复制粘贴，但容易受表层变化影响：

- `CustomerCard` 变成 `AccountCard`，每个标识符都被重命名。
- 一个辅助方法被复制到另一个类里并重新缩进。
- `setState` 逻辑被改写成一个做同样事情的 `Riverpod` notifier。
- 同一条校验规则以不同的分支顺序被重建。

这就是为什么 Deslop 从解析后的语法树出发，而非从文本出发。它用 **tree-sitter** 解析每一个 `.dart` 文件，剥离标识符名和字面量名，使重命名的副本仍能匹配，对树的*结构*生成指纹，用兄弟窗口和 MinHash 把网撒向近似重复，并可选地为行为相同的匹配加入嵌入（向量嵌入）。简而言之：它先比较结构，从不比较行。完整的审计轨迹见[工作原理](/zh/docs/how-it-works/)。

## 动笔之前先检查

Deslop 的 MCP 服务器可以把检查移入编码智能体的编辑闭环。

如果你使用编码智能体进行 Flutter 开发，可以配置它在**编写新的组件、仓储层、映射器或测试初始化代码之前**调用 Deslop 的 `find-similar`。强匹配让智能体有机会复用或扩展已有实现。配置方法见 [AI 集成](/zh/docs/ai-integration/)。

## 排序：最严重的重复排在第一行

Deslop 使用 AST 节点数、额外副本数和经对数抑制的字节跨度，按实测影响为簇排名。确切公式见[工作原理](/zh/docs/how-it-works/#rank)。

JSON 报告直接向智能体提供簇顺序、字节范围、分桶与信号，无需维护另一套表示。

## 当你发现重复的 Dart 时该怎么办

不要把每一个克隆都当成 bug。把它当成一个决策。

- **抽取** —— 当这些副本明显是同一个抽象，并且会一起变更时：把重复的布局提取成一个自定义 widget，把共享样式提升到 `ThemeData` 中。
- **复用** —— 当其中一份实现应作为权威实现，其他实现应当调用它时。
- **接受** —— 当重复是有意为之时：生成的代码、测试夹具、平台垫片，或者今天看起来相似、但预期会分道扬镳的两条路径。
