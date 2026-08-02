---
layout: layouts/docs.njk
title: AI Integration — MCP server for Claude, Cursor, Copilot
description: Wire deslop-mcp into Claude Code, Claude Desktop, Cursor, Continue, or Codex. A lean, live MCP surface led by find-similar — prevent the copy-paste before it lands.
eleventyNavigation:
  key: AI Integration
  order: 3
icon: smart_toy
---

# AI Integration

Deslop was designed from the first commit with coding agents as a first-class audience. The MCP and LSP shells ship today — both consume the same `deslop-core` pipeline, the same JSON schema, and the same on-disk caches as the CLI. Every MCP tool response is computed against the **live** workspace state — the LSP holds the live report in memory and refreshes it on every change (debounced, with a hard cap), and the MCP server reads that live state over the local IPC endpoint on the next tool call. macOS and Linux use `.deslop/cache/deslop.sock`; Windows uses a token-gated TCP loopback endpoint discovered through `.deslop/cache/deslop.port`. There is no batch step. There is no stale cache.

## The MCP tools, all live

Only `find-similar` belongs in the authoring inner loop — it's the one call the agent makes before writing new code. Everything else is a read-only report query or a config tool you reach for on demand, so the agent's working context stays lean instead of carrying a wall of tool output.

| Tool | Purpose |
| --- | --- |
| `find-similar` | **The prevention tool.** Match a proposed snippet against the live corpus before the agent writes it. |
| `top-offenders` | Ranked worst clusters in the workspace. |
| `report-for-file` | Per-file cluster slice. |
| `report-for-range` | Per-selection cluster slice. |
| `cluster-by-id` | Full member list and signals for one cluster. |
| `report-get` | Whole-workspace report. |
| `report-query` | Filtered query over the report. |
| `rescan` | Force-refresh after large external changes. |
| `list-embedding-models` | Models the provider advertises. |
| `set-embedding-model` | Switch the Type-4 (same behavior, different code) semantic model at runtime. |
| `session-config` | Inspect the running server's effective config. |
| `schema-doc` | Self-describing JSON schema for every response. |

## Wire `deslop-mcp` into your client — point at the VSIX-bundled binary

`deslop-mcp` ships **inside the VS Code extension VSIX**. After you install the extension, every external MCP client (Claude Code, Claude Desktop, Codex, Cursor, Continue) should reference the unpacked VSIX binary by absolute path so the agent runs the exact binary the extension ships — version-locked to the VSIX, no `PATH` drift.

Once the extension is installed from the Marketplace, the binary lives at:

```
~/.vscode/extensions/nimblesite.deslop-live-<VERSION>-<platform>/bin/<platform>/deslop-mcp
```

`<platform>` is `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, or `win32-x64`. `<VERSION>` is the installed extension version — bump it whenever you update the VSIX.

### Claude Code

```bash
claude mcp add deslop -s user -- \
  ~/.vscode/extensions/nimblesite.deslop-live-<VERSION>-darwin-arm64/bin/darwin-arm64/deslop-mcp \
  --root .
```

### Codex (`~/.codex/config.toml`)

```toml
[mcp_servers.deslop]
command = "/Users/you/.vscode/extensions/nimblesite.deslop-live-<VERSION>-darwin-arm64/bin/darwin-arm64/deslop-mcp"
args    = ["--root", "."]
```

### Claude Desktop (`claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "deslop": {
      "command": "/Users/you/.vscode/extensions/nimblesite.deslop-live-<VERSION>-darwin-arm64/bin/darwin-arm64/deslop-mcp",
      "args": ["--root", "/absolute/path/to/your/repo"]
    }
  }
}
```

> **Do not point an MCP client at a `cargo install` or `target/release` build.** Building Deslop from source is for testing the change you just made; it is not a distribution channel. The repo deliberately ships no `make install-binary` target.

## Homebrew / Scoop CLI users — point at the bare `deslop-mcp` on `$PATH`

If you installed the CLI with `brew install nimblesite/tap/deslop` or `scoop install deslop`, the package also puts **`deslop-mcp` and `deslop-lsp` on your `$PATH`** alongside `deslop` — the tap formula and Scoop manifest install all three binaries, version-locked to the release. No VSIX, no extension directory, no absolute path. Use the bare command:

```bash
claude mcp add deslop -s user -- deslop-mcp --root .
```

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

The same `"command": "deslop-mcp"` form works in Codex (`~/.codex/config.toml`), Cursor, and Continue. It is the right value for a checked-in `.mcp.json` or shared team config — every machine resolves it through `$PATH`.

Three things to know:

- **There is no `deslop mcp` subcommand.** The `deslop` CLI runs one-shot and CI audits only; MCP is served by the **separate `deslop-mcp` binary**.
- **`deslop-mcp` not found on `$PATH`?** It was added to the brew/scoop packages in v0.13.0. On an older install, run `brew upgrade deslop` (or `scoop update deslop`) — the current release ships `deslop-mcp` and `deslop-lsp` on `$PATH`.
- **Building from source does not put anything on `$PATH`.** Only `brew` / `scoop` do. Those package managers version the binary lock-step with the release; a `cargo build` does not.

## Prevention beats cure — `find-similar` is the keystone

The fastest deduplication is the one that never lands. Deslop's MCP server exposes `find-similar` as the **prevention** tool: an agent calls it *before* writing a new function, helper, or test setup. If the proposed pattern already exists with high similarity, the agent reuses the canonical implementation instead of authoring a fresh copy.

Paste-ready `AGENTS.md` / `CLAUDE.md` snippet that teaches this to your agents lives at [`docs/snippets/agents-md-recipe.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md). It works with Claude Code, Cursor, Copilot, Continue, and Codex.

## The agent loop (live MCP)

The headline workflow is reactive, not batch:

1. Agent proposes a change. Before it writes the new code, it calls `find-similar` over the proposed snippet via MCP.
2. If `find-similar` returns a cluster above the configured similarity floor, the agent reuses the canonical or rewrites the call site.
3. As the agent edits files, the LSP file watcher fires `deslop/reportChanged`. The MCP server queries the LSP's freshly refreshed report over the IPC socket and serves the new state on the next tool call.
4. The agent re-queries `top-offenders` or `report-for-file` to confirm the cluster is gone. No re-run, no flag, no batch CLI invocation.

For a CLI-only loop (CI gates, cold-cache audits, or agents without MCP), the workflow degrades to:

1. Agent proposes code changes.
2. Agent (or harness) runs `deslop . --output report` (writes `report.json`/`.txt`/`.html`).
3. Agent reads the top `N` clusters from `report.json`.
4. For every cluster above threshold, the agent has three choices: extract to a shared function, reuse the existing implementation, or accept the duplication and annotate why.
5. Agent re-runs Deslop. The top cluster should be different or smaller.

The incremental cache — on by default — means step 5 only pays the cost of parsing files the agent actually touched — unchanged files skip tree-sitter entirely, so a warm pass stays proportional to the size of the change, not the size of the repo.

## Reading the JSON

Every report begins with an embedded `schema_doc` explaining the shape to the agent consuming it — the model does not need a separate reference to understand the payload:

```json
{
  "tool_version": "0.0.0-dev",
  "schema_doc": "…inline description of every field…",
  "metrics": { "analysed_loc": 1832044, "duplicated_loc": 48120, "duplication_percent": 2.63, "clusters_total": 142, "duplicated_files": 318 },
  "action_hints": [
    { "pattern": "bucket=identical", "recommendation": "Identical code. Safe to extract — every copy is the same." }
  ],
  "clusters": [
    {
      "id": "0362505641efe3c7",
      "weight": 2184.0,
      "bucket": "nearly_identical",
      "size": 3,
      "canonical_node_count": 42,
      "signals": { "structural": 1.0, "token_jaccard": 0.97, "embedding_cos": 0.91, "fused": 0.99 },
      "summary": "3 near-identical copies of a 42-node method across UserRepository.cs:120-180, ProductRepository.cs:58-118, OrderRepository.cs:40-102 — safe to extract.",
      "interpretation": "Nearly identical code. Review the locations — small differences may matter.",
      "occurrences": [
        { "path": "UserRepository.cs", "start_byte": 3104, "end_byte": 4820, "start_line": 120, "end_line": 180 }
      ]
    }
  ]
}
```

`summary` and `interpretation` are pre-written for an agent reader: they state what was found, where, and — when the signals agree — whether the duplication is safe to extract. Repository-level remediation guidance lives in the top-level `action_hints`, keyed by `bucket`; it is derived from the signals, never guessed.

## Byte ranges, not line numbers

Deslop's source of truth is `[byte_start, byte_end)`. Line numbers are derived at render time only. Agents editing files should slice by byte range — line-based edits drift when surrounding code moves.

## Stable IDs

Cluster IDs are short hex digests of the cluster's content fingerprint — the first 8 bytes of the cluster's smallest member BLAKE3 hash, rendered as 16 hex characters (e.g. `0362505641efe3c7`). They carry no timestamp, so feeding the same repo to the same binary twice produces the same IDs. An agent can reference a cluster across runs.

## MCP and LSP — shipping

The `deslop-core` crate owns the entire pipeline. Three shells consume it:

- **MCP server (`deslop-mcp`)** — the agent surface. find-similar plus a focused set of read-only and config tools (see the table above). The server delegates every read — `top-offenders`, `report-get`, `report-for-file`, `find-similar`, and the rest — to the running LSP over the local IPC endpoint, so every response is computed against the LSP's live in-memory corpus, not a stale on-disk cache. Unix hosts use `.deslop/cache/deslop.sock`; Windows uses token-gated TCP loopback with `.deslop/cache/deslop.port` discovery. When the LSP isn't running, the MCP returns an actionable error; CI and one-shot audits use the `deslop` CLI instead.
- **LSP server (`deslop-lsp`)** — the editor surface. Diagnostics, hover, code lens, `textDocument/definition`, virtual `deslop://` documents, and custom `deslop/*` methods (`reportGet`, `reportDelta`, `reportForFile`, `reportForRange`, `clusterById`, `duplicatesFindSimilar`, `embeddingListModels`, `embeddingSetModel`, `sessionConfig`, `reportSchemaDoc`, `virtualDocument`, `cpuReport`). Fires `deslop/reportChanged`, `deslop/analysisState`, and `deslop/embeddingProgress` notifications. Owns the file watcher, the debouncer, and the analysis scheduler.
- **CLI (`deslop`)** — the cold-cache fallback for CI gates and one-shot audits.

All three reuse the same cache layout (`.deslop/cache/fingerprints/`, `.deslop/cache/embeddings/`) and the same JSON schema. Agents wired to the CLI today get the live channel by adding `deslop-mcp` to their MCP config — no schema change, no parser rewrite.

### Push notifications

The LSP fires `deslop/reportChanged` over the LSP wire and `resources/updated` + `deslop/reportChanged` over the MCP wire as soon as a watcher pass completes. Editor surfaces, agent caches, and webviews all observe the new report as soon as the pass commits. Stale UI is a correctness bug per the [LIVE-IS-REACTIVE](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/principles.md#principles-live-is-reactive) invariant.

## JetBrains plugin (in development)

The JetBrains plugin in `clients/jetbrains/` registers an IntelliJ Platform `lsp.serverSupportProvider` and starts `deslop-lsp` for C#, Rust, Python, Dart, JavaScript, TypeScript, PHP, F#, and Go files. Rider is the first product target; IntelliJ IDEA, PyCharm, WebStorm, RustRover, and CLion follow on the same platform LSP API. The plugin is Gradle-built, has real-binary tests against the released `deslop-lsp`, and ships with the same binary-resolution rules as the VS Code extension. Zed and Neovim plugins are on the roadmap — both LSP-capable, both wire-compatible with `deslop-lsp` today.

## What Deslop deliberately does not do

- It does not rewrite your code. Extraction is your call.
- It does not fail CI unless you set `--fail-over <percent>` yourself — a repo-wide duplication-percentage gate that exits `3` when exceeded.
- It does not assume "near-miss = bug." Some duplication is intentional (test fixtures, bootstrapping). Deslop reports; you decide.
- It does not talk to the network unless you explicitly pick a remote embedding model.
