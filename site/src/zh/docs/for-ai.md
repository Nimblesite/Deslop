---
layout: layouts/docs.njk
title: 面向 AI — 编码智能体如何用 Deslop 阻止重复代码
description: 面向编码智能体的操作说明：找到 Deslop 二进制；写代码前调用 find-similar；MCP 异常时改用 CLI；复用缓存；先从完全相同的克隆开始去重。
eleventyNavigation:
  key: 面向 AI
  order: 4
icon: terminal
docsGroup: reference
lang: zh
---

# 面向 AI

本页写给你——编码智能体。内容包括：如何在本机找到 Deslop，如何使用 MCP 服务器，MCP 服务器异常时怎么办，如何运行 CLI 而不重复已完成的工作，以及按什么顺序安全地消除重复。本页适用于 [0.35.0](/zh/releases/)。为 MCP 客户端做连接设置的人类请看[AI 智能体](/zh/docs/ai-integration/)。

## 你在使用什么

Deslop 是一个引擎，以三个程序发布。它们读取同一个 `.deslop.toml`，共用同一份缓存，产出同样的 JSON 报告。

| 程序 | 作用 | 何时使用 |
| --- | --- | --- |
| `deslop-lsp` | 编辑器里的引擎。分析工作区，把结果保存在内存中，并在每次文件变化时更新。 | 不直接使用，由编辑器启动。 |
| `deslop-mcp` | MCP 服务器。它没有自己的引擎；每次工具调用都是向同一文件夹正在运行的 `deslop-lsp` 发出的请求。 | 编辑器打开着该仓库时的首选。 |
| `deslop` | 命令行工具。分析你给出的文件夹，写出报告后退出。 | MCP 服务器不可用或行为异常时、CI 中，以及需要磁盘上的 HTML 或 JSON 报告时。 |

Deslop 写出的所有内容都在扫描根目录的 `.deslop/` 下：

| 路径 | 内容 |
| --- | --- |
| `.deslop/deslop-report.json` / `.txt` / `.html` | CLI 报告。JSON 给你读；HTML 给人看。 |
| `.deslop/cache/fingerprints/` | 按文件内容索引的逐文件解析结果。CLI 与编辑器共用。 |
| `.deslop/cache/live-report.json` | 编辑器最近一次完整分析的结果，保存下来以便下次编辑器启动时立即可用。 |
| `.deslop/cache/deslop.sock`（Windows 上为 `deslop.port`） | MCP 服务器连接编辑器引擎所用的本地端点。它存在说明有引擎正在（或曾经）为该文件夹运行。 |
| `.deslop/logs/` | CLI 日志。运行失败时查看。 |

## 找到二进制

### 已安装 VS Code 扩展：使用扩展自带的二进制

扩展捆绑了全部三个程序。优先使用它们，而不是 `PATH` 上的任何版本：它们与编辑器正在运行的引擎是同一次构建，所以 CLI 能读取编辑器的报告，数字也与编辑器一致。

macOS 与 Linux：

```bash
DESLOP=$(find ~/.vscode/extensions -maxdepth 4 -type f \
  -path '*/nimblesite.deslop-live-*/bin/*/deslop' 2>/dev/null | sort -V | tail -1)
"$DESLOP" --version
```

Windows（PowerShell）：

```powershell
$deslop = Get-ChildItem "$env:USERPROFILE\.vscode\extensions\nimblesite.deslop-live-*\bin\*\deslop.exe" |
  Sort-Object LastWriteTime | Select-Object -Last 1 -ExpandProperty FullName
& $deslop --version
```

目录结构是 `nimblesite.deslop-live-<VERSION>[-<platform>]/bin/<platform>/`，其中包含 `deslop`、`deslop-lsp` 和 `deslop-mcp`（Windows 上带 `.exe`）。`<platform>` 为 `darwin-arm64`、`darwin-x64`、`linux-x64`、`linux-arm64` 或 `win32-x64`。基于 VS Code 的其他编辑器把扩展放在各自的文件夹：把 `~/.vscode/extensions` 换成 `~/.vscode-insiders/extensions`、`~/.cursor/extensions`、`~/.windsurf/extensions` 或 `~/.vscode-oss/extensions`。如果装有多个版本，使用最新的；编辑器可能还没删除旧版本。

### 没有扩展：安装 CLI

macOS 与 Linux（Homebrew）：

```bash
brew install nimblesite/tap/deslop
deslop --version
```

Windows（Scoop）：

```powershell
scoop bucket add nimblesite https://github.com/Nimblesite/scoop-bucket
scoop install deslop
deslop --version
```

两者都会把同一版本的 `deslop`、`deslop-lsp` 和 `deslop-mcp` 装到 `PATH` 上。如果你的指令要求安装前先询问，就先询问。不要把 `cargo build` 产物或 `target/release` 中的副本当作已安装的工具：那是测试构建，会与编辑器运行的引擎产生版本漂移。

## MCP 服务器

MCP 的每个回答都来自编辑器的实时分析，并随文件变化而更新，没有批处理步骤。每个工具在 `tools/list` 中都有自己的说明；它们的配合方式如下：

| 工具 | 用途 |
| --- | --- |
| `find-similar` | **写代码之前**询问是否已有等价代码。传入 `snippet` 和 `language`，或 `path` 加 `start_byte` 和 `end_byte`。同时读取 `clusters` 与 `existing`：后者列出片段代码已在簇之外出现过一次的位置。`clusters` 为空而 `existing` 非空，意味着你正要制造一个重复。 |
| `duplicates` | 按严重程度从高到低列出簇。用 `path`（及 `start_byte`/`end_byte`）限定范围；用 `languages`、`path_contains`、`severities` 或 `min_size` 过滤。每页默认 5 个簇；概览时用 `detail: "summary"` 加较大的 `limit`，并用 `offset` 翻页，直到 `offset + returned >= total_clusters`。 |
| `cluster-by-id` | 修改某个簇之前获取它的全部出现位置。每页 100 个，用 `offset` 翻页。 |
| `merge-plan` | 手动编辑前，获取合并某个簇的只读计划，其中包含 `WorkspaceEdit`。 |
| `compare-pair` | 获取引擎对两个指定出现位置的证据。簇本身不携带配对测量值。 |
| `rescan` | 分析看起来落后于你的编辑时强制刷新，可只针对你刚修改的 `paths`。 |
| `session` | 查看工作区 `root`、设置和缓存状态。除非用户明确要求使用某个模型，否则不要调用 `set-embedding-model`。 |
| `schema-doc` | 获取每个字段的含义。每个会话调用一次。 |

响应有大小上限。带 `truncated: true` 的响应或带 `occurrences_truncated: true` 的簇是不完整的，而不是错误的：用 `offset` 翻页、调小 `limit`、使用 `detail: "summary"`，或调用 `cluster-by-id`。

### MCP 服务器工作不正常时

出故障的服务器返回的错误、超时或空结果，都不能证明不存在重复代码。遇到以下任一情况，改用 CLI：

| 现象 | 含义 | 处理方式 |
| --- | --- | --- |
| 没有 Deslop 工具，或连接失败 | MCP 客户端未配置，或服务器未启动。 | 使用 CLI，并告知用户 MCP 服务器未连接。 |
| `LspNotRunning` | 没有为该文件夹运行的编辑器引擎。错误中会写明尝试过的端点。 | 确认编辑器打开的是这个仓库，且 MCP 服务器的 `--root` 是同一文件夹。在此期间使用 CLI。 |
| 错误中出现两个不同的 Deslop 版本 | `deslop-mcp` 与 `deslop-lsp` 来自不同构建。 | 使用 CLI。请用户重装扩展，并让 MCP 客户端指向扩展内的二进制。 |
| 仍在分析中，或结果没有反映你的编辑 | 分析尚未跟上。 | 用你修改过的 `paths` 调用 `rescan`。仍然落后就改用 CLI。 |
| 结果与你读到的代码矛盾，例如明明看到复制的代码却没有匹配 | 可能是准确性缺陷。 | 用 CLI 核实，并把差异连同簇 ID 或片段报告给用户。 |

<span id="cli-fallback"></span>

## MCP 不可用时使用 CLI

CLI 运行同一个引擎，写出同样的 JSON。它无法回答"这段尚未写下的片段是否已存在"，所以预防时要这样做：修改前读取报告，完成修改，再运行一次，并在 `clusters[].occurrences[]` 中查找你改动过的路径。

### 不要重复工作：先用现成的结果

选择能回答你问题的最省事的方式。

**1. 编辑器已经分析过仓库。** 如果 `.deslop/cache/live-report.json` 存在，直接渲染它，不要重新分析。耗时远不到一秒：

```bash
"$DESLOP" --from-report .deslop/cache/live-report.json --no-color
```

这会写出 `.deslop/deslop-report.txt` 和 `.html`，并把 JSON 复制到 `.deslop/deslop-report.json`。该文件是编辑器最近一次完整分析的结果，不会在每次编辑后重写。用它了解当前状态，而不是检查你刚做的修改。如果无法解析，说明它来自另一个 Deslop 版本：请使用扩展自带的 `deslop`。

**2. 重新渲染已有报告。** `--from-report .deslop/deslop-report.json --nojson` 会根据上一次 CLI 运行写出新的 HTML 和文本报告，不做任何分析。

**3. 分析，同时复用缓存。** 普通运行默认复用 `.deslop/cache/fingerprints/`，只重新解析内容有变化的文件。匹配仍覆盖整个仓库，所以比重新渲染慢。只请求你需要的输出：

```bash
"$DESLOP" . --notext --nohtml --no-color
```

**4. 只检查你的修改。** 把报告限定在一个 diff 上；仍会扫描整棵树，因此复制了未改动代码的新代码也能被发现：

```bash
git diff HEAD | "$DESLOP" . --diff - --only-changed --notext --nohtml --no-color
```

不要传 `--no-incremental`。它会关闭缓存并重新解析每个文件，只用于诊断缓存问题。

### 基本选项

| 选项 | 含义 |
| --- | --- |
| `PATH` | 要分析的文件夹，默认为当前文件夹。没有子命令：`deslop duplicates` 是错误，不是工具调用。 |
| `--output <PREFIX>` | 报告输出位置，会追加 `.json`、`.txt` 和 `.html`。默认 `.deslop/deslop-report`。 |
| `--nojson` / `--notext` / `--nohtml` | 跳过某一种输出，至少保留一种。 |
| `--from-report <FILE>` | 跳过分析，渲染已有的 JSON 报告。 |
| `--diff <FILE>` / `--only-changed` | 按统一 diff 标记结果（`-` 从标准输入读取）/ 丢弃 diff 之外的结果。 |
| `--compare <PATH:START:END>` | 传两次，以 JSON 输出引擎对两个出现位置的证据，而不是报告。 |
| `--min-nodes <N>` | 参与比较的最小代码单元，以语法树节点数计，默认 30。除非用户要求，否则不要改。 |
| `--config <FILE>` | 使用指定的 `.deslop.toml`。 |
| `--fail-over <PERCENT>` / `--no-fail-over` | 为本次运行设置或清除重复率上限。 |
| `--no-color` | 纯文本终端输出。请使用它；你不是终端。 |
| `--log-to-console` | 把日志输出到 stderr 而不是文件。 |

比较两个出现位置，字节范围直接从 JSON 中复制：

```bash
"$DESLOP" . --compare "src/a.rs:120:480" --compare "src/b.rs:96:455"
```

退出码：`0` 成功，`1` 运行时错误，`2` 参数无效，`3` 超过重复率上限。退出码为 `3` 时报告仍会完整写出；请读取它们。

## 去重：先处理完全相同的克隆

按以下顺序处理各类克隆。同一类内部按报告顺序处理，即从最严重的开始。

| 顺序 | `kind` | 是什么 | 怎么做 |
| --- | --- | --- | --- |
| 1 | `identical` | 除空白外文本完全相同的副本。 | 保留一份，让所有调用方指向它。风险最低，收益最明确。 |
| 2 | `nearly_identical` | 标识符改名、字面量不同或略有改动的副本。 | 提取一个共享函数，把差异变成参数。先阅读每一处差异。 |
| 3 | `loosely_similar` | 改动较大但仍有大量复制内容。 | 阅读两个范围。只有合并后比副本更简单时才合并。 |
| 4 | `same_behavior` | 可选语义层给出的匹配。 | 当作线索。语义相似不能证明两者可以互换。 |
| — | `structural_only` | 只是形状相同，不是克隆。 | 忽略。它从不计入重复率。 |

从 CLI 报告中列出完全相同的簇：

```bash
jq -r '.clusters[] | select(.kind == "identical")
  | "\(.id)  mass \(.mass)  \(.occurrence_count)x  \(.occurrences[0].path):\(.occurrences[0].start_line)"' \
  .deslop/deslop-report.json
```

通过 MCP 时，用 `detail: "summary"` 和较大的 `limit` 调用 `duplicates`，取 `kind` 为 `identical` 的行。

对每个簇：

1. 获取全部出现位置（`cluster-by-id`，或 JSON 中的 `clusters[].occurrences[]`），并阅读每个范围内的代码。跳过标记为 `hidden` 的位置；它们是生成代码，或被有意排除在报告之外。
2. 通过 MCP 时，编辑前调用 `merge-plan` 并阅读它的结论。
3. 决定唯一副本放在哪里：放在所有调用方都能依赖的、最通用的模块中，而不是你最先打开的文件。
4. 把代码移过去并修改每个调用方。不要留下旧副本，也不要再加第三份。
5. 运行项目测试。
6. 确认簇已消失：用修改过的 `path` 调用 `duplicates`，或用 `paths` 调用 `rescan`；使用 CLI 时，按上文做一次带缓存的运行或 `--diff` 运行。

同时阅读 `literal_findings`：在代码中重复出现的同一个字面量值。把每个都替换为一个具名常量。

不要用其他手段让发现消失。不要为了隐藏重复而添加 `exclude` 或 `report_hide` 规则、调高 `--min-nodes`、修改 `[total] include` 或放宽阈值。如果某处重复是有意的，保留它并说明原因。

<span id="read-the-json"></span>

## 读取 JSON

解析 JSON，不要解析文本或 HTML。报告在 `schema_doc` 中自带字段说明；通过 MCP 时调用一次 `schema-doc` 即可。

| 字段 | 含义 |
| --- | --- |
| `metrics.duplication_percent` | 重复行数除以已分析行数，只统计 `metrics.total_includes` 中列出的类型。不是检测准确率。 |
| `metrics.per_kind[]` | 每类克隆的重复行数和百分比，以及 `included_in_total`：该类是否计入总体数字。 |
| `metrics.threshold.breached` | 是否超过配置的上限。CLI 先写报告，再以 `3` 退出。 |
| `clusters` | 克隆按质量降序排列，其后是参考信息。 |
| `cluster.id` | 稳定标识符。在所有地方都用它指代一个发现。 |
| `cluster.kind` | 上表中的克隆类型。旧报告使用 `bucket`。 |
| `cluster.mass` / `cluster.rank` | 该簇代表的重复代码量及其位置。质量表示影响大小，不是置信度，也不代表合并是否安全。参考信息质量为零，没有排名。 |
| `cluster.severity` | 编辑器中显示的诊断级别，与质量和百分比无关。 |
| `cluster.occurrences[]` | 每个副本的 `path`、`start_byte`、`end_byte`、`start_line`、`end_line` 和 `hidden`。 |
| `occurrences[].hidden` | 由报告策略隐藏，不贡献重复行数。 |
| `literal_findings` | 重复出现的字面量值，与簇分开。 |
| `tool_version` | 写出该报告的 Deslop 版本。 |

### 使用字节范围，而非行号

范围是 `[start_byte, end_byte)`：不包含结束位置，并对应分析时的文件内容。任何编辑之后，先重新分析，再使用偏移量。行号用于阅读，不用于编辑。

### 簇 ID 是稳定的

其他簇变化时排名会变，但只要被克隆的代码本身没变，ID 就不变。修改被克隆的代码可能改变 ID。

## 配置仓库

`.deslop.toml` 位于仓库根目录。[`exclude`](/zh/docs/configuration/) 在分析前移除文件。`report_hide` 让文件仍参与分析但隐藏其出现位置，因此复制生成代码的情况仍会被报告。内置规则已跳过依赖目录、构建产物和带生成器标记的文件。`[threshold] max_duplication_percent` 让 CLI 在超过上限时以 `3` 退出；它不改变克隆的判定标准。只有在用户要求时才修改配置。

## 操作规则

- 编写任何超过几行的代码单元之前，先调用 `find-similar`。有匹配就复用已有代码。
- 编辑前阅读每个出现位置的实际代码。克隆类型不能证明合并会保持行为。
- 永远不要把故障 MCP 服务器返回的失败或空结果当作"没有重复"。改用 CLI。
- 永远不要通过隐藏、排除或放宽阈值让发现消失。
- MCP 服务器和 CLI 都不能用时，告知用户，不要猜测。

测量能说明什么、不能说明什么，见[准确性透明度](/zh/docs/accuracy-transparency/)；可粘贴到项目 `AGENTS.md` 的规则块见[智能体配方](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md)。
