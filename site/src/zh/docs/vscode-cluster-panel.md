---
layout: layouts/docs.njk
title: VS Code — 在编辑器中解读重复代码簇
description: 使用 Deslop VS Code 扩展查看 Top Offenders、实时重复警告与代码簇信号，并比较规范出现位置。
eleventyNavigation:
  key: VS Code
  order: 5
icon: account_tree
docsGroup: guides
lang: zh
---

# VS Code 簇面板

面板展示一组出现位置，方便检查实际副本。本页介绍当前源码中的控件，包括 `0.34.0` 之后的改动；安装包见[版本发布](/zh/releases/)。

## 打开与比较

- 点击**文件链接**即可打开并选中代码范围，不再提供独立的 Open 按钮。
- **Compare To Canonical** 将参考位置放在左侧、点击的位置放在右侧。参考行有明确标记，不能与自身比较。
- 比较其他任意两处时，先在左侧目标上选 **Select for Compare**，再在另一处选 **Compare with Selected**。**Clear Selection** 取消选择。完成比较或切换簇后，选择会清除。
- **Copy Context For AI** 提供可交给智能体的发现上下文。

Canonical 指导航用的参考位置，不代表最佳实现。差异窗口标题标明两个范围及引擎对此配对的判定，包括逐字节相同或仅缩进不同。两处范围可以位于同一文件。

## 理解发现结果

**克隆类型**区分 Identical code、Nearly identical code、Similar code、可选的 Same behavior, different code，以及仅供参考的 Same shape, different content。[工作原理](/zh/docs/how-it-works/)解释它们与 Type I–IV 的关系。

**Mass（质量）**是被复制语法树大小乘以额外可见副本数。**Rank（排名）**按质量从大到小排列克隆，两者都不是置信度。**Occurrence count（出现次数）**是位置总数，大型组可能只显示部分行。**Cluster ID**用于跨界面标识同一发现。

**Severity（严重程度）**是该类型配置的诊断级别，不是质量排名。Identical 和 Nearly identical 默认 warning；Similar 和 Same behavior 默认 information。仅形状相同的结果默认没有诊断，且永远不计入重复率。颜色标识类型。

结构、词元、内容和嵌入测量描述的是两个位置之间的关系，只出现在明确的配对比较中，不是整个簇的评分。

## 使用侧边栏

**Top Offenders** 可按 **Clone Category**、**Folder**、**Language**、**File** 或 **No Grouping** 分组。克隆保持质量降序。一个分组菜单取代独立的语言和排序控件。

**Duplication** 展示工作区、文件夹和文件百分比，与报告使用相同的引擎计算。**Session** 展示分析状态和嵌入设置。视图随实时分析更新。

悬停或键盘聚焦帮助按钮可打开带格式的说明。按 **Escape** 关闭；技术细节默认折叠。

## 键盘快捷键

| 快捷键 | 操作 |
| --- | --- |
| `j` / `k` | 聚焦下一个 / 上一个出现位置。 |
| `n` / `p` | 切换到下一个 / 上一个簇。 |
| `Enter` | 打开聚焦的出现位置。 |
| `?` | 切换键盘帮助。 |
