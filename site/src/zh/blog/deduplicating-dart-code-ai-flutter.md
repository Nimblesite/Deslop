---
layout: layouts/blog.njk
title: "如何在 Flutter 项目中查找重复的 Dart 代码"
date: 2026-06-05
updated: 2026-11-11
author: Christian Findlay
tags:
  - posts
  - dart
  - flutter
  - ai-generated-code
  - duplicate-code
category: engineering
description: "结构分析如何在 Flutter 项目中发现经过重命名和近似重复的 Dart 代码，以及编码智能体如何在写下另一份副本前先检查。"
excerpt: "Flutter 项目中常会重复 widget 树、仓储层、映射器与测试初始化代码。本文说明 Deslop 如何比较它们的结构，并优先报告影响最大的重复。"
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

AI 辅助的 Flutter 开发可能以略有差异的形式复现同一个 widget、仓储层或校验规则。代码可以通过编译和测试，却留下多份今后需要同步修复的实现。

Flutter 的 widget 树和 `build` 方法可能很长，因此部分重复的结构并不总会在代码审查中显得突出。

完整的算法说明请参阅[研究背景](/zh/docs/research-background/)。本文是其中针对 Dart 与 Flutter 的版本。

## AI 辅助代码与重复

GitClear 的 [2025 年 AI Copilot 代码质量研究](https://www.gitclear.com/ai_assistant_code_quality_2025_research)报告称，在其研究的仓库中，复制粘贴的代码行与重复代码块都有所增加。[Code Copycat Conundrum](https://arxiv.org/abs/2504.12608) 则考察了 LLM 生成代码在字符、语句和代码块层面的重复。

这并不意味着 AI 编写的 Dart 天生有问题。它只是让仓库级重复检查在 AI 辅助开发中显得有用。

## Flutter 中常见的重复位置

常见的候选包括：

- `build` 方法中的 widget 树与重复的布局片段；
- 仅在名称上有差异的仓储层、数据映射器与校验路径；
- `copyWith` 方法、重试包装器，以及重复的 `*_test.dart` 初始化代码。

下面是一对典型例子。一个 `CustomerCard` widget 被复制用来构建 `AccountCard`：

```dart
class CustomerCard extends StatelessWidget {
  const CustomerCard({super.key, required this.customer});
  final Customer customer;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: ListTile(
        leading: CircleAvatar(child: Text(customer.initials)),
        title: Text(customer.name),
        subtitle: Text(customer.email),
        trailing: const Icon(Icons.chevron_right),
      ),
    );
  }
}
```

`AccountCard` 拥有相同的 `build` 方法体，只是把 `customer` 换成了 `account`。当设计变更给其中一张卡片加上徽章时，另一张就会跟不上。像这样复制的 `build` 方法体即使 widget 名称变了，也会保留在报告中。

## Deslop 在 Flutter 代码中不报告什么

Flutter 中存在大量必要的重复，报告它们会把真正的副本淹没。Deslop 将以下内容排除在外：

- **Widget shells.** 方法体只是构造 widget 并将其返回的 `StatelessWidget`、`StatefulWidget` 或 `State<T>` 声明属于框架脚手架。任何局部变量、控制流、算术运算、可变状态或闭包都会让它重新变成普通代码，而被复制的 `build` 方法体仍会被报告。
- **Renamed signatures alone.** 两个声明仅仅共享签名形状，并不构成方法体被复制的证据。
- **Generated Dart.** 以 `.g.dart`、`.freezed.dart`、`.gr.dart`、`.config.dart`、`.gen.dart`、`.mocks.dart` 结尾的文件以及 protobuf 的 `.pb*.dart` 系列会被分析，但从报告中隐藏；以生成器横幅（例如 `// GENERATED CODE - DO NOT MODIFY BY HAND`）开头的文件同样如此。
- **Tooling and dependencies.** `.dart_tool` 始终被排除；`.pub-cache` 默认被排除，除非你选择加入。

当*你的*代码与被隐藏的生成文件重复时，它们仍然会被计入。如果一个手写映射器复制了 `json_serializable` 已经生成的内容，这个簇仍会保留。完整清单见[配置](/zh/docs/configuration/#-3)。

## Dart 重复代码检查应当查找什么

有用的检查不会止步于精确的行匹配。它应当找出四个层级的相似：

1. **Exact duplicate code** —— 同样的 Dart 被复制，仅格式或注释有所改动。
2. **Renamed duplicate code** —— 结构相同，标识符不同：一个 `CustomerCard` widget 被克隆成 `AccountCard`，`customerId` 换成了 `accountId`。
3. **Near-duplicate code** —— 逻辑大体相同，但有语句被插入、删除或重新排序：同一个表单校验多出一个分支。
4. **Same behavior, different code** —— 两个 widget 或函数用不同的语法解决同一个问题（一个 `for` 循环对比一个 `map().toList()`）。

克隆检测研究将其称为 Type I 到 Type IV。Deslop 当前的标签，以及克隆与仅形状信息之间的区别，在[工作原理](/zh/docs/how-it-works/#-2)中说明。

## 为什么对 Dart 而言行匹配不够

基于行的工具能抓住字面上的复制粘贴，但对表层变化很敏感：

- `CustomerCard` 变成 `AccountCard`，每个标识符都被重命名。
- 一个辅助函数被复制到另一个类中并重新缩进。
- `setState` 逻辑被改写为做同样事情的 `Riverpod` notifier。
- 同一条校验规则以不同的分支顺序被重新构建。

这就是 Deslop 从解析后的语法树而非文本出发的原因。它用 **tree-sitter** 解析每个 `.dart` 文件，剥离标识符名与字面量名，让重命名的副本仍能匹配；对树的*结构*生成指纹；用兄弟窗口和 MinHash 把网撒向近似重复；并可选择为行为相同的匹配加入嵌入。结构负责找出候选；随后由内容检查确认这些副本执行的是相同的操作，因此仅仅共享布局的两个 widget 不会被报告为克隆。完整的审计轨迹见[工作原理](/zh/docs/how-it-works/)。

## 动笔之前先检查

Deslop 的 MCP 服务器可以把这项检查移入编码智能体的编辑闭环。

如果你在 Flutter 开发中使用编码智能体，可以配置它在**编写新的 widget、仓储层、映射器或测试初始化代码之前**调用 Deslop 的 `find-similar` 工具。强匹配让智能体有机会复用或扩展现有实现。配置方法见 [AI 集成](/zh/docs/ai-integration/)。

## 排序：最严重的重复排在第一行

Deslop 按重复体量为簇排名：基准副本的语法树大小乘以它的额外可见副本数。一个被复制到三个屏幕的 200 节点 `build` 方法，排名高于一个只被复制一次的简短辅助函数。公式及其推理见[Deslop 如何为重复代码簇排名](/zh/blog/ranking-formula/)与[工作原理](/zh/docs/how-it-works/#-4)。

JSON 报告直接向智能体提供簇顺序、字节范围和类别，无需单独的表示形式，VS Code 的簇面板显示相同的顺序。

## 发现重复的 Dart 时该怎么办

不要把每一个克隆都当成 bug。把它当成一个决策。

- **Extract** —— 当这些副本明显是同一个抽象并且会一起变更时：把重复的布局提取成自定义 widget，把共享样式提升到 `ThemeData` 中。
- **Reuse** —— 当其中一份实现已经是更好的权威实现、其余实现都应当调用它时。
- **Accept** —— 当重复是有意为之时：生成的代码、测试夹具、平台垫片，或者今天看起来相似、但预期将来会分道扬镳的两条路径。

## 把新的副本挡在拉取请求之外

清理已有的重复是一次性的工作。要把新的副本挡在外面，需要在工作流中加入检查：

- **In the editor.** VS Code 扩展会随着你的编辑更新其结果，因此被复制的 widget 会在你提交之前出现在簇面板中。
- **In the agent.** Claude Code、Cursor、Codex 和其他 MCP 客户端可以在生成 widget 或仓储类之前调用 `find-similar`。
- **In CI.** [GitHub Action](/zh/docs/github-action/) 在每个拉取请求上运行同一个 CLI。将 `--diff` 与 `--only-changed` 搭配使用，只评判拉取请求新增的行，这样存量技术债就不会让新的变更失败。

这些检查共同让 Flutter 代码库在 AI 辅助开发提速的同时保持一致：一个卡片 widget，每个资源一个仓储，每个字段一条校验规则。
