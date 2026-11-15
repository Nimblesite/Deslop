---
layout: layouts/docs.njk
docFilm: accuracy-transparency
title: 准确性透明度 — Deslop 的准确率如何衡量
description: Deslop 如何用人工判定的克隆对衡量重复代码检测准确率、已修复的准确性问题、与 jscpd 的对比，以及重复率的计算方式。
eleventyNavigation:
  key: 准确性透明度
  order: 9
icon: fact_check
docsGroup: trust
lang: zh
---

# 准确性透明度

这里回答两个不同的问题：Deslop 报告的重复是否正确？它的重复率衡量的是什么？每个数字都链接到背后的报告。

{% include "partials/doc-film.njk" %}

## 准确率如何衡量

Deslop 以[克隆登记册](https://github.com/Nimblesite/Deslop/tree/main/corpus/register)为基准评分：登记册记录了固定提交的开源仓库中的代码范围对，由评审者阅读源码后判定。

- **CLEARLY IN** 是真实的重复。没有报告它即为**漏报**。
- **CLEARLY OUT** 不是重复。报告它即为**误报**。
- **NOT CLEAR** 会被记录，但不计入任何一方。

2026 年 11 月 15 日的最新评分覆盖 11 种语言的 47 个仓库：

| 指标 | 结果 |
|---|---:|
| 评分正确的判定对 | 1,619 中的 1,579（97.5%） |
| 漏报的真实重复 | 29 |
| 误报的非重复 | 11 |
| 报告的副本覆盖的判定行 | 99.2% |

剩余的每个漏报和误报都按文件和行号列在[评分报告](https://github.com/Nimblesite/Deslop/blob/main/docs/reports/corpus-scorecard-2026-11-15.md)中，并仍按缺陷处理。评分规则见[语料库契约](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/corpus.md)。这些结果只覆盖已判定的代码对，并不保证在任何仓库上都有相同的精确率和召回率。

判定对是否正确与覆盖率是两个问题。两个判定范围出现在同一个报告结果中，该对即为正确；覆盖率衡量结果包含了这些范围的多少，因此只报告长副本中的一小段虽算找到，但会降低覆盖率。

## 已修复的准确性问题

每个确认的误报或漏报都会在 GitHub 上以 `false-positive`、`false-negative` 或 `lane/accuracy` 标签登记，并配有一个在修复前一直失败的测试。问题只有在正式发布的版本上验证修复后才会关闭。

2026 年 8 月 21 日，公开的问题关系图中有 33 个未解决的准确性问题，是最大的工作流：

<figure>
  <a href="/issues/" aria-label="打开当前的 Deslop 交互式问题关系图">
    <img src="/assets/img/blog/issue-graph-1600.webp"
         srcset="/assets/img/blog/issue-graph-800.webp 800w, /assets/img/blog/issue-graph-1600.webp 1600w"
         sizes="(max-width: 48rem) calc(100vw - 3rem), 48rem"
         alt="2026 年 8 月 21 日的 Deslop 问题关系图，红色的准确性工作流有 33 个未解决问题。"
         width="1600" height="943" loading="lazy" decoding="async">
  </a>
  <figcaption>2026 年 8 月 21 日的问题关系图。点击打开今天的关系图。</figcaption>
</figure>

截至 2026 年 11 月 15 日，**已修复并关闭 48 个准确性问题**，仍有 29 个未解决：

| 关闭月份 | 修复的准确性问题 |
|---|---:|
| 2026 年 8 月 | 1 |
| 2026 年 9 月 | 31 |
| 2026 年 10 月 | 4 |
| 2026 年 11 月（截至 15 日） | 12 |

修复内容包括：无关的样板代码被排为首要重复、重命名后的副本从未被报告、长副本被切成碎片，以及无关的簇共用一个 id。未解决的问题见[问题关系图](/issues/)和[问题规划](/issues/planner/)。

## 与 jscpd 的对比

Deslop 与 [jscpd](https://github.com/kucherenko/jscpd) 5.4.3 在相同的 11 个语料库仓库上运行，并用同一规则评分：只要某个报告结果同时覆盖判定对的两个范围，该对即算找到。

| 找到的已判定真实重复 | Deslop | jscpd |
|---|---:|---:|
| 全部（789 对） | 787（99.7%） | 408（51.7%） |
| 完全相同的副本（106 对） | 106 | 85 |
| 重命名的副本（251 对） | 251 | 84 |

请结合以下限制解读：

- **判定对来自 Deslop。** 登记册是通过判定 Deslop 报告的候选项建立的，因此偏向 Deslop 能找到的内容。
- **jscpd 报告了 Deslop 没有报告的代码对。** jscpd 的 8,666 对中，有 3,507 对没有对应的 Deslop 结果。这些对尚未判定，既不算 Deslop 漏报，也不算 jscpd 误报。
- **没有比较精确率。** 这 11 个仓库只有 8 个判定为非重复的代码对，两个工具报告的都不超过一个。
- **jscpd 快得多。** 它扫描全部 11 个仓库用时 1.0 秒，内存峰值 194 MiB；Deslop 用时 194.6 秒，峰值 1,845 MiB。
- **Deslop 二进制是 2026 年 11 月 14 日的本地构建**，未记录其提交。

完整[报告](https://github.com/Nimblesite/Deslop/blob/main/docs/reports/deslop-vs-jscpd.md)和[生成它的脚本](https://github.com/Nimblesite/Deslop/tree/main/scripts/compare-jscpd)都在仓库中。公平的精确率比较需要对 jscpd 自己的代码对抽样判定。

## 重复率如何计算

重复率衡量报告的重复覆盖了多少代码，并不说明这些重复是否正确。

```text
duplication_percent = clamp(100 × duplicated_loc / analysed_loc, 0, 100)
```

- `analysed_loc` 统计已分析源文件中的所有物理行，包括空行和注释行。
- `duplicated_loc` 统计每个文件中被报告副本覆盖的行，重叠范围只计一次。
- **形状相同、内容不同**的结果仅供参考，不计入重复行、文件、质量或 CI 门禁。
- 被 `report_hide` 或生成文件标记隐藏的副本不计入重复行，但其文件仍计入 `analysed_loc`。使用 `exclude` 可将文件完全排除。
- 文件夹重复率是重复行之和除以分析行之和，而不是各文件百分比的平均值。

所有百分比都由 Rust 在 [`report_metrics.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_metrics.rs) 中计算，编辑器只负责显示。误报会抬高百分比，漏报会拉低百分比。

## CI 门禁如何工作

设置 `--fail-over <percent>` 或 `[threshold] max_duplication_percent`（命令行参数优先）。只有实测百分比**大于**上限时运行才会失败：先写出报告，再以退出码 `3` 结束。`--no-fail-over` 在单次运行中关闭门禁。配合 `--diff` 和 `--only-changed` 时，门禁只衡量 diff 新增的行，但仍与整个代码树匹配。参见[配置](/zh/docs/configuration/)。
