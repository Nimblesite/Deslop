---
layout: layouts/docs.njk
title: Getting Started — Install Deslop's live LSP + MCP server
description: Install Deslop — the live LSP + MCP duplicate-code server for AI agents. The VS Code VSIX bundles LSP, MCP, and CLI in one install. Homebrew and Scoop for CLI-only.
eleventyNavigation:
  key: Getting Started
  order: 1
icon: rocket_launch
---

# Getting Started

Deslop is a **live duplicate-code analysis server** — LSP + MCP, running in your workspace, streaming real-time clone signals to Claude Code, Cursor, Copilot, Continue, Codex, and your editor *as code is being written*. The preferred way to install it is the **VS Code extension** — the extension bundles the LSP server, the MCP server, **and** the CLI in one install.

> The **JetBrains plugin** (Rider first, then IntelliJ IDEA, PyCharm, WebStorm, RustRover, CLion) is in active development. Zed and Neovim are on the roadmap. Until those ship, the VSIX is the headline install, and the Homebrew tap / Scoop bucket are the CLI-only shortcuts.

## Install (preferred) — VS Code extension

Install straight from the **VS Code Marketplace**. Nothing to download, no files to manage — pick whichever is closest to hand:

- **In VS Code:** open **Extensions** (`⇧⌘X` / `Ctrl+Shift+X`), search **Deslop**, click **Install**.
- **Command line:** `code --install-extension nimblesite.deslop-live`
- **Browser:** open the [Deslop.live Marketplace page](https://marketplace.visualstudio.com/items?itemName=nimblesite.deslop-live) and hit **Install**.

Then open a `.cs` / `.rs` / `.py` / `.dart` file. The live bubble is active immediately, and the **Top Offenders** tree populates as the file watcher fires.

The extension bundles native binaries for `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, and `win32-x64` — the right one is selected for you automatically.

> **Offline or air-gapped?** Grab the `.vsix` from the [latest GitHub release](https://github.com/Nimblesite/Deslop/releases/latest) and install it via **Extensions panel → `…` menu → Install from VSIX…**.

## Install the CLI only (Homebrew / Scoop)

### macOS / Linux (Homebrew)

```bash
brew install nimblesite/tap/deslop
deslop --version
```

Tap source: [github.com/Nimblesite/homebrew-tap](https://github.com/Nimblesite/homebrew-tap).

### Windows (Scoop)

```powershell
scoop bucket add nimblesite https://github.com/Nimblesite/scoop-bucket
scoop install deslop
deslop --version
```

Bucket source: [github.com/Nimblesite/scoop-bucket](https://github.com/Nimblesite/scoop-bucket).

### Direct download

Grab the per-platform archive from the [latest GitHub release](https://github.com/Nimblesite/Deslop/releases/latest) and drop the binaries on your `PATH`.

## Run the CLI

```bash
deslop .
```

That scans the current directory, writes three reports, and prints the top clusters to your terminal:

```
deslop-report.json   # canonical, agent-consumable
deslop-report.txt    # line-oriented plain text
deslop-report.html   # standalone, human-readable
```

## Tune the threshold

Default minimum AST node count is chosen so trivial getters do not pollute the top of the report. Override per-run:

```bash
deslop . --min-nodes 20
```

Raise it for large codebases where you only want major duplication. Lower it when hunting micro-patterns.

## Enable semantic detection — same behavior, different code (Type-4)

Structural and token passes are deterministic and run without network. Same-behavior matches (Type-4) — same behaviour, different syntax — require embeddings. Embeddings are **off by default**:

```bash
deslop . --embeddings auto
```

`auto` probes the local Ollama provider and falls back with a warning if it's unreachable. Use `--embeddings required` to hard-fail when the provider can't be contacted. The default model is `nomic-embed-text`; any Ollama embedding model selectable via `--embedding-model`.

See [How It Works](/docs/how-it-works/) for the fusion math.

## Exclude noise

Generated code and build outputs are filtered by default. Add `.deslop.toml` only for project-specific dependencies, migrations, or training-set code:

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

`exclude` skips parsing entirely. `report_hide` parses but omits from the final ranking — useful for training-set code you still want in the cache.

## Gate CI on a duplication threshold

By default `deslop` exits `0` no matter how much duplication it finds — it reports, it does not judge, so it never breaks a build you did not ask it to gate. Opt into a gate and it exits `3` (failing the build) when repo-wide duplication crosses your ceiling. Pass a flag for a one-off, or commit the ceiling so local runs, CI, and agents all share one number:

```bash
deslop . --fail-over 5.0          # exit 3 if more than 5% of analysed LOC is duplicated
```

```toml
# .deslop.toml
[threshold]
max_duplication_percent = 5.0
```

`--fail-over` overrides the config key; `--fail-over 0` fails on any duplication; `--no-fail-over` clears the gate for a single local run. The full exit-code table lives in [Output Formats](/docs/output-formats/#exit-codes), and a ready-to-paste GitHub Actions job is in [For AI](/docs/for-ai/#run-in-ci).

## What to do next

1. Read [How It Works](/docs/how-it-works/) to understand the ranking formula and the live pipeline.
2. Read [AI Integration](/docs/ai-integration/) to wire `deslop-mcp` into Claude Code, Claude Desktop, Cursor, Continue, or Codex.
3. Read [Output Formats](/docs/output-formats/) before parsing the JSON yourself.
4. Read [VS Code Cluster Panel](/docs/vscode-cluster-panel/) when you need the meaning of a panel label, score, or action.
5. Read [For AI](/docs/for-ai/) if you are (or are configuring) a coding agent — the operating manual for `.deslop.toml`, CI gating, and parsing the JSON report.
