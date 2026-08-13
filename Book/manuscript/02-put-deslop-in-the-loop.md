# Chapter 2 — Connect Deslop to agents, editors, and CI

The Workshop repository is open in an editor. A coding agent has been asked to add another request validator. Deslop already runs in CI, but the agent cannot query it while working.

If the agent writes a copy, CI may catch it. By then the new code exists. The agent has named it, connected it to callers, written tests, and handed the change over for review. Removing the copy now costs more than avoiding it.

The fix is to make Deslop available while the code is being written. The agent needs a way to ask about proposed code. The developer needs the editor to update as files change. CI needs a reliable command that starts, scans, reports, and exits.

These are three different jobs. Deslop provides a different tool or connection for each one.

## What you will learn

By the end of this chapter, you should be able to:

- explain what the agent connection, editor connection, and command-line tool each do;
- connect an agent to the same live analysis used by the editor;
- explain why only the live agent connection can check code before it is written;
- confirm that the editor and agent are looking at the same repository with compatible Deslop versions; and
- choose the command line for CI and other jobs that do not have a live editor session.

## What MCP, LSP, and CLI mean here

Deslop has three main ways to use it:

1. **The agent asks before writing.** It sends proposed code to `find-similar` and gets back matching code from the repository. This uses MCP, the Model Context Protocol.
2. **The editor shows findings while you work.** It starts Deslop's live server, sends file changes to it, and receives updated duplicate groups. This uses LSP, the Language Server Protocol.
3. **The terminal or CI scans the saved repository.** The `deslop` command runs one analysis, writes a report, and exits. This is the CLI, or command-line interface.

MCP and LSP are communication standards. They are not different duplicate detectors.

The official [MCP architecture](https://modelcontextprotocol.io/specification/2025-06-18/architecture) says MCP “follows a client-host-server architecture.” In this case, the host is the coding-agent application and the Deslop server provides tools such as `find-similar`.

Microsoft's [Language Server Protocol documentation](https://microsoft.github.io/language-server-protocol/) says its goal is to “standardize the protocol for how such servers and development tools communicate.” Deslop uses that connection to keep its editor features current.

You do not need to understand either protocol in detail to use Deslop. MCP carries questions from the agent to Deslop. LSP carries file changes and updated results between the editor and Deslop. The CLI scans saved files without an editor.

![A coding agent, a developer's editor, and CI use different Deslop entry points for different jobs.](assets/diagrams/02-three-jobs-one-analysis.png)

*Figure 2.1 — The agent connection checks proposed code. The editor connection updates results while files change. The CLI scans saved files in a terminal or CI job.*

## What actually runs

Deslop's analysis code lives in its core engine. That engine parses source files, compares code, builds duplicate groups, ranks them, and creates the report model used throughout the product.

The three entry points use that engine in different ways:

- `deslop-lsp` stays running for the editor. It owns the live analysis session, notices file changes, and publishes updated reports.
- `deslop-mcp` accepts tool calls from the coding agent. It does not run a second live scan. It forwards the agent's requests to the running editor server and returns the result.
- `deslop` runs the core analysis as a one-shot command. It is appropriate for CI, terminal audits, and fallback checks.

The phrase “one engine” does not mean that one process does every job. It means the matching rules, duplicate labels, ranking, and report fields come from the same implementation. The agent and editor can therefore refer to the same duplicate group by its stable group ID.

They will only get the same answer when they use the same inputs. Check these four things before treating different results as a detection bug:

1. **Repository root:** the editor and agent must use the same top-level directory.
2. **Deslop version:** the editor server, agent connector, and CLI must come from the same release.
3. **Configuration:** they must use the same `.deslop.toml` and any other settings that change the analysis.
4. **Source state:** they must be looking at the same saved or in-memory code at the same stage of the edit.

The [Deslop live-analysis specification](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/live.md) assigns the file watcher and live analysis session to `deslop-lsp`. The [MCP specification](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/mcp.md) requires agent queries to use that running session. The CLI starts its own one-shot run and exits. This separation avoids two background scanners racing to describe the same workspace.

## Why the live agent connection prevents copies

The CLI can only scan code that exists in the repository. Proposed code does not exist there yet.

`find-similar` solves that problem. The agent can send the code it is about to write, or a range it has just edited, to Deslop. Deslop compares that source with the live repository and returns the closest existing occurrences.

The steps are short:

1. The agent drafts the new function or helper without adding it to a file.
2. The agent calls `find-similar` with that proposed source.
3. When the result is strong or borderline, the agent opens the returned occurrence and reads it.
4. The agent reuses the existing code, changes the existing owner, or proceeds with genuinely new code.
5. After editing, the agent asks for a focused file or range report to check the result.

The [official Deslop instructions for agents](https://deslop.live/docs/for-ai/) make `find-similar` the authoring check. Other tools answer different questions. `report-for-file` narrows the report to one file. `report-for-range` checks a selection. `top-offenders` starts a cleanup from the highest-impact duplicate group. `cluster-by-id` returns the full evidence for one group.

Do not load the full repository report into every agent prompt. Ask the smallest question that supports the current decision. A proposed helper needs `find-similar`, not hundreds of unrelated duplicate groups.

Chapter 3 covers the decision thresholds and the exact response to each result. The important setup point here is that `find-similar` needs the live agent connection. A CLI scan after writing is a useful fallback, but it is detection after the copy exists, not prevention.

## Use matching Deslop versions and repository paths

Most setup problems are caused by a wrong executable path, a wrong repository root, or mixed Deslop versions.

If you use the VS Code extension, it starts the bundled `deslop-lsp` for the open workspace. Configure your external agent to use the `deslop-mcp` binary from the same installed extension. On macOS and Linux, the path has this form:

```text
~/.vscode/extensions/nimblesite.deslop-live-<VERSION>-<PLATFORM>/bin/<PLATFORM>/deslop-mcp
```

Use the real installed version and platform directory. Do not leave the angle-bracket placeholders in your configuration.

Codex uses TOML for this configuration. The important part looks like this:

```toml
[mcp_servers.deslop]
command = "/absolute/path/to/the/installed/deslop-mcp"
args = ["--root", "/absolute/path/to/your/repository"]
```

An absolute repository path is easier to debug than `.` because different agent hosts can start from different working directories.

If you installed Deslop with Homebrew or Scoop, those packages install `deslop`, `deslop-lsp`, and `deslop-mcp` together. In that case, a client can use the command on `PATH`:

```toml
[mcp_servers.deslop]
command = "deslop-mcp"
args = ["--root", "/absolute/path/to/your/repository"]
```

The [Deslop MCP setup guide](https://deslop.live/docs/ai-integration/) contains client-specific examples for Codex, Claude Code, Claude Desktop, Cursor, and Continue. Follow that guide for the configuration file used by your agent host.

Before debugging anything more complicated, check the installed versions. Package-manager users can run these bare commands. Extension users should replace each command with the absolute path to the corresponding binary inside the extension:

```sh
deslop --version
deslop-lsp --version
deslop-mcp --version
```

The version numbers should match. Do not point the editor at one release and the agent at a random binary left in a build directory. Matching versions keep the live connection and report format in agreement.

## What happens after a file changes

The live path is useful because it keeps moving with the code.

When you edit an open file, the editor sends the changed buffer to `deslop-lsp`. When another tool changes a file on disk, the file watcher notices it. Deslop groups rapid changes together so it does not start an analysis for every keystroke. It then analyses the new state and publishes an updated report.

The editor receives that report and updates its Deslop views. The next agent tool call reads the same current report from the live server. Neither the editor nor the agent needs to start a fresh CLI process for an ordinary edit.

![A source edit is analysed once, then the refreshed report is delivered to both the editor and the next agent query.](assets/diagrams/02-live-update-loop.png)

*Figure 2.2 — After Deslop analyses a change, the editor updates and the agent's next query reads the new report.*

In normal work, wait for the live analysis to finish and query the file or range you changed. Use `rescan` only when a large external change has occurred or you have evidence that the live state missed something. Repeatedly forcing full rescans makes the live setup harder to reason about and throws away the benefit of focused updates.

## The developer and agent should name the same finding

The editor is for quick human inspection. It can show the live bubble near edited code, the Top Offenders tree, code lenses, and detailed reports. These views use readable labels and file locations.

The agent receives structured fields: the duplicate-group ID, label, score evidence, and occurrence ranges. That response is easier for software to consume, but it describes the same report.

This gives you a simple end-to-end check:

1. Pick a real duplicate group in the editor's Top Offenders view.
2. Copy its stable group ID.
3. Ask the agent to call `cluster-by-id` with that ID.
4. Compare the label, occurrence count, and file paths.

If they agree, the human and agent are looking at the same finding. If they do not agree, stop and check the root, version, configuration, and source state. Do not make a refactoring decision from two different reports.

Call `schema-doc` once at the beginning of an agent session if the agent needs to understand Deslop's structured response. It describes the current report fields. Calling it before every query adds noise without adding new information.

## Use the CLI when there is no live editor

CI does not need an editor session. Neither does a one-off audit on a build machine. Use the CLI for those jobs:

```sh
deslop . --notext --nohtml --no-color
```

The command scans the current directory and writes the main JSON report to `.deslop/deslop-report.json`. CI can inspect that report and apply the repository's configured duplication limit.

The CLI is also the fallback when the agent connection is unavailable. Run a baseline before editing, make the change, run Deslop again, and inspect duplicate groups that include the changed file. If the new code appears in a strong group, remove or reuse it immediately.

That fallback is still valuable, but the timing is different:

- `find-similar` can stop an unwritten copy;
- a CLI scan can catch a copy soon after it is written; and
- a later cleanup must reconcile code that may already have diverged.

Kevin Moore's [Deslop Duplication Audit Protocol](https://github.com/kevmoo/kevmoo_skills/blob/main/skills/deslop-duplication-audit/SKILL.md) uses command-line reports for read-only discovery before cleanup. That is a good use of a saved report: it records the repository state before anyone changes the design. It is not a replacement for the live authoring check.

## Diagnose setup failures in a fixed order

Do not respond to a failed agent query by guessing whether duplicate code exists. Use the error to find the broken connection.

| What you see | Likely cause | What to check |
|---|---|---|
| The agent has no `find-similar` tool | The MCP configuration was not loaded | Restart or reload the agent host after checking its MCP configuration file |
| `find-similar` says the LSP is not running | The editor server is not running, or it is running for another root | Open the repository with the Deslop extension and compare the exact root named in the error |
| The editor and agent show different groups | Root, version, configuration, or source state differs | Compare all four inputs before rescanning |
| The live result does not reflect a large external edit | The watcher has not completed or missed the change | Wait for analysis, then use a focused query or one deliberate `rescan` |
| CI and the editor disagree | The CLI and extension are not using the same release or settings | Print versions and confirm the directory from which CI runs |

![Matching the repository root and Deslop version keeps the editor, agent, and CLI on the same code and report format.](assets/diagrams/02-root-and-version-check.png)

*Figure 2.3 — Check the repository root first, then the installed version. These two mistakes explain many apparently inconsistent results.*

If no live connection is available but the CLI works, use the CLI fallback and say that prevention was unavailable. If neither path works, report the setup failure. Do not silently skip the duplicate check.

## Workshop exercise

Use a disposable copy of the Workshop repository. Do not create a fake duplicate just to test the connection.

1. Open the repository root in the editor with the Deslop extension enabled.
2. Record the absolute path shown by `pwd` on macOS or Linux, or `Get-Location` in PowerShell.
3. Check that the agent's `deslop-mcp` configuration uses that same absolute path.
4. Run the three `--version` commands and record the matching version.
5. Ask the agent to call `session-config`. Confirm that its workspace root is the repository you opened.
6. Pick an existing group in Top Offenders, copy its stable ID, and ask the agent for `cluster-by-id`.
7. Confirm that the editor and agent agree on the label, number of occurrences, and file paths.
8. Make one small, reversible edit in the disposable copy. Wait for analysis and confirm that both views update.
9. Close the live path and run the CLI once. Confirm that the saved-code report uses the same labels and group IDs for unchanged findings.

Record the result in plain language:

```text
Repository root: <absolute path>
Deslop version: <matching version>
Editor server: running for this root
Agent connection: session-config reports this root
Cross-check: group <stable ID> matches in editor and agent response
CLI fallback: available for saved-code checks and CI
```

The checkpoint passes only when you can trace one real finding from the editor to the agent response. Seeing a tool name in the agent is not enough; the tool must be connected to the repository you are actually changing.

## Instruction for coding agents

```text
Use find-similar through Deslop MCP before writing a new code unit. The MCP
connection must point to the same absolute repository root and Deslop release
as the editor's live server. After editing, use a focused file or range report.
Use the CLI for CI, one-off audits, or immediate fallback checks. If the live
connection fails, diagnose it or use the CLI and state that prevention was
unavailable. Never skip the duplicate check silently.
```

## Main points

- The agent can ask about proposed code before adding it to the repository.
- The editor and agent read results from the same live analysis session.
- The CLI remains the correct tool for CI and standalone audits.
- MCP and LSP are communication methods, not separate detection engines.
- Matching roots, versions, configuration, and source state are required for matching results.
- A CLI fallback catches a new copy after writing; it cannot fully replace the live prevention check.

## Authoritative sources

- Deslop, [AI Agents — MCP setup](https://deslop.live/docs/ai-integration/).
- Deslop, [For AI — how coding agents use Deslop](https://deslop.live/docs/for-ai/).
- Deslop, [Live analysis specification](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/live.md) and [MCP specification](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/mcp.md).
- Model Context Protocol, [official architecture specification](https://modelcontextprotocol.io/specification/2025-06-18/architecture).
- Microsoft, [Language Server Protocol](https://microsoft.github.io/language-server-protocol/).
- Kevin Moore, [Deslop Duplication Audit Protocol](https://github.com/kevmoo/kevmoo_skills/blob/main/skills/deslop-duplication-audit/SKILL.md).
