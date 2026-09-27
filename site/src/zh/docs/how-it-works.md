---
layout: layouts/docs.njk
title: 工作原理 — tree-sitter AST、MinHash LSH、HNSW 嵌入
description: Deslop 如何使用 tree-sitter AST、Merkle 指纹、MinHash LSH、可选 HNSW 嵌入与最严重者优先排名来检测重复代码。
eleventyNavigation:
  key: 工作原理
  order: 2
icon: account_tree
docsGroup: trust
lang: zh
---

# 工作原理

Deslop 使用 tree-sitter 解析源代码，寻找候选副本，检查内容，再将符合条件的结果分组。CLI、编辑器和 MCP 工具共用同一个 Rust 引擎。

本页介绍当前源码，包括 `0.34.0` 之后的改动。已发布安装包见[版本发布](/zh/releases/)。

## 寻找候选副本

支持 C#、Rust、Python、Dart、JavaScript、TypeScript/TSX、PHP、F# 和 Go。默认遵循 `.gitignore`，排除依赖目录和构建产物。详见[配置](/zh/docs/configuration/)。

引擎归一化语法树，使名称、字面量和格式差异不会遮蔽候选副本。它对语法子树和连续语句生成指纹，也会查找同一文件中的复制。结构重叠和 MinHash 相似度帮助找回外层方法已经变化的副本。

仅有相同结构不足以证明重复。引擎还检查源内容、一致的重命名以及实际执行的操作。给协作对象换个名字，与改为调用另一个操作，是两种不同的变化。

## 理解分类

| 报告标签 | 含义 |
| --- | --- |
| **Identical code** | 除允许的空白差异外，源文本相同。对应 Type I；研究定义还允许注释差异，因此 Deslop 的文本一致性规则更严格。 |
| **Nearly identical code** | 重命名、参数化或略有改动的副本：Type II 和接近原文的 Type III。 |
| **Similar code** | 仍有大量复制内容，但语句或控制流变化较大：Type III。 |
| **Same behavior, different code** | 可选嵌入分析给出的 Type IV 候选。需要审查两处实现，不能据此认定可以互换。 |
| **Same shape, different content** | 布局相似，但几乎没有共享内容。仅供参考，不是克隆。 |

仅形状相同的结果不计入克隆数、重复质量或重复百分比，排在克隆之后，默认不产生诊断。分组标签描述已建立的成员关系；一串间接匹配并不能证明任意两处都匹配。

## 比较两个位置

相似度证据属于两个明确的源代码范围。簇只携带类型、出现位置、质量和排名，不会把某一对的相似度当成整个簇的置信度。

在 VS Code 中使用 **Compare To Canonical**，或先 **Select for Compare**，再 **Compare with Selected**。CLI 也支持 [`--compare`](/zh/docs/configuration/#compare-two-occurrences)。引擎会重新计算所选两个位置的判定。

## 按重复质量排名

```text
mass = canonical_node_count × max(visible_members − 1, 0)
```

质量是被复制语法树的节点数乘以额外可见副本数。克隆按质量降序排列，相同时用稳定 ID 排序。仅形状相同的结果重复质量为零，没有克隆排名。

质量既不是置信度，也不是百分比。[重复百分比](/zh/docs/accuracy-transparency/)统计被克隆覆盖的源代码行，重叠范围只计一次。

## 可选嵌入分析

嵌入默认关闭。`--embeddings auto` 在提供方可用时启用；`--embeddings required` 在无法连接时失败。Ollama 默认模型是 `nomic-embed-text`，模型和端点均可配置。嵌入相似度提供候选，不能证明行为等价或提取安全。

## 保持报告更新

未修改文件可以复用缓存的解析和签名结果。实时会话监听编辑、更新分析，并向编辑器广播新报告。MCP 查询读取该会话；CLI 为 CI 或单次检查另行扫描。

JSON、文本和 HTML 展示同一份报告。详见[报告输出](/zh/docs/configuration/#report-output)、[准确性透明度](/zh/docs/accuracy-transparency/)和[研究背景](/zh/docs/research-background/)。
