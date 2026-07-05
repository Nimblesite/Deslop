---
layout: layouts/blog.njk
title: "面向编码智能体的 MCP：在重复代码落地之前就拦住它"
date: 2026-06-12
updated: 2026-06-12
author: Christian Findlay
tags:
  - posts
  - mcp
  - lsp
  - ai-coding-agents
  - duplicate-code
category: engineering
description: "Deslop 的实时 MCP + LSP 服务器如何让编码智能体在写代码之前调用 find-similar，在仓库仍在变化时就阻止重复代码。"
keywords: "Model Context Protocol, MCP 服务器, AI 编码工具, 编码智能体, 重复代码, 代码重复, 重复代码分析, 技术债务, AI 生成代码, LSP 服务器, find-similar"
excerpt: "MCP 为编码智能体提供工具。Deslop 为它们提供实时的仓库记忆：在写代码之前调用 find-similar，然后复用已经存在的代码。"
heroImage: "/assets/img/blog/live-mcp-lsp-duplicate-code-prevention-header.png"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "页头图片，展示一个实时的 MCP 与 LSP 边车在重复代码到达仓库之前将其拦截。"
ogImage: "/assets/img/blog/live-mcp-lsp-duplicate-code-prevention-og.png"
ogImageWidth: "1200"
ogImageHeight: "630"
ogImageAlt: "Deslop 实时 MCP 与 LSP 边车在重复代码落地之前将其阻止。"
lang: zh
---

等到重复代码出现在 CI 中时，智能体早已把它写好，拉取请求里也已经包含了它，而审查者不得不追问：为什么同一条规则如今存在于两个地方。

Deslop 把这项检查提前了。当编码智能体准备添加一个 mapper、校验器、widget 或测试辅助函数时，它可以向实时的工作区询问：这样的结构是否已经存在。

## MCP 如今是智能体的工具层

Model Context Protocol 为 AI 编码工具提供了一种触及模型之外世界的标准方式。这很有用，但仅有 MCP 并不能让一个工具变得可信。一份过时的重复代码报告仍然是过时的——即便编码智能体可以通过 MCP 服务器调用它。

Deslop 对 MCP 的使用刻意保持专注：在智能体正在决定写什么的那一刻，向它暴露实时的重复代码分析。这就把重复代码从一份事后的技术债务报告，变成了一个预防的闭环。

这与整个协议生态相吻合。官方的 [Model Context Protocol 介绍](https://modelcontextprotocol.io/docs/getting-started/intro)将 MCP 定义为一项用于把 AI 应用连接到外部系统的开放标准。[MCP 工具规范](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)指出，工具让模型得以调用外部系统。VS Code 自己的文档如今也讲解了如何[添加和管理 MCP 服务器](https://code.visualstudio.com/docs/agent-customization/mcp-servers)，而 OpenAI 的 Agents SDK 也提供了[对 MCP 的支持](https://openai.github.io/openai-agents-python/mcp/)，用于把智能体连接到 MCP 服务器。

对 Deslop 而言，产品层面的问题很简单：智能体能否在写下第二份副本之前先检查一下实时的仓库？

## 仅有 MCP 并不等于仓库记忆

一个 MCP 服务器可以暴露一个工具。但这并不意味着这个工具是新鲜的、廉价的，或者可以频繁安全地调用。

对重复代码预防而言，新鲜度就是产品的全部。AI 编码智能体会快速修改文件。人类会在编辑器里改动另一个文件。格式化工具会运行。测试生成器会添加一个 fixture。会话开始时跑出的一次性 CLI 报告，在第一次编辑之后就已经过时了。

Deslop 的架构被刻意拆分开来：

- **LSP 服务器**与编辑器共处，监视工作区，并让当前的重复代码报告始终保持热态。
- **MCP 服务器**暴露面向智能体的工具，并把读取和计算调用委托给那个正在运行的 LSP 进程。
- **CLI** 仍然是用于 CI 关卡和一次性审计的冷缓存后备方案。

这种拆分顺应了两种协议各自的形态。[Language Server Protocol 规范](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)标准化了编辑器与语言服务器之间的 JSON-RPC 消息。VS Code 的[语言服务器指南](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide)把语言服务器描述为一种为诊断、跳转到定义以及跨客户端的语言智能等编辑器功能提供支撑的方式。相比之下，MCP 则是智能体的工具接口。

LSP 是人类看见重复的方式。MCP 是智能体避免写出重复的方式。

## 新的闭环：先询问，再动笔

Deslop 的基石工具是 `find-similar`。

规则很简单：

```text
Before writing a new function, class, helper, widget, mapper, or test setup,
call find-similar with the snippet or range you are about to create.
If a high-similarity cluster already exists, reuse the canonical occurrence
instead of creating another copy.
```

这听起来是件小事。其实不然。它把重复代码检测从“事后再审计这堆烂摊子”，转变为“在副本出现之前先查询工作区”。

这个闭环是这样的：

1. 智能体规划一段新的代码。
2. 它调用 Deslop 的 MCP `find-similar` 工具，传入一个文件范围，或者一段内存中的代码片段外加所用语言。
3. Deslop 用 [tree-sitter](https://tree-sitter.github.io/tree-sitter/) 解析候选代码，归一化标识符和字面量，为其结构生成指纹，探查实时索引，并返回匹配的簇。
4. 智能体要么复用规范的实现，要么抽取出共享逻辑，要么因为不存在有意义的匹配而写下这段新代码。
5. LSP 监视器察觉到由此产生的文件变更，并更新编辑器界面以及 MCP 报告的生成。

重点不是要让智能体变得畏首畏尾。重点是要在这份记忆真正重要的那一刻，赋予它一位细心的维护者所拥有的、同样的仓库级记忆。

## 为什么这对 AI 生成的代码尤为重要

AI 生成的代码不必是有缺陷的，也能变得代价高昂。它只需要在两个地方重复同一条业务规则。

这种风险如今在公开的研究文献中已清晰可见。GitClear 的 [2025 年 AI Copilot 代码质量研究](https://www.gitclear.com/ai_assistant_code_quality_2025_research)报告称，重复代码块在增加，而被移动的代码行在减少——后者本是代码被整合（而非被复制）的信号。[Code Copycat Conundrum](https://arxiv.org/abs/2504.12608) 研究了 LLM 生成代码在字符、语句和代码块三个层面上的重复。Google 的 DORA 团队在 [2025 年 AI 辅助软件开发现状报告](https://dora.dev/dora-report-2025/)中，把 AI 辅助开发描述为一个放大器：当底层的工程体系本就健康时，团队才能获得最强的成效。

这就是可落地的经验教训。不要用一段文字叮嘱编码智能体“要小心”，然后指望它能记住。把仓库记忆放进工具闭环里。

## Deslop 向智能体返回什么

智能体无法利用一句“这里可能存在重复”之类的含糊警告。它需要带有稳定标识符、且上下文开销有界的结构化数据。

Deslop 的 MCP 界面按最严重优先的顺序返回重复的簇，并附带智能体可据以行动的字段：

- 一个稳定的簇 id，
- 克隆分桶，例如完全相同的代码或近乎相同的代码，
- 排名分数，
- 各次出现的路径和字节范围，
- 在可用时提供的结构、token 和嵌入（embedding）信号，
- 一段解读文字，
- 行动提示。

这份报告刻意以 JSON 为先。人类可读的文本视图和 HTML 视图，都是同一套 schema 的渲染结果，而不是另一份真相。文档中的[输出格式](/zh/docs/output-formats/)页面解释了消费方的契约，而[工作原理](/zh/docs/how-it-works/)则解释了解析、归一化、指纹、聚簇和排名这条流水线。

MCP 工具还带有一个出现次数预算。真实的仓库里，一个簇可能有几十处位置，把每一次出现都倾倒进智能体的上下文窗口并无益处。诸如 `top-offenders`、`report-for-file`、`report-for-range` 和 `find-similar` 这样的 Deslop 工具都接受一个 `max_occurrences` 预算，好让结果保持有用，而不至于沦为一场刷屏。当智能体确实需要完整的簇时，它可以再用 `cluster-by-id` 跟进。

这个小小的约束很重要。智能体工具应当为智能体的上下文预算而设计，而不仅仅因为协议允许就把它暴露出来。

## 为什么 MCP 要委托给 LSP

一个诱人的设计是让 MCP 服务器自己去扫描仓库。这看起来更简单——直到你开始在意正确性。

如果 LSP 和 MCP 各自运行自己的分析，你就有了两个监视器、两份缓存、两份报告，以及两次彼此不一致的机会。如果 MCP 从磁盘读取一份陈旧的文件报告，智能体就可能针对一个编辑器早已移除的簇采取行动。如果 MCP 在每次工具调用时都启动一次冷扫描，智能体就会因为它太慢而不再调用它。

Deslop 使用一个实时引擎。LSP 拥有分析会话并监视工作区。MCP 通过本地 IPC 向正在运行的 LSP 索取当前的报告。项目规范在 [MCP 外壳设计](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/mcp.md)和[实时分析设计](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/live.md)中对此有所描述：MCP 是一个传输适配器，而不是第二个分析引擎。

这正是“一个能给出答案的工具”与“一个智能体可以在生成过程中安全调用的工具”之间的区别。

## 对真实世界的工作区而言有哪些改变

当编码智能体不再只是演示时，实时路径新增了几项真正重要的特性。

**外部 MCP 客户端使用 VSIX 打包的二进制文件。** Claude Code、Claude Desktop、Cursor、Continue、Codex 以及其他外部 MCP 客户端，都应当指向已安装的 VS Code 扩展内部打包的 `deslop-mcp` 二进制文件——除非用户是通过 Homebrew 或 Scoop 安装的 CLI。这样可以避免本地的 `target/release` 二进制文件与扩展的线协议契约（wire contract）发生漂移。配置步骤见 [AI 集成](/zh/docs/ai-integration/)。

**Windows 不再需要 Unix 套接字。** 在类 Unix 系统上，LSP 暴露一个本地的 Unix 套接字。在 Windows 上，Deslop 则使用一个由令牌把关的 TCP 回环端点，其信息发布在 `.deslop-cache/deslop.port` 中，这样 MCP 和 LSP 依然讲同一套 JSON-RPC 协议，而无需假装 Windows 拥有 Unix 套接字。其实现描述见实时规范中的 [TCP 回环传输](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/live.md#live-ipc-tcp)。

**编辑器和智能体共享同一份报告。** VSIX 通过编辑器 UI 呈现各个簇，而 MCP 工具则把同样的实时数据暴露给智能体。[VS Code 簇面板文档](/zh/docs/vscode-cluster-panel/)介绍了面向人类的一侧。MCP 工具则覆盖面向智能体的一侧。重要的特性在于：两者读取的是同一个引擎。

## 该在你的智能体指令里写什么

如果你的仓库使用 Deslop，最好的指令是直接而机械的：

```text
Before writing any new function, class, helper, widget, mapper, repository,
or test setup, call Deslop's find-similar MCP tool. If a high-similarity
cluster exists, reuse the canonical implementation instead of creating
another copy.
```

这条指令比“避免重复”要有力得多。它点明了工具、时机，以及期望的动作。

对于已经维护着 `AGENTS.md`、`CLAUDE.md` 或同等的编码智能体指令文件的团队来说，这正是 Deslop 该待的地方。这个工具最有价值的时刻，是在智能体写代码之前，而不是在审查者追问同一条校验规则为何出现在四个文件里之后。

## 常见问题

### 面向编码智能体的 MCP 服务器是什么？

一个 MCP 服务器通过 Model Context Protocol 向 AI 应用暴露工具和资源。在编码工作流中，这可能意味着代码库查询、文档查询、issue 访问、构建动作或仓库分析。Deslop 的 MCP 服务器暴露的是重复代码分析工具，以 `find-similar` 为首。

### 为什么 Deslop 还需要一个 LSP 服务器？

因为重复代码预防有两类受众。人类需要通过 LSP 获得编辑器反馈：诊断、悬浮提示、code lens 以及导航。智能体则需要通过 MCP 获得可调用的工具。一个实时的 LSP 进程，同时也是监视工作区、让报告保持最新的合适归属者。

### 为什么不干脆在提交拉取请求之前跑一下 CLI？

CLI 对于 CI 和一次性审计依然有用。但对预防而言，它来得太晚了。一旦重复已经进入拉取请求，智能体就已经耗费了上下文，代码审查也已经开始。`find-similar` 属于生成之前。

### 这会取代代码审查吗？

不会。Deslop 报告的是证据：簇 id、位置、分数和信号。是否抽取、复用，还是接受这处重复，仍然由人类或智能体来决定。区别在于，这个决定发生在代码还新鲜的时候。

### 这只关乎逐字的复制粘贴吗？

不是。完全相同的重复是最容易的情形。Deslop 以解析器为先的流水线，同样被设计用来捕捉经过重命名的结构和近似重复的结构。克隆的分类法在[研究背景](/zh/docs/research-background/)以及经典综述论文 [Comparison and Evaluation of Code Clone Detection Techniques and Tools](https://www.cs.usask.ca/~croy/papers/2009/RCK_SCP_Clones.pdf) 中有所讨论。

MCP 给智能体一双手。LSP 给编辑器一双眼睛。Deslop 两者兼用，好让重复代码能在预防成本唯一低廉的那个点上被拦住：在第二份副本落地之前。
