---
layout: layouts/docs.njk
title: AI Agents — MCP setup for Claude Code, Cursor, and Copilot
description: Tell your coding agent when similar code already exists, before it writes another copy. Wire deslop-mcp into Claude Code, Cursor, Continue, or Codex, and use find-similar to prevent the duplicate.
keywords: deslop, mcp server, claude code, cursor, copilot, continue, codex, find-similar, duplicate code, coding agent
eleventyNavigation:
  key: AI Agents
  order: 3
icon: smart_toy
docsGroup: guides
---

# AI Agents

**Deslop tells your coding agent when similar code already exists, before it writes another copy.** The agent asks; Deslop answers from the live analysis of the repository the agent is working in. Nothing is scheduled, nothing is batched, and no one has to remember to run a scan.

**This page is for you, the human wiring it up.** It covers what the MCP server offers and how to connect each client to it.

The instructions for the agent itself — the rule it follows before writing code, the similarity thresholds, the CLI fallback when MCP is unavailable, and how to parse the report — are on **[For AI](/docs/for-ai/)**. That page is written in the second person and addressed to the machine. Point your agent at that URL.

The paste-ready rule block for your project's `AGENTS.md` / `CLAUDE.md` is in the [agent recipe](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md). It works with Claude Code, Cursor, Copilot, Continue, and Codex.

## The MCP tools, all live

Only `find-similar` belongs in the authoring inner loop. Everything else is a read-only report query or a config tool you reach for on demand, so the agent's working context stays lean instead of carrying a wall of tool output.

| Tool | When to call it |
| --- | --- |
| `find-similar` | **Before** writing new code — does an equivalent already exist? This is the prevention tool. |
| `top-offenders` | Worst clusters in the workspace, worst first. Start cleanup here. |
| `cluster-by-id` | Full member list and signals for one cluster you are about to merge. |
| `report-for-file` | Per-file cluster slice. |
| `report-for-range` | Per-selection cluster slice. |
| `report-get` | Whole-workspace report. |
| `report-query` | Filtered query over the report. |
| `rescan` | Force-refresh after large external changes. |
| `list-embedding-models` | Models the provider advertises. |
| `set-embedding-model` | Switch the same behavior, different code [Type-4] semantic model at runtime. |
| `session-config` | Inspect the running server's effective config. |
| `schema-doc` | Authoritative JSON schema for every response. Call **once** per session, not per response. |

Every response is computed against the **live** workspace state. The editor server holds the live report in memory and refreshes it on every change (debounced, with a hard cap); the MCP server reads that live state over the local IPC endpoint on the next tool call. macOS and Linux use `.deslop/cache/deslop.sock`; Windows uses a token-gated TCP loopback endpoint discovered through `.deslop/cache/deslop.port`. There is no batch step.

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
- **Building from source does not put anything on `$PATH`.** Only `brew` / `scoop` do. Those package managers version the binary lock-step with the release; a `cargo build` does not.

## The agent loop

The headline workflow is reactive, not batch:

1. The agent proposes a change. Before it writes the new code, it calls `find-similar` over the proposed snippet.
2. If `find-similar` returns a cluster above the similarity floor, the agent reuses the canonical occurrence or rewrites the call site.
3. As the agent edits files, the file watcher fires and the analysis refreshes. The MCP server serves the new state on the next tool call.
4. The agent re-queries `top-offenders` or `report-for-file` to confirm the cluster is gone. No re-run, no flag, no batch CLI invocation.

When MCP is not available — CI, a cold-cache audit, or an agent with no MCP client — the loop degrades to the `deslop` CLI, which runs the identical pipeline and emits the identical JSON. The incremental cache is on by default, so a re-run after an edit only re-parses the files that changed. The step-by-step fallback is on [For AI](/docs/for-ai/#if-the-mcp-server-is-unavailable-use-the-cli).

## Configure it

An agent configuring Deslop for a repository needs three things, all documented in the [Configuration reference](/docs/configuration/):

- **[`exclude` vs `report_hide`](/docs/configuration/#exclude-vs-report_hide--the-core-idea)** — `exclude` drops a file before analysis; `report_hide` analyses it but keeps it out of the headline, so "hand-written code duplicates generated code" still surfaces.
- **[Built-in rules](/docs/configuration/#built-in-rules-always-on)** — `node_modules`, `target`, `dist`, generated-code suffixes, and generated-banner detection are already covered. Do not re-add them.
- **[`[threshold]`](/docs/configuration/#threshold--the-ci-gate)** — the opt-in CI gate. Commit the ceiling so local runs, CI, and agents all share one number.

To gate a build on duplication, use the [GitHub Action](/docs/github-action/) — it wraps the same exit-code contract.

## What the agent reads back

`deslop-report.json` is canonical; `.txt` and `.html` are renderers over it. Every report carries an embedded `schema_doc`, so a model can parse the payload without a separate reference. The field-by-field guide — what `bucket`, `signals.fused`, and `occurrences[].hidden` mean and how to act on each — is on [For AI](/docs/for-ai/#read-the-json).

## One engine, three surfaces

The `deslop-core` crate owns the entire pipeline. Three shells consume it:

- **MCP server (`deslop-mcp`)** — the agent surface. `find-similar` plus the focused set of read-only and config tools above. The server delegates every read to the running editor server over the local IPC endpoint, so every response is computed against the live in-memory corpus, not a stale on-disk cache. When the editor server isn't running, the MCP returns an actionable error; CI and one-shot audits use the `deslop` CLI instead.
- **LSP server (`deslop-lsp`)** — the editor surface. Diagnostics, hover, code lens, `textDocument/definition`, virtual `deslop://` documents, and custom `deslop/*` methods (`reportGet`, `reportDelta`, `reportForFile`, `reportForRange`, `clusterById`, `duplicatesFindSimilar`, `embeddingListModels`, `embeddingSetModel`, `sessionConfig`, `reportSchemaDoc`, `virtualDocument`, `cpuReport`). Fires `deslop/reportChanged`, `deslop/analysisState`, and `deslop/embeddingProgress` notifications. Owns the file watcher, the debouncer, and the analysis scheduler.
- **CLI (`deslop`)** — the cold-cache fallback for CI gates and one-shot audits.

All three reuse the same cache layout (`.deslop/cache/fingerprints/`, `.deslop/cache/embeddings/`) and the same JSON schema. Agents wired to the CLI today get the live channel by adding `deslop-mcp` to their MCP config — no schema change, no parser rewrite.

### Push notifications

The editor server fires `deslop/reportChanged` over the LSP wire and `resources/updated` + `deslop/reportChanged` over the MCP wire as soon as a watcher pass completes. Editor surfaces, agent caches, and webviews all observe the new report as soon as the pass commits. Stale UI is a correctness bug per the [LIVE-IS-REACTIVE](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/principles.md#principles-live-is-reactive) invariant.

## JetBrains plugin (in development)

The JetBrains plugin in `clients/jetbrains/` registers an IntelliJ Platform `lsp.serverSupportProvider` and starts `deslop-lsp` for C#, Rust, Python, Dart, JavaScript, TypeScript, PHP, F#, and Go files. Rider is the first product target; IntelliJ IDEA, PyCharm, WebStorm, RustRover, and CLion follow on the same platform LSP API. The plugin is Gradle-built, has real-binary tests against the released `deslop-lsp`, and ships with the same binary-resolution rules as the VS Code extension. Zed and Neovim plugins are on the roadmap — both LSP-capable, both wire-compatible with `deslop-lsp` today.

## What Deslop deliberately does not do

- It does not rewrite your code. Deslop finds, ranks, compares, and prevents duplication; extraction is your call.
- It does not fail CI unless you set a threshold yourself.
- It does not assume "near-miss = bug." Some duplication is intentional (test fixtures, bootstrapping). Deslop reports; you decide.
- It does not talk to the network unless you explicitly pick a remote embedding model.
