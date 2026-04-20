# Deslop

**Deslop** is a **live duplicate-code analysis server** — an LSP + MCP server that runs in your workspace and streams real-time clone signals to your AI coding agent (Claude Code, Cursor, Copilot, Continue, Codex…) and your editor *as code is being written*. The worst offenders — biggest clones, most copies, most lines spanned — surface inline before a duplicate lands in the repo, not in a CI report afterwards.

This is not a batch scanner that prints a report and exits. It is a long-running server feeding live analysis over LSP (for editors) and MCP (for agents) over the same tree-sitter engine.

Languages: **C#**, **Rust**, **Python**. Parsing is always tree-sitter — no regex, no line diffing, no false positives from reformatting.

- **MCP server (`deslop-mcp`)** — tools an AI agent can call mid-generation: *"before I write this block, is something like it already in the repo?"* Feeds Claude / Cursor / Codex / Continue a live duplicate-awareness channel that predates the copy-paste.
- **LSP server (`deslop-lsp`)** — live inline warnings and bubbles in VS Code (and any LSP-capable editor) the moment a duplicate is typed.
- **VS Code extension** — the reference LSP client; inline warnings, cluster explorer, worst-offender view.
- **CLI (`deslop`)** — the cold-cache fallback. One binary, runs on your repo, emits `.json` / `.txt` / `.html` reports. Same engine as the server; use it for CI gates, bulk audits, or one-shot investigations.

---

## Install the CLI

### macOS / Linux (Homebrew)

```bash
brew install nimblesite/tap/deslop
deslop --version
```

### Windows (Scoop)

```powershell
scoop bucket add nimblesite https://github.com/Nimblesite/scoop-bucket
scoop install deslop
deslop --version
```

### Direct download

Grab the binary for your platform from the [latest release](https://github.com/Nimblesite/Deslop/releases/latest) and drop it on your `PATH`.

## Use the CLI

Scan the current directory:

```bash
deslop
```

Scan a specific repo and write reports to a chosen prefix:

```bash
deslop ~/code/my-repo --output ~/reports/my-repo
# → my-repo.json, my-repo.txt, my-repo.html
```

Fail CI when duplication exceeds a percentage:

```bash
deslop --fail-over 5.0
```

Re-render a previous JSON report without re-analysing:

```bash
deslop --from-report deslop-report.json
```

Full flag reference: `deslop --help`.

---

## Install the VS Code extension

Download `deslop-vscode-X.Y.Z.vsix` from the [latest release](https://github.com/Nimblesite/Deslop/releases/latest) and install it:

```bash
code --install-extension deslop-vscode-X.Y.Z.vsix
```

Or: **Extensions panel → `…` menu → Install from VSIX…**

The extension activates automatically on `.cs`, `.rs`, and `.py` files. Duplicated blocks are highlighted live as you edit. Use the command palette (**Deslop: Open Report**, **Deslop: Open Worst Cluster**, **Deslop: Jump to Next Occurrence**) to navigate clusters.

---

## Use Deslop from an AI agent (MCP)

Deslop ships an MCP server — `deslop-mcp` — that exposes live clone analysis as tools any MCP-compatible agent can call: `report-get`, `report-for-file`, `report-for-range`, `find-similar`, `cluster-by-id`, `list-embedding-models`, `set-embedding-model`, `session-config`.

Install it the same way as the CLI — `brew install nimblesite/tap/deslop` or `scoop install deslop` ships both binaries.

### Claude Code

```bash
claude mcp add deslop -- deslop-mcp --root .
```

Or edit `~/.claude.json` / `.mcp.json` directly:

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

### Claude Desktop

Edit `claude_desktop_config.json` (macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`, Windows: `%APPDATA%\Claude\claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "deslop": {
      "command": "deslop-mcp",
      "args": ["--root", "/absolute/path/to/your/repo"]
    }
  }
}
```

Restart Claude Desktop. Use an absolute path — Claude Desktop doesn't inherit a working directory.

### Codex

Edit `~/.codex/config.toml`:

```toml
[mcp_servers.deslop]
command = "deslop-mcp"
args    = ["--root", "."]
```

### Useful flags

```
--min-nodes 30          # raise to cut noise on small clones
--incremental           # cache parsed ASTs under .deslop-cache/
--embeddings auto       # add semantic-similarity pass (needs Ollama)
--embedding-model nomic-embed-text
```

---

## License

Dual-licensed under [MIT](LICENSE) or Apache-2.0, at your option.

---

<details>
<summary>Building from source</summary>

Requires Rust 1.80+ and GNU Make.

```bash
make build   # release binary at target/release/deslop
make test    # fail-fast tests + coverage gate
make ci      # lint + test + build
```

See [CLAUDE.md](CLAUDE.md) for contributor rules and [docs/specs/SPEC.md](docs/specs/SPEC.md) for the design spec.

</details>
