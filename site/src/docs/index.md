---
layout: layouts/docs.njk
title: Getting Started — Install Deslop and find duplicate code
description: Install Deslop and find duplicate code across nine languages. The VS Code extension bundles live editor warnings, checks for coding agents, and the CLI in one install. Homebrew, Scoop, or curl for CLI-only.
eleventyNavigation:
  key: Getting Started
  order: 1
icon: rocket_launch
---

# Getting Started

**Deslop finds duplicate code across nine languages, ranks what to remove first, and tells your coding agent when similar code already exists.** It runs on your workspace and updates as you type — Claude Code, Cursor, Copilot, Continue, Codex, and your editor all read the same live analysis.

The preferred way to install it is the **VS Code extension**. One install bundles all three surfaces: live editor warnings, the check agents call before writing code, and the CLI.

> The **JetBrains plugin** (Rider first, then IntelliJ IDEA, PyCharm, WebStorm, RustRover, CLion) is in active development. Zed and Neovim are on the roadmap. Until those ship, the VSIX is the headline install, and the Homebrew tap / Scoop bucket are the CLI-only shortcuts.

## Install (preferred) — VS Code extension

Install straight from the **VS Code Marketplace**. Nothing to download, no files to manage — pick whichever is closest to hand:

- **In VS Code:** open **Extensions** (`⇧⌘X` / `Ctrl+Shift+X`), search **Deslop**, click **Install**.
- **Command line:** `code --install-extension nimblesite.deslop-live`
- **Browser:** open the [Deslop.live Marketplace page](https://marketplace.visualstudio.com/items?itemName=nimblesite.deslop-live) and hit **Install**.

Then open a supported source file (`.cs`, `.rs`, `.py`, `.dart`, `.js`, `.mjs`, `.cjs`, `.jsx`, `.ts`, `.tsx`, `.php`, `.fs`, `.fsx`, or `.go`). The live bubble is active immediately, and the **Top Offenders** tree populates as the file watcher fires.

The extension bundles native binaries for `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, and `win32-x64` — the right one is selected for you automatically.

<figure>
  <a href="/assets/img/screenshot.webp">
    <img src="/assets/img/screenshot.webp"
         alt="The Deslop VS Code extension on a live workspace: a worst-first Top Offenders tree and a per-folder Duplication breakdown in the sidebar, a live clone warning at the cursor, and a Compare diff against the canonical occurrence."
         width="2560" height="1492" loading="lazy" decoding="async">
  </a>
  <figcaption>The extension on a live workspace — worst-first clusters and a per-folder duplication breakdown in the sidebar, a live clone warning at the cursor, and a Compare diff against the canonical copy. <a href="/docs/vscode-cluster-panel/">Full panel-by-panel walkthrough →</a></figcaption>
</figure>

> **Offline or air-gapped?** Grab the `.vsix` from the [Releases page](/releases/) or the [latest GitHub release](https://github.com/Nimblesite/Deslop/releases/latest), then install it via **Extensions panel → `…` menu → Install from VSIX…**.

## Install the CLI only (Homebrew / Scoop / curl)

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

### Linux (curl)

No Homebrew? Pull the archive straight from the latest GitHub release. The snippet resolves the newest version, verifies the published SHA-256 checksum, and installs the same three binaries the Homebrew formula ships (`deslop`, `deslop-lsp`, `deslop-mcp`):

```bash
tag=$(curl -fsSLI -o /dev/null -w '%{url_effective}' https://github.com/Nimblesite/Deslop/releases/latest)
tag=${tag##*/}        # e.g. v1.2.3
version=${tag#v}      # e.g. 1.2.3
case "$(uname -m)" in
  x86_64)  platform=linux-x64 ;;
  aarch64) platform=linux-arm64 ;;
  *) echo "unsupported architecture: $(uname -m)"; exit 1 ;;
esac
archive="deslop-${version}-${platform}.tar.gz"
curl -fsSLO "https://github.com/Nimblesite/Deslop/releases/download/${tag}/${archive}"
curl -fsSLO "https://github.com/Nimblesite/Deslop/releases/download/${tag}/${archive}.sha256"
sha256sum -c "${archive}.sha256"
tar -xzf "$archive"
sudo install -m 755 "deslop-${version}-${platform}"/deslop{,-lsp,-mcp} /usr/local/bin/
deslop --version
```

Prefer a user-local install? Swap the `install` line for `install -m 755 "deslop-${version}-${platform}"/deslop{,-lsp,-mcp} ~/.local/bin/` (no `sudo`) and make sure `~/.local/bin` is on your `PATH`.

To pin a specific version instead of the latest, skip the first two lines and set `tag=vX.Y.Z` yourself.

### Direct download

Grab the per-platform archive from the [Releases page](/releases/) or the [latest GitHub release](https://github.com/Nimblesite/Deslop/releases/latest), then drop the binaries on your `PATH`.

## Run the CLI

```bash
deslop .
```

That scans the current directory, writes three reports, and prints the top clusters to your terminal. Everything Deslop writes goes into one `.deslop/` directory at the root of the scanned project — add `.deslop/` to your `.gitignore` and you are done:

```
.deslop/
  deslop-report.json   # canonical, agent-consumable
  deslop-report.txt    # line-oriented plain text
  deslop-report.html   # standalone, human-readable
  logs/                # timestamped run logs
  cache/               # fingerprints and embeddings; safe to delete
```

`--output <prefix>` sends the reports (and their logs) somewhere else instead.

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

`--fail-over` overrides the config key; `--fail-over 0` fails on any duplication; `--no-fail-over` clears the gate for a single local run. The full [exit-code table](/docs/configuration/#exit-codes) is in the configuration reference, and the [GitHub Action](/docs/github-action/) wraps the same gate for CI.

## What to do next

1. Read [How It Works](/docs/how-it-works/) to understand the ranking formula and the live pipeline.
2. Read [AI Agents](/docs/ai-integration/) to wire `deslop-mcp` into Claude Code, Cursor, Continue, or Codex — then point the agent itself at [For AI](/docs/for-ai/), the operating manual written for the machine, including what to do when MCP is unavailable.
3. Read [VS Code](/docs/vscode-cluster-panel/) when you need the meaning of a panel label, score, or action.
4. Read [Configuration and Reports](/docs/configuration/) for every `.deslop.toml` key, every CLI flag, the three report formats, and the exit codes.
5. Check [Releases](/releases/) for the current VSIX, CLI archives, checksums, and changelog links.
