---
layout: layouts/docs.njk
title: Getting Started
eleventyNavigation:
  key: Getting Started
  order: 1
icon: rocket_launch
---

# Getting Started

Deslop is a **live duplicate-code analysis server**. The preferred way to install it is the **VS Code extension** — the VSIX bundles the LSP server, the MCP server, and the CLI all at once, so you get the live bubble in your editor, the MCP channel for your AI agent, and the CLI on your `PATH`, from one download.

> More IDE extensions (JetBrains, Zed, Neovim) are on the roadmap. Until then, the VSIX is the headline install, and the Homebrew tap / Scoop bucket are the CLI-only shortcuts.

## Install (preferred) — VS Code extension

1. Grab `deslop-vscode-X.Y.Z.vsix` from the [latest GitHub release](https://github.com/Nimblesite/Deslop/releases/latest).
2. `code --install-extension deslop-vscode-X.Y.Z.vsix` — or use **Extensions panel → `…` menu → Install from VSIX…**.
3. Open a `.cs` / `.rs` / `.py` file. The live bubble is active; `deslop`, `deslop-lsp`, and `deslop-mcp` are on your VS Code integrated-terminal `PATH` for the session.

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

## Enable semantic detection (same behavior, different code)

Structural and token passes are deterministic and run without network. Same-behavior matches — same behaviour, different syntax — require embeddings:

```bash
deslop . --embeddings required
```

Deslop will use a local Ollama model if one is configured, or degrade gracefully when embeddings are unavailable. See [How It Works](/docs/how-it-works/) for the fusion math.

## Exclude noise

Generated code and build outputs are filtered by default. Add `.deslop.toml` only for project-specific dependencies, migrations, or training-set code:

```toml
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

## What to do next

1. Read [How It Works](/docs/how-it-works/) to understand the ranking formula.
2. Read [AI Integration](/docs/ai-integration/) if you're wiring Deslop into an agent.
3. Read [Output Formats](/docs/output-formats/) before parsing the JSON yourself.
4. Read [VS Code Cluster Panel](/docs/vscode-cluster-panel/) when you need the meaning of a panel label, score, or action.
