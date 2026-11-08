---
layout: layouts/blog.njk
title: "Deslop 中的 Osprey：效应、处理器与更少的副本"
date: 2026-10-25
author: Christian Findlay
tags:
  - posts
  - osprey
  - algebraic-effects
  - duplicate-code
category: engineering
description: "Osprey 在 Deslop 中获得原生 tree-sitter 分析、实时重复发现结果、智能体检查，以及 UI 中官方的青色 Osprey 图标。"
excerpt: "效应。可复用的处理器。推断的类型。原生编译。Osprey 赢得了我们的关注——如今它又拥有了原生重复检测和官方视觉标识。"
heroImage: "/assets/img/blog/osprey-first-class.webp"
heroImageWidth: "1600"
heroImageHeight: "840"
heroImageAlt: "一个发光的青色翼状分支结构悬浮于午夜蓝空间之中。"
ogImage: "/assets/img/blog/osprey-first-class-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
lang: zh
---

一门围绕代数效应、类型推断和原生编译构建的函数式语言，值得拥有理解其代码的工具。Deslop 现在能读取全部三种 Osprey 风格：默认的 `.osp`、原生 ML 的 `.ospml`，以及显式选择的 Python 语法。它会发现被复制的结构，并把发现结果带进编辑器和智能体工作流。

语言分组和报告视图使用 Osprey 官方的青色小鸟。

## 值得大书特书的这门语言

Osprey 让应用逻辑通过**效应（effect）**说出自己的需要，再由**处理器（handler）**提供实现。这使得一项能力与其提供者之间的边界变得显式。生产代码和确定性测试可以共享同一份应用逻辑。

下面是一个小例子。函数请求一个时钟；处理器选择答案。

```osprey
effect Clock { now: fn() -> int }

fn announce() = {
    let timestamp = perform Clock.now()
    "Osprey launch at ${timestamp}"
}

let preview = handler Clock {
    now => 42
}

print(preview(announce))
```

`perform` 请求这项能力。`preview` 是一个可复用的处理器值。函数专注于播报本身，检查器会推断其结果类型。对于依赖保持可见、测试保持确定性的代码来说，这是一种格外有吸引力的编写方式。

Osprey 通过 LLVM 编译为原生二进制文件，并为受支持的工作负载提供 WebAssembly 和 C ABI 目标。这门语言正在积极演进；其[文档](https://www.ospreylang.dev/docs/)描述了当前边界。

## 真实的语法配得上真实的分析

Deslop 将 Osprey 的效应、处理器分支、模式、类型和表达式保留为语法树结构。重命名函数或绑定不会让被复制的代码消失：标识符拼写会在结构比较之前归一化。字面量拼写同样会归一化，而其源内容仍可供共享的证据检查使用。

字符串插值得到了应有的重视。`${...}` 内部的代码仍然是代码：条件表达式保持为条件表达式，加法与减法依然区别分明。在这样一门语言里，一个简洁的表达式可能承载大量应用逻辑，这一点很重要。

注释和 import 不算作被复制的实现。发现结果使用与其他语言相同的实测排名，因此最大的重复问题始终排在最前面。证据决定评分。

## 当一个有用的处理器变成六份副本

设想一个项目为发票测试准备了一个确定性时钟处理器。下一个功能需要为收据做同样的准备，于是这个处理器被复制了一份。随后提醒功能也需要它。每份副本都长出略有不同的外壳；名字变了、某个字面量变了，找到共享实现变得比搜索原函数名还要困难。

这正是结构化分析证明自身价值的时刻。Deslop 比较解析后的实现，把幸存的副本聚成一个簇，并为被复制的工作排序。你可以检查真实的出现位置，判断哪些差异是有意义的，并在测试支持时抽取共享辅助函数。仅有相似的脚手架并不构成充分证据；被复制的工作才重要。

效应让能力显式化。重复报告让重复的实现显式化。两者合起来给了你一个有用的问题：这些测试应该共享一个处理器，还是它们表达的是值得拥有各自代码的不同策略？比较结果帮你决策；评分无法替你做出那个设计决策。

## 在编辑器中占据前排座位

在安装了 Deslop 的情况下打开一个 `.osp` 文件。Osprey 加入实时分析闭环，发现结果可通过编辑器、聚簇比较和编码智能体使用的 MCP 工具获取。它的语言标识出现在分组、文件行和报告 webview 中，与其他每种受支持语言的图标并列。

对 Osprey 项目做 CLI 扫描：

```bash
deslop .
```

从排名最高的真实重复开始，比较它的各处出现，判断一个经过测试的实现能否取代这些副本。Osprey 给你可复用的能力；Deslop 帮你发现理应获得同样待遇的应用逻辑。

对智能体而言，有用的时刻来得更早。在生成另一个测试辅助函数或服务包装器之前，先用 `find-similar` 询问拟议的实现。如果合适的辅助函数已经存在，就复用它。改动之后，检查 `duplicates`，再打开相关聚簇比较源码范围。MCP 工具查询的是实时分析会话，因此这正好嵌入编辑闭环之中。

## 三种风格，清晰的边界

Osprey 有默认的花括号语法、ML 缩进语法和一个看起来像 Python 的表层。理解其中一种，并不能自动确立对其他几种的正确分析。

三种风格都配有经过验证的 CLI 测试夹具和真实的 LSP/MCP 语言标签检查。文件开头的 `# osprey: flavor=python` 注释、显式的 `python_sources` 条目或 Python 项目条目，都会把一个 `.py` 文件划归 Osprey。普通 Python 文件保持其 Python 身份，而 Osprey 的 Python 表层不会继承普通 Python 的自动抽取动作。

原生 ML 的 `.ospml` 所使用的语法，经过 Osprey 真实源码语料和独立结构检查的验证。缩进布局、插值表达式以及扁平调用与柯里化调用的差异都保持为可见语法。三种解析器身份在缓存和报告中保持独立，即便界面为它们呈现同一个官方 Osprey 图标。我们并不声称跨风格的克隆等价性。

[Osprey 支持指南](/zh/docs/osprey/)包含配置细节和当前集成状态。这些变更描述的是开发树；在期待它们出现在旧版 VSIX 中之前，请先检查你安装的 Deslop 版本。

## 小鸟万岁

让依赖显式化的效应。可以保留并复用的处理器。检查器自行推断的类型。原生编译。Osprey 带来了一个引人注目的组合，我们希望它的工具链配得上这份雄心。

探索 [Osprey](https://www.ospreylang.dev/)，然后[安装 Deslop](/zh/docs/)并让它开始工作。
