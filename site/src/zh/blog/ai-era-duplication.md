---
layout: layouts/blog.njk
title: 为什么编码智能体需要快速的重复代码反馈
date: 2026-04-20
updated: 2026-11-11
author: Christian Findlay
tags: posts
description: 编码智能体可能重复仓库中已有的逻辑。快速的 LSP 与 MCP 反馈可以在编辑过程中呈现这些重复。
excerpt: 编码智能体不会自动知道仓库中已有相似代码。这就是重复代码反馈需要进入编辑闭环的原因。
heroImage: "/assets/img/blog/ai-era-duplication-header.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "标题图，展示在 AI 时代的仓库中，重复代码的增长速度超过了功能审查的速度。"
ogImage: "/assets/img/blog/ai-era-duplication-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
lang: zh
---

编码智能体可以在不知道仓库其他地方已有相似代码的情况下，产出一个看似合理的实现。这种模式在多个功能开发中反复出现，就会留下多份实现，而它们今后都必须接受同样的修复。

问题出在仓库上下文，而不是意图。即使项目中已经存在一个同类实现——只是名称不同——模型仍可能再生成一个仓储类、校验器或映射器。

## AI 智能体为何最终把同样的代码写了两遍

Claude Code、Cursor、GitHub Copilot 和 Codex 都基于所给的上下文工作：打开的文件、提示词、智能体搜索过的内容，以及它的工具返回的结果。大型仓库很少能完整装进这个窗口。当智能体需要一个日期格式化器或一个重试包装器时，它有两个选择：搜索现有的，或者新写一个。新写往往是更快让测试通过的路径。

结果很少是一字不差的复制，而是换了新名字的同一个想法：`parseOrderDate` 挨着一个已有的 `toInvoiceDate`，第二个超时设置略有不同的 HTTP 客户端包装器，一条分支顺序重排后重建的校验规则。每个版本都能编译，每个都能通过自己的测试。重复代码只在后来才显出分量：一次缺陷修复或安全补丁落在了其中一份副本上，另一份却没有。

[Vibe coding](https://en.wikipedia.org/wiki/Vibe_coding)——开发者大体上只看生成代码“似乎能工作”就予以接受——让这种情况更容易发生。代码被接受得越快，审查者回想起“同样的逻辑就在三个文件夹之外”的机会就越小。

## 研究表明了什么

这一风险有文献记载，而不只是坊间传闻：

- [*Code Copycat Conundrum*](https://arxiv.org/abs/2504.12608) 研究了 19 个代码 LLM，报告了字符、语句和块级别的重复，包括结构上冗余的代码。
- [*An Empirical Study of Code Clones from Commercial AI Code Generators*](https://conf.researchr.org/details/fse-2025/fse-2025-research-papers/111/An-Empirical-Study-of-Code-Clones-from-Commercial-AI-Code-Generators)（FSE 2025）测量了商业代码生成器输出中 Type-1 和 Type-2 克隆的出现率。
- [*Debt Behind the AI Boom*](https://arxiv.org/abs/2603.28592) 追踪了真实仓库中经确认由 AI 编写的提交所引入的静态分析问题。

这些并不意味着 AI 编写的代码比人写的代码更差。人类也会复制粘贴。它说明的是新代码的量增加了，重复代码检查也应该与之保持同步。[研究背景](/zh/docs/research-background/)汇总了完整的阅读清单。

## 为什么反馈时机很重要

在代码及其上下文仍处于活跃状态时，修改一处重复的成本更低。等到 CI 或审查之后才发现，就要多出一次交接，才能有人决定是抽取、复用还是接受它。

考虑一份副本通常会经历的几种走向：

| 重复何时被发现 | 代价 |
| --- | --- |
| 在智能体编写它之前 | 一次工具调用。智能体复用现有的函数。 |
| 在文件仍打开时 | 一次快速修改。作者仍然掌握着上下文。 |
| 在 AI 代码审查或 CI 中 | 一轮新的往返：评论、修改、重跑。 |
| 数月之后 | 一处缺陷在一份副本中被修复，另一份却被遗漏。 |

越靠前的行成本越低，因为没有人需要重建上下文。这正是 Deslop 把分析放进编辑闭环、而不是只放进夜间任务的原因。

## Deslop 在闭环中做什么

Deslop 把分析留在编辑闭环中。LSP 监视工作区并增量更新报告；MCP 服务器则通过 `find-similar` 和报告查询，把这份正在运行的分析提供给编码智能体。

[Language Server Protocol](https://microsoft.github.io/language-server-protocol/) 是 VS Code 等编辑器接收诊断的方式。Deslop 的语言服务器把当前报告保存在内存中，并随文件变化刷新它，因此簇面板和诊断会跟随你的编辑。

[Model Context Protocol](https://modelcontextprotocol.io/) 是 AI 智能体调用外部工具的方式。Deslop 的 MCP 服务器读取的是同一份实时报告，而不是重新发起一次扫描。编写代码时最重要的工具是 `find-similar`：在写下新代码之前，智能体把自己拟写的代码片段传入，取回任何结构相同的现有代码。`duplicates`、`cluster-by-id` 和 `compare-pair` 等只读查询则覆盖审查与清理。[AI 智能体](/zh/docs/ai-integration/)展示了如何连接 Claude Code、Codex、Claude Desktop 和其他 MCP 客户端。

一条实用的智能体指令如下：

```text
Before writing a new function, call find-similar with the proposed code.
If it returns a match, reuse or extend the existing implementation.
After editing, call duplicates to confirm no new cluster appeared.
```

CLI 覆盖编辑器之外的一切：一次性扫描、用 `--diff` 把合并前检查限定在差异范围内，或者用 `--fail-over` 设置 CI 门禁。CLI、编辑器和 MCP 服务器运行的是同一个 Rust 引擎，因此同一个发现在每一处的含义都相同。

## 拿到一个发现该怎么办

Deslop 报告中的一个簇是一个决策，而不是一纸定论。工具负责报告；由你来决定。大体上有三条路径：

- **提取。** 这些副本是同一个抽象，并且会一起变化，因此共享函数是显而易见的答案。诸如 Extract Function（提炼函数）这样的经典[重构手法](https://martinfowler.com/books/refactoring.html)可以直接套用。JSON 报告包含 `action_hints`——每个类别的一份简短行动手册——让智能体在动手修改之前就能分辨出一模一样的副本和较松散的匹配。
- **复用。** 其中一份副本是“真正的”实现，其余的都应当调用它。挑出测试最完善的那一份，删掉其余的。
- **接受。** 有些重复是有意为之的：测试夹具、引导初始化，以及两个今天看起来相似、日后会分道扬镳的东西。使用 [`report_hide` 或 `exclude`](/zh/docs/configuration/#exclude--report_hide--)，让有意保留的副本不进入头条数字。Deslop 不作评判；它只是记分。

一个列出了各份副本、它们的精确位置和所属类别的簇，能给 AI 代码审查提供具体的讨论对象，而不是一句含糊的“这看起来重复了”。

## 保持信号精简

智能体的上下文中并不需要一整面墙的克隆数据。Deslop 按[重复质量](/zh/blog/ranking-formula/)为簇排名，因此被复制最多的代码排在最前面，而 MCP 工具会先返回简短摘要，再给出任何细节。智能体读取最上面的发现，做出决定，进行编辑，然后再次询问。此时报告已经是最新的。

这正是快速重复代码反馈的意义：在代码编写的同时就防止技术债堆积，而不是等到将来不得不再安排一次清理。
