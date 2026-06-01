# Deslop

**The live duplicate-code server for AI coding agents.** Deslop is an **LSP + MCP server** that runs in your workspace and streams **real-time** clone signals to Claude Code, Cursor, Copilot, Continue, Codex — and your editor — **as code is being written**. Worst offenders surface inline *before* the duplicate lands, not in a CI report three commits later.

This is not a batch scanner. It is a long-running server with a file watcher, a debouncer, an incremental cache, and atomic state writes. Every editor surface and every MCP tool reads the same live state on every keystroke.

[VS Code Marketplace install](https://marketplace.visualstudio.com/items?itemName=nimblesite.deslop-live) · [Latest release](https://github.com/Nimblesite/Deslop/releases/latest) · [Docs](https://deslop.live/docs/) · [AGENTS.md recipe](docs/snippets/agents-md-recipe.md)

---

## The MCP edge — prevent the copy-paste, don't audit it

The **MCP server** is the headline. It exposes the running analysis to any MCP-capable agent (Claude Code, Claude Desktop, Cursor, Continue, Codex) and lets the agent ask, *mid-generation*, **"is something like this already in the repo?"** — before a single byte of duplicated code is written.

The keystone tool is **[`find-similar`](docs/snippets/agents-md-recipe.md)**. The agent calls it before authoring a new function, helper, or test setup. If the proposed pattern already exists with high similarity, the agent reuses the canonical implementation instead of authoring a fresh copy. **Prevention beats cure.** Post-hoc deduplication is what every static analyzer already does — Deslop's edge is being live in the agent's inner loop.

Twelve MCP tools ship today:

| Tool | Purpose |
| --- | --- |
| `find-similar` | **Prevention.** Match a proposed snippet against the live corpus before the agent writes it. |
| `top-offenders` | Ranked worst clusters in the workspace. |
| `report-for-file` / `report-for-range` | Localised report for one file or one selection. |
| `cluster-by-id` | Full member list and signals for a single cluster. |
| `report-get` / `report-query` | Whole-workspace report + filtered query. |
| `rescan` | Force-refresh after large external changes. |
| `list-embedding-models` / `set-embedding-model` | Toggle the semantic (Type-4) pass. |
| `session-config` | Inspect the running server's config. |
| `schema-doc` | Self-describing JSON schema for the report payload. |

The MCP server reads `.deslop-cache/live-report.json` — atomically rewritten by the LSP after every file-watcher pass — and delegates `find-similar` to the LSP over a Unix-domain socket so snippet matching runs against the *running* corpus, not a stale cache. **Every MCP response reflects the workspace as of the last keystroke.**

---

## Live = reactive, all the way through

**Live means reactive.** When you change code, every Deslop surface — the live bubble, the editor decorations, the **Top Offenders** tree, the cluster webview, the status bar, the MCP query results, the agent's view of the workspace — reflects the new state **immediately**. Not on the next save. Not when the editor refreshes. Not on a polling timer.

The LSP file watcher fires on every change with a **250 ms debounce and a 2 s cap**. The pipeline runs incrementally — only the touched files get re-parsed. The new report is written atomically and a `deslop/reportChanged` notification fans out over the LSP wire and over MCP `resources/updated`. Editor surfaces, agent caches, and webviews observe the new report in the same microtask. A cluster that no longer exists in the source code cannot remain on screen, in a hover, in a code lens, or in an MCP response. **Stale UI is a correctness bug, not a polish issue.**

The CLI is the **cold-cache fallback** for one-shot audits and CI gates. Every other surface — LSP, MCP, VS Code extension — is reactive by construction. See [SPEC §[PRINCIPLES-LIVE-IS-REACTIVE]](docs/specs/principles.md#principles-live-is-reactive).

---

## What ships today

- **MCP server (`deslop-mcp`)** — the agent surface. Twelve tools, all powered by the live LSP state.
- **LSP server (`deslop-lsp`)** — diagnostics, hover, code lens, `textDocument/definition`, virtual `deslop://` documents, and custom `deslop/*` methods (`reportGet`, `reportDelta`, `reportForFile`, `reportForRange`, `clusterById`, `duplicatesFindSimilar`, `embeddingListModels`, `embeddingSetModel`, `sessionConfig`, `reportSchemaDoc`, `virtualDocument`, `cpuReport`). Fires `deslop/reportChanged`, `deslop/analysisState`, and `deslop/embeddingProgress` notifications.
- **VS Code extension** — bundles the LSP + MCP binaries; surfaces the live bubble, **Top Offenders** tree, focused-file view, session panel, cluster webview, status bar, hover, code lens, decorations, and a Compare action.
- **CLI (`deslop`)** — cold-cache fallback. Same engine, emits `.json`, `.txt`, `.html` reports for CI gates and bulk audits.

**Languages today:** C#, Rust, Python, Dart — all tree-sitter grammars. **No regex on source. Ever.** TypeScript / JavaScript and Go are on the roadmap.

**IDE extensions in development:**

| IDE | Status |
| --- | --- |
| VS Code | **Shipping** (preferred install) |
| JetBrains (Rider first, then IntelliJ IDEA, PyCharm, WebStorm, RustRover, CLion) | **Actively building** — LSP plugin in `clients/jetbrains/`, Gradle-built, real-binary tests passing |
| Zed | Roadmap |
| Neovim | Roadmap |

---

## What's actually implemented

Every detection algorithm in the table below is a real file, not a future plan:

| Research line | What it implements | Code |
| --- | --- | --- |
| Tree-sitter parsing (Baxter 1998) | C# / Rust / Python / Dart AST per language | [`crates/deslop-core/src/lang/`](crates/deslop-core/src/lang/) |
| AST normalization | Type-2 collapse: `__id__` / `__lit__` + comment/trivia drop | [`lang/shared.rs`](crates/deslop-core/src/lang/shared.rs) |
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

**The VSIX is the one install that gives you everything**: the live bubble, the LSP server, the MCP server, and the `deslop` CLI all at once. The bundled binaries are the canonical ones — every MCP client below points at them by absolute path so you stay version-locked to the extension.

1. Grab `deslop-vscode-X.Y.Z-<target>.vsix` from the [latest GitHub release](https://github.com/Nimblesite/Deslop/releases/latest).
2. Install it:

   ```bash
   code --install-extension deslop-vscode-X.Y.Z-<target>.vsix
   ```

   Or: **Extensions panel → `…` menu → Install from VSIX…**

3. Open a `.cs` / `.rs` / `.py` file. The live bubble lights up the moment you type a duplicate. The command palette exposes **Deslop: Open Report**, **Deslop: Open Worst Cluster**, **Deslop: Jump to Next Occurrence**. The extension prepends its bundled `bin/<platform>/` to the VS Code process `PATH`, so `deslop`, `deslop-lsp`, and `deslop-mcp` are callable from the integrated terminal.

VSIX bundles binaries for `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, and `win32-x64`.

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

Deslop ships `deslop-mcp` — an MCP server exposing the live workspace analysis as twelve tools (see the table at the top of this README).

### The binary lives inside the VSIX — point your agent at it by absolute path

The VS Code extension bundles `deslop-mcp` alongside the LSP, **and that bundled binary is the canonical one for every external MCP client too** (Claude Code, Claude Desktop, Codex, Cursor, Continue). The MCP config snippets below use an absolute path into the unpacked VSIX so the agent runs the exact binary the extension ships — version-locked to the VSIX, no PATH drift, no stale `cargo install` shadowing the release.

After `code --install-extension deslop-vscode-X.Y.Z-<target>.vsix`, the binary lives at:

| Platform | Path |
| --- | --- |
| macOS / Linux | `~/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/<platform>/deslop-mcp` |
| Windows | `%USERPROFILE%\.vscode\extensions\nimblesite.deslop-live-<VERSION>\bin\win32-x64\deslop-mcp.exe` |

`<platform>` is one of `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, `win32-x64`. `<VERSION>` is the installed extension version — bump it whenever you update the VSIX.

> **Do not use a `cargo install`-built binary in your MCP config.** Building Deslop from source produces `target/release/deslop-mcp` for testing only — it is not a distribution channel. The repo deliberately ships no `make install-binary` target, and `make delete-path-binaries` runs before every test target to scrub leaked PATH copies. The only "PATH-resolved" form that is supported is when the user installed the CLI via `brew install nimblesite/tap/deslop` or `scoop install deslop` — those package managers version the binary lock-step with releases.

### Teach your agent to call `find-similar` first

Paste-ready snippet for your project's `AGENTS.md` / `CLAUDE.md` / `.cursorrules`: [docs/snippets/agents-md-recipe.md](docs/snippets/agents-md-recipe.md). It teaches Claude Code, Cursor, Copilot, Continue, and Codex to check for prior art *before* writing new code.

### Claude Code

```bash
claude mcp add deslop -s user -- \
  ~/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/darwin-arm64/deslop-mcp \
  --root .
```

Or edit `~/.claude.json` directly:

```json
{
  "mcpServers": {
    "deslop": {
      "command": "/Users/you/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/darwin-arm64/deslop-mcp",
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
      "command": "/Users/you/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/darwin-arm64/deslop-mcp",
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
command = "/Users/you/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/darwin-arm64/deslop-mcp"
args    = ["--root", "."]
```

### Useful flags

```
--min-nodes 30                       # raise to cut noise on small clones
--no-incremental                     # disable the .deslop-cache/ AST cache (default: on)
--embeddings auto                    # opt in to semantic (Type-4) matching (default: off)
--embedding-model nomic-embed-text   # any local Ollama embedding model
```

`--embeddings` defaults to `off`. `auto` probes the provider and falls back with a warning if Ollama is unreachable; `required` hard-fails when the provider is unreachable.

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
