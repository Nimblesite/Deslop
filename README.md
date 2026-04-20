# Deslop

**Deslop** (a.k.a. Deslop Live) finds, ranks, and helps you fix duplicated code *as you type*. It surfaces the worst offenders first — biggest clones, most copies, most lines spanned — so you stop chasing trivia and start removing real duplication. Today that's inline warnings and worst-first reports; next comes AI-assisted and mechanical deduplication so the fix is a keystroke away.

Languages: **C#**, **Rust**, **Python**. Parsing is always tree-sitter — no regex, no line diffing, no false positives from reformatting.

- **CLI** — one binary, runs on your repo, emits `.json` / `.txt` / `.html` reports and drives downstream fix tooling.
- **VS Code extension** — inline warnings the moment you paste a duplicate, with refactor actions on the roadmap.

---

## Install the CLI

### macOS / Linux (Homebrew)

```bash
brew install melbournedeveloper/tap/deslop
deslop --version
```

### Windows (Scoop)

```powershell
scoop bucket add melbournedeveloper https://github.com/MelbourneDeveloper/scoop-bucket
scoop install deslop
deslop --version
```

### Direct download

Grab the binary for your platform from the [latest release](https://github.com/MelbourneDeveloper/CodeDedup/releases/latest) and drop it on your `PATH`.

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
deslop --from-report codededup-report.json
```

Full flag reference: `deslop --help`.

---

## Install the VS Code extension

### From the Marketplace

Search for **Deslop** in the Extensions panel, or:

```bash
code --install-extension deslop
```

### From a `.vsix`

Download `deslop.vsix` from the [latest release](https://github.com/MelbourneDeveloper/CodeDedup/releases/latest) and install it:

```bash
code --install-extension deslop.vsix
```

Or: **Extensions panel → `…` menu → Install from VSIX…**

The extension activates automatically on `.cs`, `.rs`, and `.py` files. Duplicated blocks are highlighted live as you edit. Use the command palette (**Deslop: Open Report**, **Deslop: Open Worst Cluster**, **Deslop: Jump to Next Occurrence**) to navigate clusters.

---

## Use Deslop from an AI agent (MCP)

Deslop ships an MCP server — `deslop-mcp` — that exposes live clone analysis as tools any MCP-compatible agent can call: `report-get`, `report-for-file`, `report-for-range`, `find-similar`, `cluster-by-id`, `list-embedding-models`, `set-embedding-model`, `session-config`.

Install it the same way as the CLI — `brew install melbournedeveloper/tap/deslop` or `scoop install deslop` ships both binaries.

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
--incremental           # cache parsed ASTs under .codededup-cache/
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
make build   # release binary at target/release/codededup
make test    # fail-fast tests + coverage gate
make ci      # lint + test + build
```

See [CLAUDE.md](CLAUDE.md) for contributor rules and [docs/specs/SPEC.md](docs/specs/SPEC.md) for the design spec.

</details>
