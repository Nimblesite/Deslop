---
layout: layouts/docs.njk
title: 准确性透明度 — 重复率的计算方式
description: Deslop 如何计算重复率，以及 CI 门禁如何使用测得的数值。
keywords: deslop, 准确性, 重复率, 重复行, ci 门禁, 误报, 漏报
eleventyNavigation:
  key: 准确性透明度
  order: 9
icon: fact_check
docsGroup: trust
lang: zh
---

# 准确性透明度

Deslop 的百分比是对当前报告中发现结果进行精确算术计算所得的数值。它不是统计估计，也不声称检测具有完美的精确率或召回率。

它是一项原始覆盖度指标：衡量检测器当前标记了代码库中的多少内容，并将筛选后保留的每个发现原样计入。它不会根据发现背后证据的强弱进行降权。

## 百分比的计算方式

```text
duplication_percent = clamp(100 × duplicated_loc / analysed_loc, 0, 100)
```

- `analysed_loc` 是所有已分析源文件的物理行数。空白行和注释行均计入；空文件贡献零行。在分析前被排除的文件不贡献任何行数。
- `duplicated_loc` 是按文件计算的物理行号并集，这些行由通过报告筛选的簇中未隐藏的出现项所触及。由相互重叠的出现项或簇覆盖的同一行只计算一次。
- `report_hide` 和带有生成文件头标记的出现项不进入分子。生成文件仍保留在 `analysed_loc` 中，因此可能压低百分比（[#385](https://github.com/Nimblesite/Deslop/issues/385)）。除此之外，不会对任何发现降权：无论排名、大小、分桶还是置信度，全部按同等权重计入。某一行即使唯一证据只是代码形状匹配——也就是报告本身标注为 *"verify before extracting"* 的那一类——其计数方式也与已经逐字节证明完全相同的副本一致（[#344](https://github.com/Nimblesite/Deslop/issues/344)）。
- 每个文件的百分比使用相同的计算方式。文件夹百分比先将其中各文件的重复行数和已分析行数分别求和，再执行除法；它绝不是各文件百分比的平均值。求和与除法均由引擎自身完成，文件夹行也由引擎写入报告——所有界面（CLI、LSP、MCP、编辑器）上的每一个百分比都由引擎中的同一个函数计算；客户端只负责显示数值。
- 对于总行数为零的语料库，报告结果为 `0%`。JSON 保留完整的浮点数值；面向用户的报告会对其进行舍入显示。

实现完全公开：[`render_report` 选择可见簇集合](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report.rs)，[`compute_repo_metrics` 对覆盖行求并集](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_metrics.rs)，同一文件中的 `percent` 函数负责除法并将结果限制在规定范围内。

## CI 门禁的工作方式

设置 `--fail-over <percent>`，或在 `.deslop.toml` 中设置 `[threshold] max_duplication_percent`。CLI 参数会覆盖配置值；`--no-fail-over` 会为本次运行禁用门禁。

只有当完整精度的实测值**大于**上限时，门禁才会失败。等于上限时通过。超过上限后，程序会先写出报告，再以 `3` 退出；未设置阈值时，仅仅存在重复绝不会使运行失败。阈值必须是从 `0` 到 `100` 的有限数值。

确切的比较逻辑位于 [`ThresholdSummary::resolve`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_metrics.rs)，CLI 优先级与退出处理位于 [`main.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop/src/main.rs)，而 [GitHub Action 会在重新传递退出码之前保留报告](https://github.com/Nimblesite/Deslop/blob/main/action.yml)。

请将该百分比视为对当前报告中可见发现的精确测量，而不是代码库中每一处重复的绝对真值。
