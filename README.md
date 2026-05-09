# Deslop

**Deslop** is a **live duplicate-code analysis server** — an LSP + MCP server that runs in your workspace and streams real-time clone signals to your AI coding agent (Claude Code, Cursor, Copilot, Continue, Codex…) and your editor *as code is being written*. The worst offenders — biggest clones, most copies, most lines spanned — surface inline before a duplicate lands in the repo, not in a CI report afterwards.

This is not a batch scanner that prints a report and exits. It is a long-running server feeding live analysis over LSP (for editors) and MCP (for agents) over the same tree-sitter engine.

## Live = Reactive

**Live means reactive.** When you change code, every Deslop surface — the live bubble, the editor decorations, the **TOP OFFENDERS** tree, the cluster webview, the status bar, the MCP query results, the agent's view of the workspace — reflects the new state **immediately**. Not on the next save. Not when the editor refreshes. Not on a polling timer. **Immediately.** As soon as the file watcher fires and the pipeline finishes its incremental pass, every reader sees the same fresh report in the same microtask. A cluster that no longer exists in the source code cannot remain on screen, in a hover, in a code lens, or in an MCP response. Stale UI is a correctness bug, not a polish issue. The CLI is the cold-cache fallback for one-shot audits — every other surface is reactive by construction. See [SPEC §[PRINCIPLES-LIVE-IS-REACTIVE]](docs/specs/principles.md#principles-live-is-reactive) and [vsix.md §[VSIX-REACTIVITY-INVARIANT]](docs/specs/vsix.md#vsix-reactivity-invariant) for the enforcing rules.

Languages: **C#**, **Rust**, **Python**. Parsing is always tree-sitter — no regex, no line diffing, no false positives from reformatting.

- **MCP server (`deslop-mcp`)** — tools an AI agent can call mid-generation: *"before I write this block, is something like it already in the repo?"* Feeds Claude / Cursor / Codex / Continue a live duplicate-awareness channel that predates the copy-paste. The keystone tool is **`find-similar`** — agents are expected to call it **before authoring new code**, not after the fact. **Prevention beats cure.** See [docs/snippets/agents-md-recipe.md](docs/snippets/agents-md-recipe.md) for the paste-ready `AGENTS.md` / `CLAUDE.md` snippet that teaches this to your own AI agents.
- **LSP server (`deslop-lsp`)** — live inline warnings and bubbles in VS Code (and any LSP-capable editor) the moment a duplicate is typed.
- **VS Code extension** — the reference LSP client; inline warnings, cluster explorer, worst-offender view.
- **CLI (`deslop`)** — the cold-cache fallback. One binary, runs on your repo, emits `.json` / `.txt` / `.html` reports. Same engine as the server; use it for CI gates, bulk audits, or one-shot investigations.

## What's actually implemented

Deslop draws on a small set of clone-detection research lines. Each one is a real file, not a future plan:

| Research line | What it implements | Code |
| --- | --- | --- |
| Tree-sitter parsing (Baxter 1998) | C# / Rust / Python AST per language | [`crates/deslop-core/src/lang/`](crates/deslop-core/src/lang/) |
| AST normalization | Type-2 collapse: `__ident__` / `__literal__` + comment/trivia drop | [`lang/shared.rs`](crates/deslop-core/src/lang/shared.rs) |
| Merkle subtree fingerprints (Chilowicz 2009) | Bottom-up BLAKE3 over normalized AST | [`fingerprint.rs`](crates/deslop-core/src/fingerprint.rs) |
| Sibling-window extension | Type-3 recall over widths 2–8 | [`sibling.rs`](crates/deslop-core/src/sibling.rs) |
| MinHash + LSH (Broder 1997 / Indyk-Motwani 1998 / SourcererCC 2016) | 128-value MinHash, 32 × 4 banding over normalized k-grams | [`tokens.rs`](crates/deslop-core/src/tokens.rs), [`lsh.rs`](crates/deslop-core/src/lsh.rs) |
| HNSW ANN over local embeddings (SSCD 2024) | `instant-distance` HNSW, deterministic seed, top-k cosine | [`embedding/pairs.rs`](crates/deslop-core/src/embedding/pairs.rs) |
| Max/sum fusion (ensemble-LLM 2025) | `clamp(structural + token_jaccard + embedding_cos, 0, 1)`, threshold 0.85 | [`pair.rs`](crates/deslop-core/src/pair.rs) |
| Worst-offenders ranking | `clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes)` | [`cluster.rs`](crates/deslop-core/src/cluster.rs) |
| Live + reactive (LSP watcher → state file → MCP) | 250 ms debounce, 2 s cap, atomic state-file rewrite, IPC socket | [`live/`](crates/deslop-core/src/live/), [`deslop-lsp/`](crates/deslop-lsp/), [`deslop-mcp/`](crates/deslop-mcp/) |

Full research → code map: [docs/specs/SPEC.md §Algorithm implementation status](docs/specs/SPEC.md#algorithm-implementation-status). Site-facing version: [Research Background](https://deslop.live/docs/research-background/).

---

## Install (preferred) — VS Code extension

The VSIX is the **one install that gives you everything**: the live bubble, the LSP server, the MCP server, and the `deslop` CLI all at once. Other IDE extensions (JetBrains, Zed, Neovim) are on the roadmap — until then, this is the headline path.

1. Grab `deslop-vscode-X.Y.Z.vsix` from the [latest GitHub release](https://github.com/Nimblesite/Deslop/releases/latest).
2. Install it:

   ```bash
   code --install-extension deslop-vscode-X.Y.Z.vsix
   ```

   Or: **Extensions panel → `…` menu → Install from VSIX…**

3. Open a `.cs` / `.rs` / `.py` file. The live bubble lights up the moment you type a duplicate. The command palette exposes **Deslop: Open Report**, **Deslop: Open Worst Cluster**, **Deslop: Jump to Next Occurrence**. The extension prepends its bundled `bin/<platform>/` to the VS Code process `PATH`, so `deslop`, `deslop-lsp`, and `deslop-mcp` are callable from the integrated terminal.

---

## Install the CLI only

If you just want the `deslop` binary on your shell `PATH` — no VS Code — use Homebrew or Scoop. Binaries are published through two repos:

- Homebrew tap: [github.com/Nimblesite/homebrew-tap](https://github.com/Nimblesite/homebrew-tap)
- Scoop bucket: [github.com/Nimblesite/scoop-bucket](https://github.com/Nimblesite/scoop-bucket)

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

Grab the archive for your platform from the [latest release](https://github.com/Nimblesite/Deslop/releases/latest) and drop the binaries on your `PATH`.

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

## Use Deslop from an AI agent (MCP)

Deslop ships an MCP server — `deslop-mcp` — that exposes live clone analysis as tools any MCP-compatible agent can call: `top-offenders`, `rescan`, `report-get`, `report-for-file`, `report-for-range`, `find-similar`, `cluster-by-id`, `list-embedding-models`, `set-embedding-model`, `session-config`.

### The binary lives inside the VSIX — point your agent at it by absolute path

The VS Code extension bundles `deslop-mcp` alongside the LSP, **and that bundled binary is the canonical one for every external MCP client too** (Claude Code, Claude Desktop, Codex, Cursor, Continue). The MCP config snippets below use an absolute path into the unpacked VSIX so the agent runs the exact binary the extension ships — version-locked to the VSIX, no PATH drift, no stale `cargo install` shadowing the release.

After `code --install-extension deslop-vscode-X.Y.Z-<target>.vsix`, the binary lives at:

| Platform | Path |
| --- | --- |
| macOS / Linux | `~/.vscode/extensions/nimblesite.deslop-vscode-<VERSION>/bin/<platform>/deslop-mcp` |
| Windows | `%USERPROFILE%\.vscode\extensions\nimblesite.deslop-vscode-<VERSION>\bin\win32-x64\deslop-mcp.exe` |

`<platform>` is one of `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, `win32-x64`. `<VERSION>` is the installed extension version — bump it whenever you update the VSIX.

> **Do not use a `cargo install`-built binary in your MCP config.** Building Deslop from source produces `target/release/deslop-mcp` for testing only — it is not a distribution channel. The repo deliberately ships no `make install-binary` target, and `make delete-path-binaries` runs before every test target to scrub leaked PATH copies. The only "PATH-resolved" form that is supported is when the user installed the CLI via `brew install nimblesite/tap/deslop` or `scoop install deslop` — those package managers version the binary lock-step with releases.

### Claude Code

```bash
claude mcp add deslop -s user -- \
  ~/.vscode/extensions/nimblesite.deslop-vscode-<VERSION>/bin/darwin-arm64/deslop-mcp \
  --root .
```

Or edit `~/.claude.json` directly:

```json
{
  "mcpServers": {
    "deslop": {
      "command": "/Users/you/.vscode/extensions/nimblesite.deslop-vscode-<VERSION>/bin/darwin-arm64/deslop-mcp",
      "args": ["--root", "."]
    }
  }
}
```

Homebrew/Scoop CLI users may substitute `"command": "deslop-mcp"` (PATH lookup) since the package manager guarantees the binary version matches the install.

### Claude Desktop

Edit `claude_desktop_config.json` (macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`, Windows: `%APPDATA%\Claude\claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "deslop": {
      "command": "/Users/you/.vscode/extensions/nimblesite.deslop-vscode-<VERSION>/bin/darwin-arm64/deslop-mcp",
      "args": ["--root", "/absolute/path/to/your/repo"]
    }
  }
}
```

Restart Claude Desktop. Use an absolute path for `--root` — Claude Desktop doesn't inherit a working directory.

### Codex

Edit `~/.codex/config.toml`:

```toml
[mcp_servers.deslop]
command = "/Users/you/.vscode/extensions/nimblesite.deslop-vscode-<VERSION>/bin/darwin-arm64/deslop-mcp"
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
