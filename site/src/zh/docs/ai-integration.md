---
layout: layouts/docs.njk
title: AI 智能体 — 面向 Claude Code、Cursor、Copilot 的 MCP 设置
description: 在编码智能体写出另一个副本之前，告诉它相似代码已经存在。将 deslop-mcp 接入 Claude Code、Cursor、Continue 或 Codex，并用 find-similar 预防重复。
keywords: deslop, mcp 服务器, claude code, cursor, copilot, continue, codex, find-similar, 重复代码, 编码智能体
eleventyNavigation:
  key: AI 智能体
  order: 3
icon: smart_toy
lang: zh
---

# AI 智能体

**在你的编码智能体写出另一个副本之前，Deslop 会告诉它相似代码已经存在。** 智能体发问，Deslop 依据它正在工作的这个仓库的实时分析作答。没有定时任务，没有批处理，也不需要谁记得去跑一次扫描。

**本页是写给你 —— 正在做配置的人类。** 它说明 MCP 服务器提供了什么，以及如何把各个客户端连接上去。

写给智能体本身的说明 —— 它在写代码前遵循的规则、相似度阈值、MCP 不可用时的 CLI 回退，以及如何解析报告 —— 都在 **[面向 AI](/zh/docs/for-ai/)**。那一页以第二人称写成，直接写给机器。请把你的智能体指向那个 URL。

可直接粘贴到你项目 `AGENTS.md` / `CLAUDE.md` 的规则块参见[智能体配方](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md)。它适用于 Claude Code、Cursor、Copilot、Continue 与 Codex。

## MCP 工具，全部实时

只有 `find-similar` 属于编写代码的内循环。其余的全是只读报告查询，或按需取用的配置工具，因此智能体的工作上下文保持精简，而不必背负一整面墙的工具输出。

| 工具 | 何时调用 |
| --- | --- |
| `find-similar` | **在**编写新代码**之前**——是否已存在等价实现？这就是预防工具。 |
| `top-offenders` | 工作区中最严重的簇，最严重者优先。从这里开始清理。 |
| `cluster-by-id` | 你即将合并的某个簇的完整成员列表与信号。 |
| `report-for-file` | 单文件的簇切片。 |
| `report-for-range` | 单选区的簇切片。 |
| `report-get` | 整个工作区的报告。 |
| `report-query` | 对报告的过滤查询。 |
| `rescan` | 在大规模外部变更后强制刷新。 |
| `list-embedding-models` | 提供方公布的模型。 |
| `set-embedding-model` | 在运行时切换「行为相同、代码不同」[Type-4] 语义模型。 |
| `session-config` | 检查运行中服务器的生效配置。 |
| `schema-doc` | 每个响应的权威 JSON schema。每个会话调用**一次**，而非每次响应都调用。 |

每一个响应都针对**实时**工作区状态计算。编辑器服务器在内存中持有实时报告，并在每次变更时刷新（防抖，并设有硬上限）；MCP 服务器则在下一次工具调用时通过本地 IPC 端点读取该实时状态。macOS 与 Linux 使用 `.deslop/cache/deslop.sock`；Windows 使用通过 `.deslop/cache/deslop.port` 发现的 token 门控 TCP 回环端点。没有批处理步骤。没有陈旧缓存。

## 将 `deslop-mcp` 接入你的客户端——指向 VSIX 捆绑的二进制文件

`deslop-mcp` **随 VS Code 扩展 VSIX 一同发布**。安装扩展后，每个外部 MCP 客户端（Claude Code、Claude Desktop、Codex、Cursor、Continue）都应通过绝对路径引用解包后的 VSIX 二进制文件，这样智能体运行的就是扩展所发布的那个确切二进制文件——与 VSIX 版本锁定，不会发生 `PATH` 漂移。

从 Marketplace 安装扩展后，二进制文件位于：

```
~/.vscode/extensions/nimblesite.deslop-live-<VERSION>-<platform>/bin/<platform>/deslop-mcp
```

`<platform>` 为 `darwin-arm64`、`darwin-x64`、`linux-x64`、`linux-arm64` 或 `win32-x64`。`<VERSION>` 为已安装的扩展版本——每次更新 VSIX 时都要相应递增。

### Claude Code

```bash
claude mcp add deslop -s user -- \
  ~/.vscode/extensions/nimblesite.deslop-live-<VERSION>-darwin-arm64/bin/darwin-arm64/deslop-mcp \
  --root .
```

### Codex (`~/.codex/config.toml`)

```toml
[mcp_servers.deslop]
command = "/Users/you/.vscode/extensions/nimblesite.deslop-live-<VERSION>-darwin-arm64/bin/darwin-arm64/deslop-mcp"
args    = ["--root", "."]
```

### Claude Desktop (`claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "deslop": {
      "command": "/Users/you/.vscode/extensions/nimblesite.deslop-live-<VERSION>-darwin-arm64/bin/darwin-arm64/deslop-mcp",
      "args": ["--root", "/absolute/path/to/your/repo"]
    }
  }
}
```

> **不要让 MCP 客户端指向 `cargo install` 或 `target/release` 构建产物。** 从源码构建 Deslop 是为了测试你刚做的改动；它不是分发渠道。本仓库刻意不提供 `make install-binary` 目标。

## Homebrew / Scoop CLI 用户——指向 `$PATH` 上的裸 `deslop-mcp`

如果你通过 `brew install nimblesite/tap/deslop` 或 `scoop install deslop` 安装了 CLI，该包还会把 **`deslop-mcp` 和 `deslop-lsp` 一并放到你的 `$PATH`** 上，与 `deslop` 并列——tap formula 和 Scoop manifest 会安装全部三个二进制文件，并与发布版本锁定。无需 VSIX、无需扩展目录、无需绝对路径。直接使用裸命令：

```bash
claude mcp add deslop -s user -- deslop-mcp --root .
```

```json
{
  "mcpServers": {
    "deslop": {
      "command": "deslop-mcp",
      "args": ["--root", "."]
    }
  }
}
```

同样的 `"command": "deslop-mcp"` 形式适用于 Codex（`~/.codex/config.toml`）、Cursor 和 Continue。它也是签入 `.mcp.json` 或团队共享配置的正确取值——每台机器都通过 `$PATH` 解析它。

需要知道的三点：

- **不存在 `deslop mcp` 子命令。** `deslop` CLI 只用于一次性运行和 CI 审计；MCP 由**独立的 `deslop-mcp` 二进制文件**提供。
- **`$PATH` 上找不到 `deslop-mcp`？** 它是在 v0.13.0 加入 brew/scoop 包的。在较旧的安装上，运行 `brew upgrade deslop`（或 `scoop update deslop`）——当前发布版本会把 `deslop-mcp` 和 `deslop-lsp` 放到 `$PATH` 上。
- **从源码构建不会把任何东西放到 `$PATH` 上。** 只有 `brew` / `scoop` 会这么做。这些包管理器会让二进制文件与发布版本步调一致地版本化；`cargo build` 不会。

## 智能体循环

主打的工作流是响应式的，而非批处理：

1. 智能体提出一个改动。在它写出新代码之前，它通过 MCP 对候选片段调用 `find-similar`。
2. 如果 `find-similar` 返回一个高于所配置相似度下限的簇，智能体就复用规范实现，或重写该调用点。
3. 当智能体编辑文件时，LSP 文件监视器触发 `deslop/reportChanged`。MCP 服务器通过 IPC 套接字查询 LSP 刚刷新的报告，并在下一次工具调用时提供新状态。
4. 智能体重新查询 `top-offenders` 或 `report-for-file`，确认该簇已消失。无需重新运行、无需标志、无需批处理 CLI 调用。

当 MCP 不可用时 —— CI、冷缓存审计，或没有 MCP 客户端的智能体 —— 循环会降级到 `deslop` CLI，它运行完全相同的流水线，产出完全相同的 JSON。指纹缓存默认开启，因此编辑后的重新运行只会重新解析发生变化的文件。逐步的回退方案见[面向 AI](/zh/docs/for-ai/#cli-fallback)。

## 配置它

为某个仓库配置 Deslop 的智能体需要三样东西，全部记录在[配置与报告参考](/zh/docs/configuration/)中：

- **[`exclude` 与 `report_hide`](/zh/docs/configuration/#exclude-vs-report-hide)** —— `exclude` 在分析前丢弃文件；`report_hide` 会分析它但不让它进入头条，因此「手写代码与生成代码重复」仍会浮现。
- **[内置规则](/zh/docs/configuration/#built-in-rules)** —— `node_modules`、`target`、`dist`、生成代码后缀以及生成横幅检测均已覆盖。不要重复添加。
- **[`[threshold]`](/zh/docs/configuration/#threshold)** —— 需要显式启用的 CI 门禁。把上限提交进仓库，让本地运行、CI 与智能体共用同一个数字。

要基于重复拦截构建，请使用 [GitHub Action](/zh/docs/github-action/) —— 它封装了同一套退出码约定。

## 智能体读回什么

`deslop-report.json` 是规范产物；`.txt` 与 `.html` 是其之上的渲染器。每份报告都带有内嵌的 `schema_doc`，因此模型无需另一份参考文档就能解析载荷。逐字段说明 —— `bucket`、`signals.fused` 与 `occurrences[].hidden` 各自的含义以及如何据此行动 —— 见[面向 AI](/zh/docs/for-ai/#read-the-json)。

## 一套引擎，三种接口

`deslop-core` crate 拥有整条流水线。三个外壳消费它：

- **MCP 服务器（`deslop-mcp`）**——智能体接口面。find-similar 加上一组聚焦的只读与配置工具（见上表）。该服务器将每一次读取——`top-offenders`、`report-get`、`report-for-file`、`find-similar` 及其余——通过本地 IPC 端点委托给运行中的 LSP，因此每个响应都针对 LSP 的实时内存语料库计算，而非陈旧的磁盘缓存。Unix 主机使用 `.deslop/cache/deslop.sock`；Windows 使用带 `.deslop/cache/deslop.port` 发现记录的 token 门控 TCP 回环端点。当 LSP 未运行时，MCP 返回一条可操作的错误；CI 与一次性审计则改用 `deslop` CLI。
- **LSP 服务器（`deslop-lsp`）**——编辑器接口面。诊断、悬停、代码透镜、`textDocument/definition`、虚拟 `deslop://` 文档，以及自定义的 `deslop/*` 方法（`reportGet`、`reportDelta`、`reportForFile`、`reportForRange`、`clusterById`、`duplicatesFindSimilar`、`embeddingListModels`、`embeddingSetModel`、`sessionConfig`、`reportSchemaDoc`、`virtualDocument`、`cpuReport`）。触发 `deslop/reportChanged`、`deslop/analysisState` 与 `deslop/embeddingProgress` 通知。拥有文件监视器、防抖器与分析调度器。
- **CLI（`deslop`）**——面向 CI 门禁与一次性审计的冷缓存兜底方案。

三者复用相同的缓存布局（`.deslop/cache/fingerprints/`、`.deslop/cache/embeddings/`）与相同的 JSON schema。如今接入 CLI 的智能体只需把 `deslop-mcp` 加入其 MCP 配置即可获得实时通道——无需 schema 变更，无需重写解析器。

### 推送通知

只要一次监视器扫描完成，LSP 就会通过 LSP 线路触发 `deslop/reportChanged`，并通过 MCP 线路触发 `resources/updated` + `deslop/reportChanged`。编辑器接口面、智能体缓存与 webview 都会在该次扫描提交后立即观察到新报告。按照 [LIVE-IS-REACTIVE](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/principles.md#principles-live-is-reactive) 不变式，陈旧的 UI 属于正确性缺陷。

## JetBrains 插件（开发中）

位于 `clients/jetbrains/` 的 JetBrains 插件注册了一个 IntelliJ Platform 的 `lsp.serverSupportProvider`，并为 C#、Rust、Python、Dart、JavaScript、TypeScript、PHP、F# 与 Go 文件启动 `deslop-lsp`。Rider 是第一个产品目标；IntelliJ IDEA、PyCharm、WebStorm、RustRover 与 CLion 将在同一平台 LSP API 上紧随其后。该插件以 Gradle 构建，针对已发布的 `deslop-lsp` 进行真实二进制测试，并随附与 VS Code 扩展相同的二进制解析规则。Zed 与 Neovim 插件已在路线图上——两者均具备 LSP 能力，如今都与 `deslop-lsp` 线路兼容。

## Deslop 刻意不做的事

- 它不会重写你的代码。Deslop 负责发现、排名、比较与预防重复；提取由你决定。自动清理是方向，而非已发布的能力。
- 它不会让 CI 失败，除非你自己设置阈值。
- 它不会假定"近似命中 = bug"。有些重复是有意为之的（测试夹具、引导代码）。Deslop 负责报告；由你来决定。
- 它不会访问网络，除非你显式选择一个远程嵌入模型。
