# Chapter 2 — Connect Deslop to agents, editors, and CI

Use the editor while changing code, the agent connection before writing it, and the CLI for saved-code scans.

| Program | Job |
|---|---|
| `deslop-lsp` | Runs the editor's live analysis and publishes updates |
| `deslop-mcp` | Forwards agent requests to that running analysis |
| `deslop` | Scans saved files, writes reports, and exits |

MCP is the agent's tool protocol. LSP connects the editor to its language server. The MCP process does not start another scanner. See the [MCP specification](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/mcp.md).

![Agent, editor, and CI use different entry points into Deslop's analysis.](assets/diagrams/02-three-jobs-one-analysis.png)

*Use `find-similar` for proposed code and the CLI for saved source.*

## Use the editor's installed bundle

Open the repository with the Deslop extension enabled. Point the external agent at `deslop-mcp` inside that installed extension. On macOS and Linux the layout is:

```text
~/.vscode/extensions/nimblesite.deslop-live-<VERSION>-<platform>/bin/<platform>/deslop-mcp
```

Some installations omit the platform suffix on the extension directory. Use the actual directory. Windows uses `deslop-mcp.exe`. Set an absolute executable path and an absolute repository root in your client's MCP configuration. The [setup guide](https://deslop.live/docs/ai-integration/) has client-specific examples.

Homebrew and Scoop users can use `deslop-mcp` on `PATH` because those packages install the three programs together. For an extension installation, use its bundled binaries for CLI checks too.

Check each binary with `--version`. Matching version strings are necessary, but two development builds can share a version string. Use one bundle, and record binary hashes when reproducing a result.

## Confirm the connection

1. Call `session {}` and verify its `root` is the repository open in the editor.
2. Call `duplicates {"detail":"summary","limit":5}`.
3. Select one group in the editor and request `cluster-by-id` with its full ID.
4. Compare its kind, occurrence count, and paths.

Top Offenders is the editor view. The agent's ranked-list tool is `duplicates`.

## Let ordinary edits refresh

The editor sends buffer changes to the live server; its watcher observes disk changes. After analysis, the editor updates and the agent's next query reads the refreshed report. Use `rescan` to recover suspected watcher lag, rather than after every edit.

![Changes reach the live server, which refreshes the editor and subsequent agent queries.](assets/diagrams/02-live-update-loop.png)

*Wait for analysis before checking the changed file.*

## Use the CLI without a live editor

For package-manager installations, run:

```sh
deslop . --notext --nohtml --no-color
```

Extension users run the same arguments with the bundled CLI's absolute path. The default JSON output is `.deslop/deslop-report.json` under the scan root. The CLI honours the configured duplication ceiling and exits `3` when it is exceeded, after writing reports.

If MCP fails, inspect an existing JSON report first. Check its version and source ranges: a snapshot may be stale. Generate a fresh CLI report when needed. A saved report cannot search an unwritten snippet.

| Problem | Check |
|---|---|
| No Deslop tools | Client configuration and executable path |
| `LspNotRunning` | Editor server and exact repository root |
| Version or wire mismatch | MCP and LSP from the same bundle |
| Results disagree | Root, build, configuration, and source state |
| Results lag behind edits | Analysis completion, then a focused query or deliberate `rescan` |

**Try it:** trace one actual group from Top Offenders to `cluster-by-id`. Seeing the tool name alone does not prove that it is connected to the right repository.
