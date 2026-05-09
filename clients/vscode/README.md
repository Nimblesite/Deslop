# Deslop for VS Code

**The reference client for the Deslop live duplicate-code analysis server.** A long-running LSP + MCP process sits in your workspace and feeds duplicate-code signals — live, on every keystroke — to your editor *and* to whichever AI coding agent is driving it (Claude Code, Cursor, Copilot, Continue, Codex).

Every other clone tool — PMD CPD, jscpd, SonarLint, JetBrains inspections — flags duplication on CI, on save, or in a panel you have to remember to open. Deslop surfaces duplicates **inline, next to your cursor, 250 ms after the last keystroke**, and exposes the same live analysis to the agent over MCP so it can check *before* it copy-pastes. No save, no push, no context switch, no batch report.

## Features

- **Live duplication bubble.** The moment you type code that matches an existing cluster, a severity-coloured label — **Identical code**, **Nearly identical code**, **Loosely similar code**, or **Same behavior, different code** (AI match) — appears at the end of the line, with a signal strip showing how structural vs. token vs. embedding similarity scored.
- **Worst-first activity-bar view.** The Duplicate Clusters panel always has cluster `#1` — the single highest-impact offender in the whole workspace — one click away. No drilling.
- **Ollama-powered semantic matches.** Plug in any local embedding model (`nomic-embed-code`, `nomic-embed-text`, `unixcoder`, your own) via the built-in picker. Stays loopback-only.
- **Live report webview.** Sorted worst-first, filterable by language / severity / path, refreshes as you type via Preact Signals — no stale pixels, ever.
- **Bundled LSP + MCP servers.** Every platform ships the `deslop-lsp` and `deslop-mcp` binaries offline-ready. No post-install downloads. The MCP server auto-registers with Copilot Chat (and any other VS Code-hosted MCP client) so your AI agents inside VS Code consult the same live analysis you see — the duplicate is visible to the agent *before* it generates the copy-paste.
- **Uses the installed extension bundle.** The VSIX runs the binaries unpacked under its own `bin/<platform>/` folder. No post-install copying and no PATH lookup are required.

## Wire `deslop-mcp` into external MCP clients

External MCP clients that run *outside* VS Code's process — Claude Code (CLI), Claude Desktop, Codex, Cursor, Continue — do not inherit VS Code's bundled `PATH`, so they cannot auto-discover the `deslop-mcp` binary. Point them at the **VSIX-bundled binary by absolute path** so the agent runs the exact binary this extension ships.

After installing this VSIX, the binary lives at:

```
~/.vscode/extensions/nimblesite.deslop-vscode-<VERSION>/bin/<platform>/deslop-mcp
```

Where `<platform>` is `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, or `win32-x64`, and `<VERSION>` matches the installed extension. Bump `<VERSION>` whenever you update the extension.

Example — Claude Code:

```bash
claude mcp add deslop -s user -- \
  ~/.vscode/extensions/nimblesite.deslop-vscode-0.1.0/bin/darwin-arm64/deslop-mcp \
  --root .
```

Example — Codex (`~/.codex/config.toml`):

```toml
[mcp_servers.deslop]
command = "/Users/you/.vscode/extensions/nimblesite.deslop-vscode-0.1.0/bin/darwin-arm64/deslop-mcp"
args    = ["--root", "."]
```

The full set of client wiring snippets — including Claude Desktop and the rule against pointing MCP clients at `cargo install` / `target/release` binaries — lives in the [root README](https://github.com/Nimblesite/Deslop#use-deslop-from-an-ai-agent-mcp).

## Design

Built on **the Kinetic Manuscript** — a high-density, editorial aesthetic inspired by technical whitepapers. Inter for UI, JetBrains Mono for data, crimson `#B3261E` as a surgical accent reserved for the worst offenders. No 1px borders, no bubbly radii, no consumer-SaaS greens. Professionalism comes from transparency.

## Install

- Download the platform-specific `deslop-vscode-X.Y.Z-<target>.vsix` from the [latest GitHub release](https://github.com/Nimblesite/Deslop/releases/latest), then run `code --install-extension deslop-vscode-X.Y.Z-<target>.vsix`.
- CLI too: `brew install nimblesite/tap/deslop` or `scoop install deslop`.

## Settings

See `Deslop` in the Settings UI. Key knobs: `deslop.embedding.model`, `deslop.minNodes`, `deslop.liveBubble.mode` (inline / ghost).

## License

MIT.
