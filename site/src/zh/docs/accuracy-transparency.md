---
layout: layouts/docs.njk
title: 准确性透明度 — 重复率的计算方式
description: 了解 Deslop 如何计算仓库重复率、哪些文件和代码范围会计入，以及实测值如何驱动 CI 阈值门禁。
eleventyNavigation:
  key: 准确性透明度
  order: 9
icon: fact_check
docsGroup: trust
lang: zh
---

# 准确性透明度

重复百分比衡量报告中的克隆覆盖了多少代码，不是检测器的准确率。本页适用于 [0.35.0](/zh/releases/)。

## 百分比的计算方式

```text
duplication_percent = clamp(100 × duplicated_loc / analysed_loc, 0, 100)
```

- `analysed_loc` 统计已分析源文件的物理行，包括空白行和注释行。排除的文件不计入；空文件贡献零行。
- `duplicated_loc` 统计每个文件中可见克隆覆盖的行号并集。重叠范围只计一次。
- **Same shape, different content** 仅供参考，不计入重复行、重复文件、克隆数、质量或阈值超限。其已分析的源代码行仍在分母中。
- 因 `report_hide` 或生成文件头而隐藏的出现位置不计入分子，但文件仍在分母中。用 `exclude` 可将文件完全排除在分析之外。
- 文件夹百分比是重复行数之和除以已分析行数之和，不是文件百分比的平均值。所有百分比由 Rust 引擎计算，客户端只负责显示。
- 总行数为零时报告 `0%`。JSON 保留完整精度，面向人的报告会舍入显示。

计算位于 [`report_metrics.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_metrics.rs)。误报可能抬高百分比，漏报可能压低百分比。算术正确并不证明检测正确。

## 如何检查准确性

[克隆登记表](https://github.com/Nimblesite/Deslop/tree/main/corpus/register)记录固定仓库提交中的代码对。**CLEARLY IN** 表示漏报该克隆就是假阴性；**CLEARLY OUT** 表示将该对报告为克隆就是假阳性；**NOT CLEAR** 只记录，不断言任一判定。

版本比较必须考虑分类变化。旧版的 `structural_only` 不是克隆：如果登记表标为 CLEARLY IN，只报告这种参考信息仍算漏报。

配对正确性和**克隆范围覆盖率**回答不同问题。前者检查是否找到了两处已知副本；后者检查报告范围覆盖了经判定源代码范围的多少部分。只找到长副本中的一个短片段，可能通过前者，却仍漏掉大部分重复代码。

[语料库规范](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/corpus.md)定义评分及自动生成的评分表。结果只适用于已判定样本和实测构建，不能保证任意仓库上的精确率和召回率都完美。

## CI 门禁的工作方式

设置 `--fail-over <percent>` 或 `[threshold] max_duplication_percent`。CLI 参数覆盖配置；`--no-fail-over` 禁用本次门禁。阈值必须是 `0` 到 `100` 之间的有限数值。

只有完整精度的实测百分比**大于**上限时才失败，等于上限时通过。超限会先写出报告，再以 `3` 退出。没有阈值时，仅存在重复不会使运行失败。

同时使用 `--diff` 和 `--only-changed` 时，门禁依据 diff 新增行上的重复率。仍会分析整个目录树，以便新副本匹配已有代码。详见[配置](/zh/docs/configuration/)。
