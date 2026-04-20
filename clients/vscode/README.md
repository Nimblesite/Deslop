# CodeDedup for VS Code

**The first clone detector that tells you you're duplicating code as you type.**

Every other tool — PMD CPD, jscpd, SonarLint, JetBrains inspections — flags duplication on CI, on save, or in a panel you have to remember to open. CodeDedup surfaces duplicates **inline, next to your cursor, 250 ms after the last keystroke**. No save, no push, no context switch.

## Features

- **Live duplication bubble.** The moment you type code that matches an existing cluster, a severity-coloured verdict (`DUPLICATE`, `NEAR-MISS`, `SEMANTIC MATCH`) appears at the end of the line, with a signal strip showing how structural vs. token vs. embedding similarity scored.
- **Worst-first activity-bar view.** The Duplicate Clusters panel always has cluster `#1` — the single highest-impact offender in the whole workspace — one click away. No drilling.
- **Ollama-powered semantic matches.** Plug in any local embedding model (`nomic-embed-code`, `nomic-embed-text`, `unixcoder`, your own) via the built-in picker. Stays loopback-only.
- **Live report webview.** Sorted worst-first, filterable by language / severity / path, refreshes as you type via Preact Signals — no stale pixels, ever.
- **Bundled LSP + MCP binaries.** Every platform (`darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, `win32-x64`) ships offline-ready. No post-install downloads. The MCP server auto-registers with Claude Code / Copilot Chat so your AI agents see the same duplicates you do.
- **Falls back to your CLI install.** If you already have `codededup` on `PATH` via Homebrew tap or Scoop bucket at a matching version, the extension uses it — one binary, one cache, one truth.

## Design

Built on **the Kinetic Manuscript** — a high-density, editorial aesthetic inspired by technical whitepapers. Inter for UI, JetBrains Mono for data, crimson `#B3261E` as a surgical accent reserved for the worst offenders. No 1px borders, no bubbly radii, no consumer-SaaS greens. Professionalism comes from transparency.

## Install

- Marketplace: search `CodeDedup` in VS Code's Extensions view.
- OpenVSX: also published under the same id.
- CLI too: `brew install codededup/tap/codededup` or `scoop install codededup`. The extension will pick up the PATH install automatically when its version matches.

## Settings

See `CodeDedup` in the Settings UI. Key knobs: `codededup.embedding.model`, `codededup.minNodes`, `codededup.liveBubble.mode` (inline / ghost).

## License

MIT.
