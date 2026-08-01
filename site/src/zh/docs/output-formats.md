---
layout: layouts/docs.njk
title: 输出格式 — JSON、TXT、HTML
description: Deslop 每次运行都会生成三种报告 — 供智能体使用的规范 JSON、面向终端的按行组织的 TXT，以及面向人类的独立 HTML。同一套 schema，同一套排名。
keywords: deslop, 输出格式, json 报告, txt 报告, html 报告, 退出码, ci 门禁, 重复代码报告, schema
eleventyNavigation:
  key: 输出格式
  order: 5
icon: description
lang: zh
---

# 输出格式

Deslop 每次运行都会生成三种报告。JSON 才是产品本身；文本和 HTML 是基于同一份数据的渲染器。TXT 或 HTML 中不会出现任何 JSON 里没有的结论。

## JSON — 规范格式

`deslop-report.json` 是智能体读取的内容，也是 schema 消费方应当解析的内容。

```json
{
  "tool_version": "0.0.0-dev",
  "schema_doc": "…",
  "metrics": {
    "analysed_loc": 1832044,
    "duplicated_loc": 48120,
    "duplication_percent": 2.63,
    "clusters_total": 142,
    "duplicated_files": 318,
    "threshold": { "…": "--fail-over verdict" }
  },
  "clusters": [ { "…": "see AI Integration" } ]
}
```

### 保证

- schema 中标记为 `optional` 的字段可能缺失。标记为 `required` 的字段始终存在。
- 簇按 `weight` 降序排序。`clusters[0]` 始终是最严重的重复。
- UTF-8。无 BOM。LF 行尾。

## TXT — 终端格式

`deslop-report.txt` 是 ASCII、按行组织、刻意保持朴素的格式。没有 ANSI 颜色，没有 Unicode 制表符绘图，没有分页转义码。可以无意外地通过管道送入 `head`、`grep`、`awk`。

```
deslop 0.0.0-dev -- 840 file(s), 142 cluster(s), 0 hidden
repo: 2.6% duplicated (48120 / 1832044 LOC, 142 clusters across 318 files)
embeddings: off
-- action hints --
  [identical] Extract the shared code into one definition and call it from every duplicate site.
#1 [0362505641efe3c7] weight=1252.80 size=3 nodes=58
  3 near-identical copies — safe to extract.
  :: Nearly identical across UserRepository.cs, ProductRepository.cs, OrderRepository.cs.
```

每个簇都是一个带编号的代码块 — `#1` 是最严重的重复 — 包含其权重、大小和节点数，随后是一段通俗易懂的摘要和一行解读。簇按最严重者优先列出。这种格式可以在每一种终端、每一次 SSH 会话和每一份 CI 日志中保持完好。

## HTML — 可移植格式

`deslop-report.html` 是单个文件。所有 CSS 均已内联。无网络请求。把它放进 CI 工件、用邮件发送、在飞机上打开 — 它都能渲染。

HTML 渲染器使用与 JSON 和 TXT 相同的排名和相同的簇摘要。它额外提供：

- 语法高亮的示例代码片段，过长的片段和额外的出现位置会折叠进可展开的开关中
- 每个重复组上的「AI 匹配」徽章和影响力标签
- 在可折叠的「运行详情」页脚中，每组一张信号表（结构 / token / 嵌入 / 融合）

它不会额外提供：JSON 中没有的评分、超出 `summary` 字段的评论，或指向外部服务的链接。

<span id="exit-codes"></span>

## 退出码

| 退出码 | 含义 |
| --- | --- |
| `0` | 已完成。重复低于 `--fail-over` 阈值，或未设置阈值。 |
| `1` | 运行时错误 — 扫描路径不存在、分析失败、I/O 错误，或一个标记为 `required` 的嵌入提供方无法访问。绝不会是 panic。 |
| `2` | 用法错误 — 未知的标志或无效的参数值，在运行开始前即被拒绝。 |
| `3` | 重复百分比超过了 `--fail-over` 门禁。 |

`deslop` 绝不会因用户输入而 panic。失败通过这些退出码和 `stderr` 上的结构化错误呈现。

退出码 `3` 是 **CI 门禁**。它只在你主动启用时才会触发 — 通过传入 `--fail-over` 百分比，或在 `.deslop.toml` 中设置 `[threshold] max_duplication_percent`。详见[基于重复阈值设置 CI 门禁](/zh/docs/#gate-ci-on-a-duplication-threshold)的演练，或[面向 AI](/zh/docs/for-ai/#run-in-ci)中一份可直接粘贴的 GitHub Actions 作业。

## 日志与报告的区别

默认情况下，诊断信息通过带结构化字段的 `tracing` 写入一个带时间戳的日志文件（`.deslop/logs/deslop-<timestamp>.log`）；传入 `--log-to-console` 可改为将其发送到 `stderr`。人类可读的前言和摘要始终输出到 `stderr`。报告本身则写入文件 — 默认为被扫描项目下的 `.deslop/deslop-report.json`/`.txt`/`.html`，或你通过 `--output` 传入的前缀（日志会随之一起移动）。若只想生成 JSON 报告，请抑制另外两种格式：

```bash
deslop . --notext --nohtml      # writes .deslop/deslop-report.json
```
