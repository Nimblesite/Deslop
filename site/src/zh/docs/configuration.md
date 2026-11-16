---
layout: layouts/docs.njk
docFilm: configuration
title: 配置 — .deslop.toml、可调参数、命令行参数、退出码
description: 一页列出 Deslop 的全部可调参数、.deslop.toml 配置段与命令行参数：相似度阈值、CI 门禁、排除规则与退出码。
eleventyNavigation:
  key: 配置
  order: 6
icon: tune
docsGroup: reference
lang: zh
---

# 配置

Deslop 从扫描根目录读取 `.deslop.toml`，或读取 `--config` 指定的文件；没有配置文件时使用下文的默认值。命令行参数优先于对应的配置键。未知的配置段或键会让运行失败并指出其名称，拼写错误永远不会悄悄关掉门禁。

报告的内容见[报告](/zh/docs/reports/)。

{% include "partials/doc-film.njk" %}

<span id="dials"></span>

## 可调参数

这里列出所有可以调整的数值。每一项都会改变 Deslop 找到什么、如何标注发现，或何时让 CI 失败。

<div class="key-table">

| 参数 | 默认值 | 作用 |
| --- | --- | --- |
| `--min-nodes` | `30` | 参与比较的最小语法子树（AST 节点数）。调高得到更少、更大的克隆；调低可捕获小克隆。 |
| `nearly_identical_min_shape` | `0.90` | 标注为 **Nearly identical** 所需的结构相似度。 |
| `nearly_identical_min_content` | `0.70` | 标注为 **Nearly identical** 所需的内容支持度。 |
| `similar_min_content` | `0.50` | 标注为 **Loosely similar** 所需的内容支持度。 |
| `shape_only_max_content` | `0.05` | 内容支持度不超过此值即视为没有：该代码对为 **Shape-only**，永远不计入重复。 |
| `max_duplication_percent` | 关闭 | 全仓库重复率超过该百分比时退出码为 `3`。`--fail-over` 可按次设置。 |
| `[threshold.kinds]` | 关闭 | 每种克隆类型各自的百分比上限，对照该类型自己的数值检查。 |
| `max_average_line_bytes` | `200` | 文件平均每行字节数超过此值即视为压缩后的构建产物并跳过。 |
| `--jobs` | 全部核心 | 每个并行阶段的工作线程数。只影响速度，任何取值下报告都完全相同。 |

</div>

<span id="similarity-dials"></span>

### 相似度参数

`[tuning.routing]` 下的四个参数决定一对代码范围被标为哪种**类型**。它们读取 Deslop 对每对代码的两项测量：

- **结构（shape）**：忽略名称和字面值后，两棵语法树有多相像。`1.0` 表示结构相同。
- **内容支持度（content support）**：填充结构的内容（名称、字面值、运算符、调用）有多少一致，或被一致地重命名（每个 `a` 都变成了 `b`）。`1.0` 表示内容相同。

Deslop 先判断一对代码是否算克隆（[准入机制](/zh/docs/how-it-works/)），然后由这些参数按以下顺序贴标签：

1. **Identical**：源码文本相同。不受任何参数影响。
2. **Nearly identical**：一致重命名，或结构 ≥ `nearly_identical_min_shape` **且**内容支持度 ≥ `nearly_identical_min_content`。
3. **Same behavior**：嵌入判断两者行为相同（仅在启用 `--embeddings` 时）。
4. **Loosely similar**：有一段完全相同的复制核心，或内容支持度 ≥ `similar_min_content`。

**不是**克隆、但结构 ≥ `nearly_identical_min_shape` 且内容支持度 ≤ `shape_only_max_content` 的代码对列为 **Shape-only**：结构相同、内容不同。它永远不计入重复。

<div class="key-table">

| 目的 | 调整 | 副作用 |
| --- | --- | --- |
| 减少被标为 Nearly identical 的代码对 | 调高 `nearly_identical_min_shape` 或 `nearly_identical_min_content` | 这些代码对改为 Loosely similar。默认只有 Identical 和 Nearly identical 计入总体百分比，因此它会下降。 |
| 让弱匹配的代码对不进入 Loosely similar | 调高 `similar_min_content` | 低于该值且没有复制核心的已准入代码对不再获得类型。 |
| 把更多结构相同的代码对视为 Shape-only | 调高 `shape_only_max_content` | 必须保持低于 `similar_min_content`。 |
| 发现更小的克隆 | 调低 `--min-nodes` | 发现更多、更小，扫描更慢。 |

</div>

每个参数取值 `0` 到 `1`，且必须满足 `shape_only_max_content < similar_min_content <= nearly_identical_min_content`，否则配置会被拒绝。任何参数都不能把被拒绝的代码对变成克隆。JSON 报告在 `routing` 下记录实际生效的值。默认值仍是暂定的，对其他取值打分的语料库扫描记录在 [corpus/tuning-sweep.md](https://github.com/Nimblesite/Deslop/blob/main/corpus/tuning-sweep.md)。

```toml
[tuning.routing]
nearly_identical_min_shape = 0.90
nearly_identical_min_content = 0.70
similar_min_content = 0.50
shape_only_max_content = 0.05
```

<span id="a-complete-desloptoml"></span>

## 完整的 `.deslop.toml`

模式为 gitignore 语义的 glob，相对扫描根目录解析。

```toml
[defaults]                       # 适用于所有语言
exclude = ["third_party/**"]     # 不分析
report_hide = ["**/*.snapshot.cs"]  # 分析，但在报告中隐藏

[defaults.boilerplate]
imports = "report"               # "suppress"（默认）| "report"

[language.csharp]                # 为单一语言扩展 [defaults]
report_hide = ["**/Migrations/**"]

[threshold]
max_duplication_percent = 20     # 超过 20% 退出码为 3

[threshold.kinds]
identical = 3                    # 仅完全相同的副本超过 3% 时退出码为 3

[total]
include = ["identical", "nearly_identical"]  # 计入总体数值的类型

[analysis]
allow_cross_language_comparison = false
include_dependencies = false
incremental = true
max_average_line_bytes = 200

[report]
split_by_language = false

```

<span id="exclude-vs-report_hide"></span>

## `exclude` 与 `report_hide`

<div class="key-table">

| 键 | 效果 | 适用场景 |
| --- | --- | --- |
| `exclude` | 文件在分析前被丢弃。 | 第三方代码，以及任何你永远不想比较的内容。 |
| `report_hide` | 文件会被分析，但其出现位置被隐藏。只有当集群的**所有**成员都被隐藏时，该集群才会离开报告。 | 生成的代码：手写代码重复生成代码时你仍会知道。 |

</div>

这两个键与 `boilerplate.imports` 一起写在 `[defaults]` 或 `[language.<name>]` 下。`suppress` 丢弃仅由 import 构成的克隆；`report` 把它们变成低严重度提示。语言覆盖项会**扩展**默认值。语言 id：`csharp`、`rust`、`python`、`dart`、`javascript`、`typescript`、`tsx`、`php`、`fsharp`、`go`、`c`。

<span id="built-in-rules"></span>

### 内置规则

这些规则先于你的模式生效。

- **始终排除：** `target`、`dist`、`build`、`out`、`coverage`、`.vscode-test`、`__pycache__`、`.dart_tool`、`.git`、`.claude`。压缩和打包后的文件也会按文件名后缀和 `max_average_line_bytes` 排除。
- **除非 `include_dependencies = true`，否则排除：** `node_modules`、`vendor`、`.cargo`、`.pub-cache`、`.venv`。
- **始终在报告中隐藏：**
  - 任何 `generated/` 目录
  - `alembic/versions`、`test/fixtures` 和 `tests/fixtures`，除非该目录就是扫描根目录
  - 生成文件后缀：`.g.cs`、`.generated.cs`、`.designer.cs`、`.pb.cs`、`.openapi.cs`、`.generated.py`、`_generated.py`、`.g.dart`、`.freezed.dart`、`.gr.dart`、`.config.dart`、`.gen.dart`、`.mocks.dart`、`.pb.dart`、`.pbenum.dart`、`.pbjson.dart`、`.pbserver.dart`、`.pbgrpc.dart`
  - 第一行注释是生成器标记的文件，例如 `// GENERATED CODE - DO NOT MODIFY BY HAND` 或 `// @generated`。标记从解析后的注释中读取，字符串里或后面注释中的同样短语不算。见[完整标记列表](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/exclusion.md)。

<span id="ci-gate"></span>

## CI 门禁

除非你启用门禁，Deslop 的退出码始终为 `0`。任何门禁被突破都会退出码 `3`，完整报告照常写出。

<div class="key-table">

| 键 | 默认值 | 作用 |
| --- | --- | --- |
| `[threshold] max_duplication_percent` | 关闭 | 全仓库重复率上限，`0`–`100`。 |
| `[threshold.kinds]` | 关闭 | 每种类型的可选上限：`identical`、`nearly_identical`、`loosely_similar`、`same_behavior`。各自独立于总体数值检查。 |
| `[total] include` | `["identical", "nearly_identical"]` | 哪些类型计入总体数值。未计入的类型保留其集群与分类型数值。`structural_only` 会被拒绝，因为 shape-only 永远不是重复。 |

</div>

`--fail-over <PERCENT>` 在单次运行中覆盖 `max_duplication_percent`；`--no-fail-over` 清除所有门禁。配合 `--diff` 和 `--only-changed` 时，门禁读取限定在 diff 内的百分比，旧债不会让合并前检查失败。操作指南：[用重复阈值把关 CI](/zh/docs/getting-started/#gate-ci-on-a-duplication-threshold) 与 [GitHub Action](/zh/docs/github-action/)。

<span id="other-settings"></span>

## 其他设置

<div class="key-table">

| 键 | 默认值 | 作用 |
| --- | --- | --- |
| `[analysis] allow_cross_language_comparison` | `false` | 允许克隆对跨越两种语言。 |
| `[analysis] include_dependencies` | `false` | 分析依赖目录。默认关闭，否则其重复会排在你能修改的代码之前。 |
| `[analysis] incremental` | `true` | 在所有入口复用磁盘解析缓存。`false` 强制每次完整重新解析。 |
| `[report] split_by_language` | `false` | HTML 报告按语言分节。`--split-by-language` 也可启用。 |

</div>

克隆只按重复质量排名：AST 节点数 × 额外副本数。没有任何设置会对某种类型重新加权。

<span id="cli-flags"></span>

## 命令行参数

<div class="key-table">

| 参数 | 默认值 | 作用 |
| --- | --- | --- |
| `PATH` | `.` | 要分析的目录。 |
| `--output <PREFIX>` | `.deslop/deslop-report` | 报告路径前缀，追加 `.json`、`.txt`、`.html`。日志写到 `<dir>/logs/`。 |
| `--config <FILE>` | `<root>/.deslop.toml` | 改用的配置文件。 |
| `--nojson` | 关闭 | 不写 JSON 报告。 |
| `--notext` | 关闭 | 不写文本报告。 |
| `--nohtml` | 关闭 | 不写 HTML 报告。至少保留一种格式。 |
| `--diff <FILE>` | — | 标记统一 diff 新增行上的克隆；`-` 从 stdin 读取。与代码树不匹配的 diff 会被拒绝。 |
| `--only-changed` | 关闭 | 丢弃 diff 之外的克隆。需要 `--diff`。 |
| `--technical` | 关闭 | 在 stderr 显示研究者视图：分类 ID、信号字母、节点数。 |
| `--no-incremental` | 关闭 | 本次运行不使用磁盘缓存。 |
| `--embeddings <MODE>` | `off` | `auto` 探测提供方，不可用时带警告回退；`required` 不可用时失败。 |
| `--embedding-provider <ID>` | `ollama` | 提供方。目前只有 `ollama`。 |
| `--embedding-model <MODEL>` | `nomic-embed-text` | 模型名，与 `ollama list` 显示一致。 |
| `--embedding-endpoint <URL>` | `http://127.0.0.1:11434` | 提供方端点。 |
| `--log-to-console` | 关闭 | 日志写到 stderr，而非 `logs/deslop-<timestamp>.log`。 |
| `--log-level <LEVEL>` | `info` | `error`、`warn`、`info`、`debug` 或 `trace`。`RUST_LOG` 优先。 |
| `--no-color` | 关闭 | stderr 不带颜色。设置 `NO_COLOR` 或 stderr 不是终端时同样如此。 |

</div>

上面的可调参数（`--min-nodes`、`--jobs`、`--fail-over`、`--no-fail-over`）也是命令行参数。开发者参数：`--from-report <FILE>` 重新渲染已保存的 JSON 报告，`--debug-ast <FILE>` 打印单个文件的规范化 AST，`--rerun-touch`、`--rerun-remove` 和 `--rerun-add` 通过实时更新路径重放文件变更。

<span id="compare-two-occurrences"></span>

### 比较两个出现位置

以 `<path>:<start_byte>:<end_byte>` 传入两个端点，数值从 JSON 报告复制：

```text
deslop . --compare "<left-path>:<start-byte>:<end-byte>" --compare "<right-path>:<start-byte>:<end-byte>"
```

Deslop 把该代码对的 JSON 判定打印到 stdout。偏移量以字节计，结束位置不包含在内。每个范围必须对应一个已指纹化的出现位置；不接受集群 ID。

<span id="exit-codes"></span>

## 退出码

| 代码 | 含义 |
| --- | --- |
| `0` | 完成，没有门禁被突破。 |
| `1` | 运行时错误：扫描路径不存在、`.deslop.toml` 无效、分析或 I/O 失败，或 `required` 的嵌入提供方不可达。 |
| `2` | 用法错误：未知参数或 `--fail-over` 取值越界。 |
| `3` | [CI 门禁](#ci-gate)被突破。报告照常写出。 |
