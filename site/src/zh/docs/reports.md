---
layout: layouts/docs.njk
title: 报告 — JSON、文本与 HTML 输出
description: Deslop 扫描后写出什么、文件放在哪里，以及 JSON、文本和 HTML 三种报告之间的关系。
eleventyNavigation:
  key: 报告
  order: 6.5
icon: code
docsGroup: reference
lang: zh
---

# 报告

扫描会把三份报告写到扫描根目录下的 `.deslop/`，或写到 `--output` 指定的前缀。三者渲染同一份数据：文本或 HTML 中不会出现 JSON 里没有的内容。用 `--nojson`、`--notext` 或 `--nohtml` 关闭某种格式；改变报告内容的设置见[配置](/zh/docs/configuration/)。

```bash
deslop .                    # .deslop/deslop-report.{json,txt,html}
deslop . --notext --nohtml  # 仅 JSON
```

<span id="json"></span>

## JSON

`deslop-report.json` 是规范报告，供智能体和工具解析。

- 克隆按 `mass` 降序排列，平局由稳定 ID 决定。shape-only 发现排在其后，不参与排名，也不计入重复。
- schema 中标为 `required` 的字段始终存在；`optional` 字段可能缺失。
- UTF-8、LF 换行、无 BOM。

逐字段结构见 [For AI → 读取 JSON](/zh/docs/for-ai/#read-the-json)。

<span id="text"></span>

## 文本

`deslop-report.txt` 用于终端和 CI 日志。每个克隆按排名列出类型、质量、出现次数和源码位置。该报告不含代码对相似度；需要时用 [`--compare`](/zh/docs/configuration/#compare-two-occurrences) 查询。

<span id="html"></span>

## HTML

`deslop-report.html` 是单个自包含文件：CSS 内联、无网络请求，在任何地方都能打开，包括作为 CI 产物。在共享数据之外它还提供：

- 语法高亮片段，较长片段和额外位置默认折叠
- 每种克隆类型一个可折叠分组，标出该组的质量
- “Run details”页脚，包含工具版本、分组数量与嵌入来源

`--split-by-language`（或 `[report] split_by_language`）让每种语言单独成节。

<span id="logs"></span>

## 日志

日志不是报告。默认写到 `.deslop/logs/` 下带时间戳的文件，使用 `--log-to-console` 时写到 stderr。简短的人类可读摘要始终写到 stderr。
