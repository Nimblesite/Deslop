---
layout: layouts/docs.njk
title: 面向 AI — 编码智能体如何用 Deslop 阻止重复代码
description: 面向编码智能体的操作说明：写代码前调用 find-similar；MCP 不可用时使用 Deslop CLI；正确解读阈值与字节范围。
eleventyNavigation:
  key: 面向 AI
  order: 4
icon: terminal
docsGroup: reference
lang: zh
---

# 面向 AI

使用与编辑器相同的引擎结果。本页介绍 `0.34.0` 之后的当前源码；安装包见[版本发布](/zh/releases/)，连接设置见[AI 智能体](/zh/docs/ai-integration/)。

## 动手写之前先检查

调用 `find-similar`，传入拟写片段，或 `path`、`start_byte` 和 `end_byte`。即使已有实现在仓库中只出现一次，也可以找到它。检查已有重复时，先用 `duplicates`，再用 `cluster-by-id` 查看结果。

读取返回的 `kind` 和源范围：

| `kind` | 含义与处理方式 |
| --- | --- |
| `identical` | 除允许的空白差异外，源文本相同。在适合时复用已有实现。 |
| `nearly_identical` | 重命名、参数化或略有编辑的副本。提取共享代码前检查差异。 |
| `loosely_similar` | 仍有大量复制内容，但改动较大。阅读两个范围。 |
| `same_behavior` | 可选嵌入分析的匹配。相似度不能证明可以互换。 |
| `structural_only` | 仅供参考的形状匹配，不是克隆，不计入重复度指标。 |

簇的 `mass` 表示影响大小，不是置信度或提取安全性。簇不携带配对相似度。需要某一对的证据时，明确指定两个端点并请求比较。

## MCP 不可用时使用 CLI

检查编辑器的 LSP 是否运行，以及 MCP 是否指向同一工作区。尚未就绪或连接错误不代表没有相似代码。构建不匹配时，应使用配套的捆绑二进制。

单独扫描：

```bash
deslop . --notext --nohtml --no-color
```

读取 `.deslop/deslop-report.json`。CLI 不能查询尚未写入的片段：修改前查看基线报告，修改后重新扫描，并检查 `clusters[].occurrences[]` 中的改动路径。复用或整合确认的副本后再扫描。未修改文件可以复用解析缓存，但分析仍考虑全仓库关系。

CLI 也能[比较两个已有位置](/zh/docs/configuration/#compare-two-occurrences)。两个入口都不可用时，应说明限制，不要猜测。

<span id="read-the-json"></span>

## 读取 JSON

使用报告内嵌的 `schema_doc`；MCP 会话中请求一次 `schema-doc` 即可。解析 JSON，而非文本或 HTML。

| 字段 | 含义 |
| --- | --- |
| `metrics.duplication_percent` | 克隆覆盖行数除以已分析行数，不是检测准确率。 |
| `metrics.threshold.breached` | 是否超过配置的重复率上限。CLI 先写报告，再以 `3` 退出。 |
| `clusters` | 克隆按质量降序排列，其后是参考信息。 |
| `cluster.kind` | 上表的分类；当前报告使用 `kind`，不是旧字段 `bucket`。 |
| `cluster.mass` / `cluster.rank` | 克隆影响和位置。参考信息质量为零，没有克隆排名。 |
| `cluster.severity` | 诊断级别，与质量和重复百分比独立。 |
| `occurrences[].hidden` | 由报告策略隐藏的位置，不贡献重复行数。 |

### 使用字节范围

范围是 `[start_byte, end_byte)`，不包含结束位置，且对应扫描时的源版本。编辑后先刷新分析，再使用偏移量。

### 稳定的簇 ID

用簇 ID 跨界面标识发现。排名随报告顺序变化；底层克隆内容改变时，ID 也可能改变。

## 配置仓库

[`exclude`](/zh/docs/configuration/) 在分析前移除文件；`report_hide` 保留分析，但隐藏出现位置。内置规则已覆盖依赖目录、构建产物和常见生成文件。阈值控制是否让 CI 失败，不改变克隆的判定标准。

## 操作规则

修改前阅读实际出现位置。不能把标签当作保持行为的证明，也不要放宽阈值来隐藏发现。重复若是有意保留，应记录原因。测量限制见[准确性透明度](/zh/docs/accuracy-transparency/)；可复用的项目规则见[智能体配方](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md)。
