---
layout: layouts/docs.njk
title: 面向 AI — 编码智能体如何用 Deslop 阻止重复代码
description: 本页写给智能体本身。写代码前先调用 find-similar；当 MCP 不可用时，回退到 deslop CLI。含阈值、JSON 字段与字节范围。
keywords: deslop, AI 智能体, 编码智能体, find-similar, mcp, CLI 回退, 重复代码, Claude Code, Cursor, Codex
eleventyNavigation:
  key: 面向 AI
  order: 4
icon: terminal
docsGroup: reference
lang: zh
---

# 面向 AI

**本页是写给你的 —— 编码智能体。** 下面的每一条都是你可以直接执行的指令，而不是写给别人评估的产品说明。如果你是正在配置 MCP 客户端的人类，你要看的是 [AI 智能体](/zh/docs/ai-integration/)；配置完成后，把你的智能体指向本页这个 URL。

Deslop 存在的意义，是让重复代码从一开始就不会被写出来。写代码的是你，所以必须做检查的也是你。

## 唯一铁律：动手写之前先检查

在你编写任何新的代码单元之前 —— 函数、方法、类、辅助函数、fixture、测试初始化、解析分支、错误类型、路由处理器、视图模型 —— 调用 `find-similar`，传入拟写的代码片段（或 `path` + `start_byte` + `end_byte` 字节范围），然后读取响应中的 `signals.fused`：

| `signals.fused` | 分桶 | 你该怎么做 |
| --- | --- | --- |
| `≥ 0.85` | `identical` / `nearly_identical` | **不要写这份副本。** 复用工具返回的规范出现位置。若两个调用点都无法直接复用，就提取一个共享辅助函数。 |
| `0.6 – 0.85` | 任意 | 先阅读规范出现位置再做决定。倾向于复用。 |
| `< 0.6` | 任意，或响应为空 | 可以编写。 |
| 任意 | `structural_only` | 仅形状匹配 —— 通常是同类样板代码。下结论之前先读规范出现位置。 |

不要因为这段代码“看起来是新的”就跳过这次调用。绝大多数重复代码，都是由某个确信它是新代码的人写下的。

`find-similar` 是**编写阶段**的工具。当你要清理*已经存在*的重复时，从 `top-offenders` 开始，再用 `cluster-by-id` 拉取你准备合并的那个簇。

可直接粘贴到项目 `AGENTS.md` / `CLAUDE.md` 的规则块，见[智能体配方](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md)。

<span id="cli-fallback"></span>

## 如果 MCP 服务器不可用，就改用 CLI

**不要因为一次工具调用失败就跳过检查，也不要凭记忆蒙。** 因为闸门失守而落地的重复代码，正是 Deslop 要防止的那种失败。请按下面的阶梯逐级处理。

### 1. 判断你遇到的是哪种故障

| 你看到的现象 | 含义 |
| --- | --- |
| `LSP is not running — start deslop-lsp to enable this tool.` | MCP 服务器接线正确，但持有实时分析的编辑器服务器没有启动。该错误会给出它尝试连接的套接字绝对路径。 |
| 同样的错误，但确实**有** `deslop-lsp` 在运行 | MCP 与 LSP 的 `--root` 不一致。将错误中的套接字路径与你正在编辑的工作区做比对。 |
| 根本不存在 `find-similar` 工具 | 本次会话没有配置任何 MCP 服务器。 |
| 工具调用超时或传输报错 | 视为不可用，直接降级到 CLI。 |

### 2. 先尝试恢复实时通道

如果工作区已在装有 Deslop 扩展的编辑器中打开，编辑器服务器会自行启动 —— 打开一个受支持的源文件，然后重试该工具调用。如果 MCP 与 LSP 对根目录的判断不一致，正确的修法是改 MCP 客户端的 `--root` 参数，而不是绕过去。

### 3. 否则，降级到 CLI

`deslop` CLI 运行的是完全相同的流水线，产出完全相同的 JSON schema。在仓库根目录运行：

```bash
deslop . --notext --nohtml --no-color
```

这会把规范报告写入 `.deslop/deslop-report.json` —— 这是你唯一应该解析的文件。`--notext --nohtml` 跳过你并不需要的两种人类可读渲染；`--no-color` 让 stderr 摘要在日志中保持干净。

**要如实认识这样做的代价。** CLI 中没有代码片段查询 —— `find-similar` 是 MCP 工具，而 CLI 无法评估你尚未写出的代码。因此 CLI 的循环是*即时发现*，而非*事前预防*：

1. 开始工作前先跑一次 `deslop .`，得到一个基线。
2. 动笔之前，在基线的 `clusters[]` 中检索你即将改动的文件及其相邻文件。如果某个簇已经覆盖了你打算新增的模式，就复用它的规范出现位置 —— 这就是 CLI 版本的“预防”，它能拦住常见情形。
3. 写下你的改动。
4. 重新运行 `deslop . --notext --nohtml`。指纹缓存默认开启，因此这次只会重新解析你实际改动过的文件 —— 开销与你的改动量成正比，而不是与仓库规模成正比。请按每次改动运行，而不是每个会话只跑一次。
5. 在 `clusters[].occurrences[]` 中搜索你刚写入的路径。如果你的新代码出现在某个 `signals.fused ≥ 0.85` 的簇里，那你刚刚写下了一份重复代码。趁改动还在工作区里，立刻把它收敛掉。
6. 再跑一次，确认该簇已经消失或变小了。

当全仓库范围的重复越过配置的上限时，运行会以 `3` 退出；即便触发门禁，报告依然会被写出，所以无论如何都要解析它。完整对照表见[退出码](/zh/docs/configuration/#exit-codes)。

如果 MCP 和 CLI 都不可用，请如实说明并停下来。不要猜。

<span id="read-the-json"></span>

## 读取 JSON

`deslop-report.json` 是规范产物，也是你唯一应该解析的文件 —— `.txt` 与 `.html` 都是它之上的渲染器。每份报告都以内嵌的 `schema_doc` 开头，用于描述其自身结构，因此你无需另找参考文档就能读懂载荷。通过 MCP 时，`schema-doc` 每个会话只调用**一次**，绝不要每次响应都调。

```json
{
  "tool_version": "0.0.0-dev",
  "schema_doc": "…inline description of every field…",
  "metrics": {
    "analysed_loc": 1832044,
    "duplicated_loc": 48120,
    "duplication_percent": 2.63,
    "clusters_total": 142,
    "duplicated_files": 318,
    "threshold": { "percent": 5.0, "breached": false, "source": "config" }
  },
  "action_hints": [
    { "pattern": "bucket=identical", "recommendation": "Identical code. Safe to extract — every copy is the same." }
  ],
  "clusters": [
    {
      "id": "0362505641efe3c7",
      "weight": 2184.0,
      "bucket": "nearly_identical",
      "size": 3,
      "canonical_node_count": 42,
      "signals": { "structural": 1.0, "token_jaccard": 0.97, "embedding_cos": 0.91, "fused": 0.99 },
      "summary": "3 near-identical copies of a 42-node method across UserRepository.cs:120-180, ProductRepository.cs:58-118, OrderRepository.cs:40-102 — safe to extract.",
      "interpretation": "Nearly identical code. Review the locations — small differences may matter.",
      "occurrences": [
        { "path": "UserRepository.cs", "start_byte": 3104, "end_byte": 4820, "start_line": 120, "end_line": 180, "hidden": false }
      ]
    }
  ]
}
```

`summary` 与 `interpretation` 就是为你写的：它们说明发现了什么、在哪里，以及当各路信号一致时，这处重复是否可以安全提取。仓库级别的建议位于顶层的 `action_hints`，以 `bucket` 为键，由信号推导得出，而非凭空猜测。

| 字段 | 如何据此行动 |
| --- | --- |
| `metrics.duplication_percent` | CI 门禁所比对的全仓库头号数字。 |
| `metrics.threshold.breached` | `true` → 本次运行以 `3` 退出，门禁失败。`source` 为 `cli`、`config` 或 `none`。 |
| `clusters` | 按 `weight` **降序**排列 —— `clusters[0]` 永远是最严重的那个。自上而下处理，不要从中间插手。 |
| `bucket` | `identical` / `nearly_identical` → 提取共享定义。`structural_only` → 仅形状匹配，没有词元或语义证据；提取前先确认这确实是真重复。`loosely_similar` → 将差异参数化。`same_behavior` → 将同一行为的两份实现合并（需要启用嵌入）。 |
| `signals.fused` | 取值有界的置信度。`≥ 0.85` 是立即行动线 —— 与上文铁律中的阈值相同。 |
| `occurrences[].hidden` | `true` 表示命中了 `report_hide` —— 通常是生成代码的手写副本。 |

### 用字节范围，而不是行号

Deslop 的真相来源是 `[start_byte, end_byte)`。行号只是在渲染时为人类推导出来的。编辑时请按字节范围切片 —— 一旦周围代码移动，基于行号的编辑就会发生漂移。

### 簇 id 是稳定的

簇 id 取自该簇最小成员 BLAKE3 哈希的前 8 个字节，渲染为 16 个十六进制字符（例如 `0362505641efe3c7`）。它不含时间戳，因此同一个仓库用同一个二进制分析两次会得到相同的 id。跨运行、在 issue 中以及在你自己的笔记里，都请用 id 引用簇 —— 不要用排名，排名是渲染时的位置，会随仓库变化而移动。

## 为仓库做配置

如果你是在*配置* Deslop 而不是消费它，有三件事几乎决定了一切，它们都在[配置参考](/zh/docs/configuration/)中：

- **[`exclude` 与 `report_hide`](/zh/docs/configuration/#exclude-vs-report-hide)** —— `exclude` 在分析前就丢弃文件；`report_hide` 会分析它但将其排除在头号数字之外，因此“手写代码重复了生成代码”这一情形依然能浮现。
- **[内置规则](/zh/docs/configuration/#built-in-rules)** —— `node_modules`、`target`、`dist`、生成代码后缀与生成标记横幅均已覆盖。不要重复添加。
- **[`[threshold]`](/zh/docs/configuration/#threshold)** —— 可选启用的 CI 门禁。把上限提交入库，让本地运行、CI 与智能体共享同一个数值。

要为构建设置门禁，请使用 [GitHub Action](/zh/docs/github-action/)；它封装了同一套退出码约定。

## 你不可以破坏的规则

- **不要为了让告警消失而消音。** 调高阈值、添加 `report_hide` 模式来掩埋你自己的代码，或把一处重复拆成几个只有细微差别的形状 —— 这些都是失败，不是修复。
- **在证明某个告警是噪声之前，不要把它当成噪声。** 如果 Deslop 报告了它，先把两处出现位置都读一遍。
- **不要盲目合并 `same_behavior` 匹配。** 该分桶来自语义嵌入。请阅读两处位置；代码之所以看起来不同，往往是有原因的。
- **有些重复是有意为之。** 测试 fixture 与引导代码是常见且正当的例外。接受一处重复是合理的结论 —— 但*默默*接受它则不是。请说明你接受了哪个簇，以及为什么。
- **Deslop 不会重写你的代码。** 它负责发现、排名、比较与预防。提取重构由你来写，也由你负责写对。
