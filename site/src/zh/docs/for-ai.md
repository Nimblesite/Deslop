---
layout: layouts/docs.njk
title: 面向 AI — 配置、设置 CI 门禁并读取 Deslop 报告（智能体指南）
description: 面向使用 Deslop 的 AI 编码智能体的操作手册——在编写代码前调用 find-similar、配置 .deslop.toml、以重复阈值（退出码 3）设置 CI 门禁，并解析规范的 JSON 报告。
eleventyNavigation:
  key: 面向 AI
  order: 4
icon: terminal
lang: zh
---

# 面向 AI

本页是面向 **AI 编码智能体**（Claude Code、Cursor、GitHub Copilot、Continue、Codex）以及配置它们的人员的操作手册。它刻意采用指令式风格：如何配置 Deslop、应排除什么、如何设置 CI 门禁，以及如何解析报告。要将 `deslop-mcp` 接入你的客户端，请先阅读 [AI 集成](/zh/docs/ai-integration/)。

## 唯一法则：在编写代码前调用 `find-similar`

Deslop 的价值通过**预防**而非清理来体现。在你编写任何新的函数、方法、类、辅助函数、fixture 或测试初始化代码之前，请使用拟写的代码片段（或其字节范围）调用 `find-similar` MCP 工具，并阅读其响应：

- `signals.fused ≥ 0.85`，或归入 `identical` / `nearly_identical` 桶 → **不要编写这份副本。** 复用该工具返回的规范出现位置；如有必要，提取一个共享的辅助函数。
- `signals.fused < 0.6`，或响应为空 → 继续编写。
- `0.6 ≤ fused < 0.85` → 阅读该规范出现位置，并倾向于复用。

`find-similar` 用于**编写代码**。当你在*清理*现有重复时，先从 `top-offenders` 开始，再对你将要合并的簇调用 `cluster-by-id`。可直接粘贴到你项目 `AGENTS.md` / `CLAUDE.md` 的规则块参见[智能体配方](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md)。

| 工具 | 何时调用 |
| --- | --- |
| `find-similar` | **在**编写新代码**之前**——是否已存在等价实现？ |
| `top-offenders` | 工作区中最严重的簇，最严重者优先。从这里开始清理。 |
| `cluster-by-id` | 你即将合并的某个簇的完整成员列表与信号。 |
| `report-for-file` / `report-for-range` | 触及某个特定文件或选区的簇。 |
| `schema-doc` | 权威的 JSON schema。每个会话调用**一次**，而非每次响应都调用。 |

## 使用 `.deslop.toml` 配置

Deslop 从扫描根目录（或传给 `--config` 的路径）读取 `.deslop.toml`。该文件并非必需——Deslop 内置了保守的默认值。每个节和键都是可选的；不需要的可以省略。

```toml
# Shared rules, applied to every language.
[defaults]
exclude     = ["vendor/**", "third_party/**"]    # dropped before parsing — never analysed
report_hide = ["**/*.generated.cs", "**/*.g.cs"] # analysed, but hidden from the ranked report

# Per-language overlays, keyed by the parser language id:
# csharp, rust, python, dart. Overlays EXTEND [defaults]; they never replace it.
[language.csharp]
report_hide = ["**/Migrations/**/*.cs"]

[language.rust]
exclude = ["**/target/**"]

# Opt-in CI gate. Exceeding this exits 3. See "Run in CI" below.
[threshold]
max_duplication_percent = 5.0

# Analysis behaviour.
[analysis]
allow_cross_language_comparison = false  # true → compare clones across languages (ports, generated clients)

# Report rendering.
[report]
split_by_language = false  # true → one HTML section per language
```

**键必须位于某个节之下。** 没有 `[defaults]` 标头的裸顶层 `exclude = [...]` 会被静默忽略。按语言划分的节是叠加式的：一个 `.rs` 文件会与 `defaults.exclude ∪ language.rust.exclude` 进行匹配。

## 应排除什么

两个层级，语义不同。按意图选择：

| 键 | 效果 | 适用于 |
| --- | --- | --- |
| `exclude` | 文件在发现阶段就被丢弃——从不解析、从不计入 `analysed_loc`、从不出现在任何簇中。 | 你不拥有且完全不希望被分析的厂商 / 第三方代码。 |
| `report_hide` | 文件**仍会**被分析并可作为簇的锚点，但每个出现位置都会被标记为 `hidden: true`。成员*全部*隐藏的簇会从排名中剔除；含有一个可见成员的簇会保留，因此你仍能看到"手写代码与生成代码重复"。 | 你仍希望检测其手写副本的生成产物。 |

**内置默认值已涵盖常见情形——不要重复添加。** 默认排除：`node_modules`、`target`、`dist`、`build`、`.venv`、`__pycache__`、`.cargo`。默认在报告中隐藏：任何含有 `generated` 路径组成部分的路径、`alembic/versions` 下的 Alembic 迁移，以及后缀 `*.g.cs`、`*.generated.cs`、`*.designer.cs`、`*.pb.cs`、`*.openapi.cs`、`*.generated.py`、`_generated.py`、`*_pb2.py`、`*_pb2_grpc.py`。只需在此基础上添加项目特定的模式。

Glob 采用 gitignore 风格，并针对相对于配置文件的路径进行匹配。

## 在 CI 中运行

无论发现多少重复，`deslop` 都会以 `0` 退出——除非你选择启用门禁。届时，当仓库整体的 `duplication_percent` 超过你设定的上限时，它会以 `3` 退出。有两种方式；命令行标志优先于配置键：

```bash
deslop . --fail-over 5.0   # exit 3 when duplication_percent > 5.0
```

```toml
# .deslop.toml — shared by local runs, CI, and agents
[threshold]
max_duplication_percent = 5.0
```

`--fail-over 0` 对任何重复都会失败。`--no-fail-over` 为单次本地运行清除门禁。该值必须是 `[0.0, 100.0]` 范围内的有限数；其他任何值都会以 `2` 退出。

### 退出码

| 码 | 含义 |
| --- | --- |
| `0` | 成功；在阈值内，或未设置阈值。 |
| `1` | 运行时错误——扫描路径无效、解析/I-O 失败，或无法访问 `required` 的嵌入提供方。绝不会是 panic。 |
| `2` | 用法错误——未知标志，或超出范围 / 非有限的阈值。 |
| `3` | **阈值被突破。** 完整报告仍会写入磁盘，以便呈现这些重复。 |

### GitHub Actions

```yaml
name: deslop
on: [push, pull_request]
jobs:
  duplication-gate:
    runs-on: ubuntu-latest
    env:
      DESLOP_VERSION: "0.1.0"   # pin the tool version — see the Releases page
    steps:
      - uses: actions/checkout@v4
      - name: Install the Deslop CLI
        run: |
          curl -sSfL "https://github.com/Nimblesite/Deslop/releases/download/v${DESLOP_VERSION}/deslop-${DESLOP_VERSION}-linux-x64.tar.gz" | tar -xz
          echo "$PWD/deslop-${DESLOP_VERSION}-linux-x64" >> "$GITHUB_PATH"
      - name: Gate on duplication
        run: deslop . --fail-over 5.0   # or omit --fail-over to use [threshold] in .deslop.toml
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: deslop-report
          path: deslop-report.html
```

非零退出会使该步骤失败。`if: always()` 的上传即使在阈值被突破时也会保留 `deslop-report.html`，以便人工浏览这些重复。

## 读取报告

`deslop-report.json` 是规范文件，也是你**唯一**应当解析的文件——`.txt` 和 `.html` 都是基于它的渲染器。调用 `schema-doc` 一次以获取权威 schema，完整结构参见[输出格式](/zh/docs/output-formats/)。与决策相关的切片：

```json
{
  "metrics": {
    "duplication_percent": 2.63,
    "threshold": { "percent": 5.0, "breached": false, "source": "config" }
  },
  "clusters": [
    {
      "id": "0362505641efe3c7",
      "weight": 1252.8,
      "size": 3,
      "bucket": "identical",
      "signals": { "structural": 1.0, "token_jaccard": 0.98, "embedding_cos": 0.0, "fused": 0.99 },
      "occurrences": [ { "path": "src/UserRepository.cs", "start_line": 12, "end_line": 41, "hidden": false } ],
      "summary": "3 near-identical copies — safe to extract."
    }
  ]
}
```

| 字段 | 如何据此行动 |
| --- | --- |
| `metrics.duplication_percent` | CI 门禁用以比较的仓库整体核心指标数字。 |
| `metrics.threshold.breached` | `true` → 该次运行以 `3` 退出且门禁失败。`source` 为 `cli`、`config` 或 `none`。 |
| `clusters` | 按 `weight` **降序**排序——`clusters[0]` 始终是最严重的重复。自上而下处理。 |
| `bucket` | `identical` / `nearly_identical` → 提取一个共享定义。`loosely_similar` → 将差异参数化。`same_behavior` → 调和同一行为的两份实现（需要 `--embeddings`）。 |
| `signals.fused` | 单位区间内的置信度。`≥ 0.85` 是立即行动线，与上文 `find-similar` 法则中的阈值相同。 |
| `occurrences[].hidden` | `true` 标记一次 `report_hide` 匹配——即生成代码的手写克隆。 |

不要通过放宽阈值、将代码标记为 `hidden`，或将其拆分成微不足道的不同形态来压制发现项。如果 Deslop 标记了它，在你证明并非如此之前，应将其视为真实信号。
