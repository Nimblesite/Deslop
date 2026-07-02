---
layout: layouts/docs.njk
title: 快速开始 — 安装 Deslop 的实时 LSP + MCP 服务器
description: 安装 Deslop —— 面向 AI 智能体的实时 LSP + MCP 重复代码服务器。VS Code VSIX 一次安装即捆绑 LSP、MCP 与 CLI。仅需 CLI 可使用 Homebrew 和 Scoop。
eleventyNavigation:
  key: 快速开始
  order: 1
icon: rocket_launch
lang: zh
---

# 快速开始

Deslop 是一个**实时重复代码分析服务器** —— LSP + MCP，运行于你的工作区，在*代码正被编写时*将实时的克隆信号流式传输到 Claude Code、Cursor、Copilot、Continue、Codex 以及你的编辑器。安装它的首选方式是 **VS Code 扩展** —— 该扩展一次安装即捆绑 LSP 服务器、MCP 服务器**以及** CLI。

> **JetBrains 插件**（先支持 Rider，随后是 IntelliJ IDEA、PyCharm、WebStorm、RustRover、CLion）正在积极开发中。Zed 与 Neovim 已列入路线图。在它们发布之前，VSIX 是首要安装方式，而 Homebrew tap / Scoop bucket 则是仅需 CLI 时的快捷方式。

## 安装（首选） —— VS Code 扩展

直接从 **VS Code Marketplace** 安装。无需下载，无需管理文件 —— 挑选离你最近的方式即可：

- **在 VS Code 中：** 打开**扩展**（`⇧⌘X` / `Ctrl+Shift+X`），搜索 **Deslop**，点击**安装**。
- **命令行：** `code --install-extension nimblesite.deslop-live`
- **浏览器：** 打开 [Deslop.live Marketplace 页面](https://marketplace.visualstudio.com/items?itemName=nimblesite.deslop-live) 并点击**安装**。

随后打开一个受支持的源文件（`.cs`、`.rs`、`.py`、`.dart`、`.js`、`.mjs`、`.cjs`、`.jsx`、`.ts` 或 `.tsx`）。实时气泡会立即生效，并且随着文件监视器触发，**Top Offenders** 树状视图会随之填充。

该扩展捆绑了面向 `darwin-arm64`、`darwin-x64`、`linux-x64`、`linux-arm64` 和 `win32-x64` 的原生二进制文件 —— 系统会自动为你选择正确的那一个。

<figure>
  <a href="/assets/img/screenshot.webp">
    <img src="/assets/img/screenshot.webp"
         alt="Deslop VS Code 扩展正在分析实时工作区：侧边栏中以最严重优先排序的 Top Offenders 树与按目录划分的 Duplication 占比，编辑器中光标处的实时克隆警告，以及与规范出现位置的 Compare 差异对比。"
         width="2560" height="1492" loading="lazy" decoding="async">
  </a>
  <figcaption>实时工作区中的扩展——侧边栏里以最严重优先排序的克隆簇与按目录划分的重复占比、光标处的实时克隆警告，以及与规范副本的 Compare 差异对比。<a href="/zh/docs/vscode-cluster-panel/">逐面板完整解读 →</a></figcaption>
</figure>

> **离线或隔离网络环境？** 从[发布页](/zh/releases/)或[最新的 GitHub release](https://github.com/Nimblesite/Deslop/releases/latest) 获取 `.vsix`，并通过**扩展面板 → `…` 菜单 → 从 VSIX 安装…**进行安装。

## 仅安装 CLI（Homebrew / Scoop）

### macOS / Linux（Homebrew）

```bash
brew install nimblesite/tap/deslop
deslop --version
```

Tap 源：[github.com/Nimblesite/homebrew-tap](https://github.com/Nimblesite/homebrew-tap)。

### Windows（Scoop）

```powershell
scoop bucket add nimblesite https://github.com/Nimblesite/scoop-bucket
scoop install deslop
deslop --version
```

Bucket 源：[github.com/Nimblesite/scoop-bucket](https://github.com/Nimblesite/scoop-bucket)。

### 直接下载

从[发布页](/zh/releases/)或[最新的 GitHub release](https://github.com/Nimblesite/Deslop/releases/latest) 获取对应平台的归档文件，并将二进制文件放入你的 `PATH`。

## 运行 CLI

```bash
deslop .
```

这会扫描当前目录、写入三份报告，并将最严重的簇打印到你的终端：

```
deslop-report.json   # canonical, agent-consumable
deslop-report.txt    # line-oriented plain text
deslop-report.html   # standalone, human-readable
```

## 调整阈值

默认的最小 AST 节点数量经过精心选择，以避免琐碎的 getter 污染报告顶部。可按运行逐次覆盖：

```bash
deslop . --min-nodes 20
```

对于只想关注重大重复的大型代码库，可以调高它。在追查微观模式时，则调低它。

## 启用语义检测 —— 行为相同、代码不同（Type-4）

结构与 token 通道是确定性的，无需联网即可运行。行为相同的匹配（Type-4） —— 行为相同、语法不同 —— 需要嵌入（向量嵌入）。嵌入**默认关闭**：

```bash
deslop . --embeddings auto
```

`auto` 会探测本地 Ollama 提供方，若无法连通则发出警告并回退。使用 `--embeddings required` 可在无法联系到提供方时直接硬性失败。默认模型为 `nomic-embed-text`；任何 Ollama 嵌入模型均可通过 `--embedding-model` 选择。

参见 [工作原理](/zh/docs/how-it-works/) 了解信号融合的数学原理。

## 排除噪声

生成的代码与构建产物默认会被过滤。仅为项目特定的依赖、迁移或训练集代码添加 `.deslop.toml`：

```toml
[defaults]
exclude = [
  "**/bin/**",
  "**/obj/**",
  "**/node_modules/**",
  "**/target/**",
  "**/*.Designer.cs",
]

report_hide = [
  "**/*.g.cs",
]
```

`exclude` 完全跳过解析。`report_hide` 会解析但从最终排名中省略 —— 对于你仍希望保留在缓存中的训练集代码很有用。

<span id="gate-ci-on-a-duplication-threshold"></span>

## 以重复阈值对 CI 设置门禁

默认情况下，无论发现多少重复，`deslop` 都会以 `0` 退出 —— 它只报告，不评判，因此绝不会破坏一个你并未要求它把关的构建。一旦选择启用门禁，当全仓库范围的重复超过你的上限时，它会以 `3` 退出（使构建失败）。可以传入一个标志用于一次性运行，或将上限提交入库，让本地运行、CI 与智能体共享同一个数值：

```bash
deslop . --fail-over 5.0          # exit 3 if more than 5% of analysed LOC is duplicated
```

```toml
# .deslop.toml
[threshold]
max_duplication_percent = 5.0
```

`--fail-over` 会覆盖配置键；`--fail-over 0` 在任何重复时都会失败；`--no-fail-over` 会为单次本地运行清除门禁。完整的退出码对照表见 [输出格式](/zh/docs/output-formats/#exit-codes)，一个可直接粘贴的 GitHub Actions 作业见 [面向 AI](/zh/docs/for-ai/#run-in-ci)。

## 下一步做什么

1. 阅读 [工作原理](/zh/docs/how-it-works/)，理解排名公式与实时流水线。
2. 阅读 [AI 集成](/zh/docs/ai-integration/)，将 `deslop-mcp` 接入 Claude Code、Claude Desktop、Cursor、Continue 或 Codex。
3. 在自行解析 JSON 之前，先阅读 [输出格式](/zh/docs/output-formats/)。
4. 当你需要了解某个面板标签、评分或操作的含义时，阅读 [VS Code 簇面板](/zh/docs/vscode-cluster-panel/)。
5. 如果你是（或正在配置）一个编码智能体，阅读 [面向 AI](/zh/docs/for-ai/) —— 这是关于 `.deslop.toml`、CI 门禁与解析 JSON 报告的操作手册。
6. 查看[发布](/zh/releases/)以获取当前 VSIX、CLI 归档、校验和与变更日志链接。
