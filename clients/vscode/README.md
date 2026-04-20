# Deslop for VS Code

**The first clone tool that flags duplicated code as you type — and is built to fix it, not just report it.**

Every other tool — PMD CPD, jscpd, SonarLint, JetBrains inspections — flags duplication on CI, on save, or in a panel you have to remember to open. Deslop surfaces duplicates **inline, next to your cursor, 250 ms after the last keystroke**. No save, no push, no context switch. Detection and ranking ship today; AI-assisted and mechanical deduplication actions are on the roadmap, so the same engine that spots the clone will soon help you collapse it.

## Features

- **Live duplication bubble.** The moment you type code that matches an existing cluster, a severity-coloured label — **Identical code**, **Nearly identical code**, **Loosely similar code**, or **Same behavior, different code** (AI match) — appears at the end of the line, with a signal strip showing how structural vs. token vs. embedding similarity scored.
- **Worst-first activity-bar view.** The Duplicate Clusters panel always has cluster `#1` — the single highest-impact offender in the whole workspace — one click away. No drilling.
- **Ollama-powered semantic matches.** Plug in any local embedding model (`nomic-embed-code`, `nomic-embed-text`, `unixcoder`, your own) via the built-in picker. Stays loopback-only.
- **Live report webview.** Sorted worst-first, filterable by language / severity / path, refreshes as you type via Preact Signals — no stale pixels, ever.
- **Bundled LSP + MCP binaries.** Every platform (`darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, `win32-x64`) ships offline-ready. No post-install downloads. The MCP server auto-registers with Claude Code / Copilot Chat so your AI agents see the same duplicates you do.
- **Falls back to your CLI install.** If you already have `deslop` on `PATH` via Homebrew tap or Scoop bucket at a matching version, the extension uses it — one binary, one cache, one truth.

## Design

Built on **the Kinetic Manuscript** — a high-density, editorial aesthetic inspired by technical whitepapers. Inter for UI, JetBrains Mono for data, crimson `#B3261E` as a surgical accent reserved for the worst offenders. No 1px borders, no bubbly radii, no consumer-SaaS greens. Professionalism comes from transparency.

## Install

- Download `deslop-vscode-X.Y.Z.vsix` from the [latest GitHub release](https://github.com/Nimblesite/Deslop/releases/latest), then `code --install-extension deslop-vscode-X.Y.Z.vsix`.
- CLI too: `brew install nimblesite/tap/deslop` or `scoop install deslop`. The extension will pick up the PATH install automatically when its version matches.

## Settings

See `Deslop` in the Settings UI. Key knobs: `deslop.embedding.model`, `deslop.minNodes`, `deslop.liveBubble.mode` (inline / ghost).

## License

MIT.
