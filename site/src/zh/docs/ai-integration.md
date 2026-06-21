---
layout: layouts/docs.njk
title: AI 集成 — 面向 Claude、Cursor、Copilot 的 MCP 服务器
description: 将 deslop-mcp 接入 Claude Code、Claude Desktop、Cursor、Continue 或 Codex。一个精简、实时、以 find-similar 为核心的 MCP 接口面——在复制粘贴落地之前就将其预防。
eleventyNavigation:
  key: AI 集成
  order: 3
icon: smart_toy
lang: zh
---

# AI 集成

Deslop 从第一次提交起就将编码智能体设计为一等受众。MCP 与 LSP 外壳如今均已发布——两者都消费同一套 `deslop-core` 流水线、同一套 JSON schema，以及与 CLI 相同的磁盘缓存。每一个 MCP 工具响应都针对**实时**工作区状态计算——LSP 在内存中持有实时报告，并在每次变更时刷新（防抖，并设有硬上限），MCP 服务器则在下一次工具调用时通过本地 IPC 端点读取该实时状态。macOS 与 Linux 使用 `.deslop-cache/deslop.sock`；Windows 使用通过 `.deslop-cache/deslop.port` 发现的 token 门控 TCP 回环端点。没有批处理步骤。没有陈旧缓存。

## MCP 工具，全部实时

只有 `find-similar` 属于编写代码的内循环——它是智能体在写新代码之前所做的那一次调用。其余的全是只读报告查询，或按需取用的配置工具，因此智能体的工作上下文保持精简，而不必背负一整面墙的工具输出。

| 工具 | 用途 |
| --- | --- |
| `find-similar` | **预防工具。** 在智能体写出新代码之前，将候选代码片段与实时语料库进行匹配。 |
| `top-offenders` | 工作区中排名最严重的簇。 |
| `report-for-file` | 单文件的簇切片。 |
| `report-for-range` | 单选区的簇切片。 |
| `cluster-by-id` | 单个簇的完整成员列表与信号。 |
| `report-get` | 整个工作区的报告。 |
| `report-query` | 对报告的过滤查询。 |
| `rescan` | 在大规模外部变更后强制刷新。 |
| `list-embedding-models` | 提供方公布的模型。 |
| `set-embedding-model` | 在运行时切换 Type-4（行为相同、代码不同）语义模型。 |
| `session-config` | 检查运行中服务器的生效配置。 |
| `schema-doc` | 每个响应的自描述 JSON schema。 |

## 将 `deslop-mcp` 接入你的客户端——指向 VSIX 捆绑的二进制文件

`deslop-mcp` **随 VS Code 扩展 VSIX 一同发布**。安装扩展后，每个外部 MCP 客户端（Claude Code、Claude Desktop、Codex、Cursor、Continue）都应通过绝对路径引用解包后的 VSIX 二进制文件，这样智能体运行的就是扩展所发布的那个确切二进制文件——与 VSIX 版本锁定，不会发生 `PATH` 漂移。

从 Marketplace 安装扩展后，二进制文件位于：

```
~/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/<platform>/deslop-mcp
```

`<platform>` 为 `darwin-arm64`、`darwin-x64`、`linux-x64`、`linux-arm64` 或 `win32-x64`。`<VERSION>` 为已安装的扩展版本——每次更新 VSIX 时都要相应递增。

### Claude Code

```bash
claude mcp add deslop -s user -- \
  ~/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/darwin-arm64/deslop-mcp \
  --root .
```

### Codex (`~/.codex/config.toml`)

```toml
[mcp_servers.deslop]
command = "/Users/you/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/darwin-arm64/deslop-mcp"
args    = ["--root", "."]
```

### Claude Desktop (`claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "deslop": {
      "command": "/Users/you/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/darwin-arm64/deslop-mcp",
      "args": ["--root", "/absolute/path/to/your/repo"]
    }
  }
}
```

> **不要让 MCP 客户端指向 `cargo install` 或 `target/release` 构建产物。** 从源码构建 Deslop 是为了测试你刚做的改动；它不是分发渠道。本仓库刻意不提供 `make install-binary` 目标。Deslop 支持的唯一通过 PATH 解析的形式是面向 CLI 用户的 `brew install nimblesite/tap/deslop` / `scoop install deslop`——这些包管理器会让二进制文件与发布版本步调一致地版本化。

## 预防胜于治疗——`find-similar` 是基石

最快的去重是那种永远不会落地的去重。Deslop 的 MCP 服务器将 `find-similar` 暴露为**预防**工具：智能体在写新函数、辅助函数或测试设置*之前*调用它。如果候选模式已经以高相似度存在，智能体就复用规范实现，而不是另写一份新的副本。

可直接粘贴、用于把这一点教给你的智能体的 `AGENTS.md` / `CLAUDE.md` 片段位于 [`docs/snippets/agents-md-recipe.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md)。它适用于 Claude Code、Cursor、Copilot、Continue 与 Codex。

## 智能体循环（实时 MCP）

主打的工作流是响应式的，而非批处理：

1. 智能体提出一个改动。在它写出新代码之前，它通过 MCP 对候选片段调用 `find-similar`。
2. 如果 `find-similar` 返回一个高于所配置相似度下限的簇，智能体就复用规范实现，或重写该调用点。
3. 当智能体编辑文件时，LSP 文件监视器触发 `deslop/reportChanged`。MCP 服务器通过 IPC 套接字查询 LSP 刚刷新的报告，并在下一次工具调用时提供新状态。
4. 智能体重新查询 `top-offenders` 或 `report-for-file`，确认该簇已消失。无需重新运行、无需标志、无需批处理 CLI 调用。

对于仅 CLI 的循环（CI 门禁、冷缓存审计，或没有 MCP 的智能体），工作流降级为：

1. 智能体提出代码改动。
2. 智能体（或外壳）运行 `deslop . --output report`（写出 `report.json`/`.txt`/`.html`）。
3. 智能体从 `report.json` 读取排名靠前的 `N` 个簇。
4. 对于每个高于阈值的簇，智能体有三种选择：提取为共享函数、复用现有实现，或接受重复并注明原因。
5. 智能体重新运行 Deslop。排名最靠前的簇应当不同或更小。

增量缓存（`--incremental`）意味着第 5 步只为解析智能体实际改动过的文件付出成本——未改动的文件完全跳过 tree-sitter，因此一次热运行的耗时与改动规模成正比，而非与仓库规模成正比。

## 阅读 JSON

每个报告都以一段嵌入的 `schema_doc` 开头，向消费它的智能体解释其结构——模型无需另一份参考资料就能理解负载：

```json
{
  "tool_version": "0.0.0-dev",
  "schema_doc": "…inline description of every field…",
  "metrics": { "analysed_loc": 1832044, "duplicated_loc": 48120, "duplication_percent": 2.63, "clusters_total": 142, "duplicated_files": 318 },
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
        { "path": "UserRepository.cs", "start_byte": 3104, "end_byte": 4820, "start_line": 120, "end_line": 180 }
      ]
    }
  ]
}
```

`summary` 与 `interpretation` 是为智能体读者预先撰写的：它们说明发现了什么、在哪里，以及——当信号一致时——该重复是否可以安全提取。仓库级别的修复指导位于顶层 `action_hints`，以 `bucket` 为键；它由信号推导得出，绝不靠猜测。

## 字节范围，而非行号

Deslop 的事实来源是 `[byte_start, byte_end)`。行号仅在渲染时派生。编辑文件的智能体应按字节范围切片——当周围代码移动时，基于行的编辑会发生漂移。

## 稳定的 ID

簇 ID 是簇内容指纹的短十六进制摘要——取簇中最小成员 BLAKE3 哈希的前 8 字节，渲染为 16 个十六进制字符（例如 `0362505641efe3c7`）。它们不携带时间戳，因此将同一个仓库两次喂给同一个二进制文件会产生相同的 ID。智能体可以跨多次运行引用同一个簇。

## MCP 与 LSP——已发布

`deslop-core` crate 拥有整条流水线。三个外壳消费它：

- **MCP 服务器（`deslop-mcp`）**——智能体接口面。find-similar 加上一组聚焦的只读与配置工具（见上表）。该服务器将每一次读取——`top-offenders`、`report-get`、`report-for-file`、`find-similar` 及其余——通过本地 IPC 端点委托给运行中的 LSP，因此每个响应都针对 LSP 的实时内存语料库计算，而非陈旧的磁盘缓存。Unix 主机使用 `.deslop-cache/deslop.sock`；Windows 使用带 `.deslop-cache/deslop.port` 发现记录的 token 门控 TCP 回环端点。当 LSP 未运行时，MCP 返回一条可操作的错误；CI 与一次性审计则改用 `deslop` CLI。
- **LSP 服务器（`deslop-lsp`）**——编辑器接口面。诊断、悬停、代码透镜、`textDocument/definition`、虚拟 `deslop://` 文档，以及自定义的 `deslop/*` 方法（`reportGet`、`reportDelta`、`reportForFile`、`reportForRange`、`clusterById`、`duplicatesFindSimilar`、`embeddingListModels`、`embeddingSetModel`、`sessionConfig`、`reportSchemaDoc`、`virtualDocument`、`cpuReport`）。触发 `deslop/reportChanged`、`deslop/analysisState` 与 `deslop/embeddingProgress` 通知。拥有文件监视器、防抖器与分析调度器。
- **CLI（`deslop`）**——面向 CI 门禁与一次性审计的冷缓存兜底方案。

三者复用相同的缓存布局（`.deslop-cache/fingerprints/`、`.deslop-cache/embeddings/`）与相同的 JSON schema。如今接入 CLI 的智能体只需把 `deslop-mcp` 加入其 MCP 配置即可获得实时通道——无需 schema 变更，无需重写解析器。

### 推送通知

只要一次监视器扫描完成，LSP 就会通过 LSP 线路触发 `deslop/reportChanged`，并通过 MCP 线路触发 `resources/updated` + `deslop/reportChanged`。编辑器接口面、智能体缓存与 webview 都会在该次扫描提交后立即观察到新报告。按照 [LIVE-IS-REACTIVE](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/principles.md#principles-live-is-reactive) 不变式，陈旧的 UI 属于正确性缺陷。

## JetBrains 插件（开发中）

位于 `clients/jetbrains/` 的 JetBrains 插件注册了一个 IntelliJ Platform 的 `lsp.serverSupportProvider`，并为 `.cs`、`.rs` 与 `.py` 文件启动 `deslop-lsp`。Rider 是第一个产品目标；IntelliJ IDEA、PyCharm、WebStorm、RustRover 与 CLion 将在同一平台 LSP API 上紧随其后。该插件以 Gradle 构建，针对已发布的 `deslop-lsp` 进行真实二进制测试，并随附与 VS Code 扩展相同的二进制解析规则。Zed 与 Neovim 插件已在路线图上——两者均具备 LSP 能力，如今都与 `deslop-lsp` 线路兼容。

## Deslop 刻意不做的事

- 它不会重写你的代码。提取由你决定。
- 它不会让 CI 失败，除非你自己设置 `--fail-over <percent>`——一个仓库级的重复百分比门禁，超出时以 `3` 退出。
- 它不会假定"近似命中 = bug"。有些重复是有意为之的（测试夹具、引导代码）。Deslop 负责报告；由你来决定。
- 它不会访问网络，除非你显式选择一个远程嵌入模型。
