---
layout: layouts/blog.njk
title: 为什么 Deslop 使用 tree-sitter 解析源代码
date: 2026-04-10
author: Christian Findlay
tags: posts
description: Deslop 使用 tree-sitter 解析源代码并比较归一化后的语法树，因此格式与标识符变化不会掩盖结构性重复。
excerpt: Deslop 比较解析后的语法树，而不是匹配源代码文本。本文说明这样做能解决什么问题，以及语言专用的归一化位于何处。
heroImage: "/assets/img/blog/tree-sitter-over-regex-header.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "对比正则表达式逐行匹配与 tree-sitter AST 指纹的页头图片。"
ogImage: "/assets/img/blog/tree-sitter-over-regex-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
lang: zh
---

Deslop 使用 tree-sitter 解析源文件，并比较归一化后的语法树。解析器为检测器提供稳定的结构化输入，无需依赖针对源代码文本的正则表达式。

## 原始文本匹配会漏掉什么

原始文本匹配容易受不改变程序结构的改动影响：

- **格式。** 两个完全相同但格式不同的函数看起来像是不同的代码。
- **重命名。** 在一个方法中把 `user` 改成 `customer` 会破坏每一次匹配。

## tree-sitter 让我们能做什么

tree-sitter 解析器为仓库中的每个文件生成一棵 AST。从这棵树出发，我们可以：

- 将标识符和字面量归一化为规范的占位符，使重命名坍缩到同一个指纹；
- 独立地哈希子树，使一个方法的指纹无论它位于文件中的何处都保持稳定；
- 在子树而非行上进行操作，使格式和空白字符变得无关紧要；
- 返回当前源代码快照中每个出现位置的精确字节范围。

不同语言的语法会以不同方式表示标识符、字面量与样板结构，因此归一化规则按语言实现。归一化后的语法树再进入共享的指纹、聚簇与排名流水线。

## 这对你意味着什么

- **重命名重构不会隐藏重复。** 一个簇能在标识符重命名后存活，因为指纹是在归一化后的 AST 上计算的。
- **格式变更不会改变结构指纹。** 使用 `rustfmt` 重新格式化文件，不会改变 Deslop 所比较的语法结构。
