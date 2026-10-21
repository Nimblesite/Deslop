---
layout: layouts/docs.njk
title: For AI — how coding agents use Deslop to stop duplicate code
description: "For coding agents: find the Deslop binary, call find-similar before writing, fall back to the CLI when MCP fails, reuse the cache, and fix identical clones first."
eleventyNavigation:
  key: For AI
  order: 4
icon: terminal
docsGroup: reference
---

# For AI

This page is written for you, the coding agent. It tells you how to find Deslop on this machine, how to use the MCP server, what to do when the MCP server is not working, how to run the CLI without repeating work, and how to remove duplicates in a safe order. It covers [0.35.0](/releases/). A human wiring up an MCP client wants [AI Agents](/docs/ai-integration/) instead.

## What you are working with

Deslop is one engine shipped as three programs. They read the same `.deslop.toml`, use the same cache and produce the same JSON report.

| Program | What it does | When you use it |
| --- | --- | --- |
| `deslop-lsp` | The engine inside the editor. It analyses the workspace, keeps the result in memory and updates it on every file change. | Never directly. The editor starts it. |
| `deslop-mcp` | The MCP server. It has no engine of its own; every tool call is a request to the running `deslop-lsp` for the same folder. | First choice while the editor is open on the repository. |
| `deslop` | The command-line tool. It analyses the folder you give it, writes reports and exits. | When the MCP server is unavailable or misbehaving, in CI, and for HTML or JSON reports on disk. |

Everything Deslop writes lives under `.deslop/` at the scan root:

| Path | Contents |
| --- | --- |
| `.deslop/deslop-report.json` / `.txt` / `.html` | The CLI's reports. JSON is for you; HTML is for humans. |
| `.deslop/cache/fingerprints/` | Per-file parse results, keyed on file content. The CLI and the editor share them. |
| `.deslop/cache/live-report.json` | The editor's most recent full analysis, saved so the next editor start is instant. |
| `.deslop/cache/deslop.sock` (`deslop.port` on Windows) | The local endpoint the MCP server uses to reach the editor's engine. Its presence means an engine is, or was, running for this folder. |
| `.deslop/logs/` | CLI logs. Read them when a run fails. |

## Find the binary

### The VS Code extension is already installed: use its binaries

The extension bundles all three programs. Prefer them over anything on `PATH`: they are the same build as the engine the editor is running, so the CLI can read the editor's reports and its numbers agree with the editor's.

macOS and Linux:

```bash
DESLOP=$(find ~/.vscode/extensions -maxdepth 4 -type f \
  -path '*/nimblesite.deslop-live-*/bin/*/deslop' 2>/dev/null | sort -V | tail -1)
"$DESLOP" --version
```

Windows (PowerShell):

```powershell
$deslop = Get-ChildItem "$env:USERPROFILE\.vscode\extensions\nimblesite.deslop-live-*\bin\*\deslop.exe" |
  Sort-Object LastWriteTime | Select-Object -Last 1 -ExpandProperty FullName
& $deslop --version
```

The layout is `nimblesite.deslop-live-<VERSION>[-<platform>]/bin/<platform>/`, holding `deslop`, `deslop-lsp` and `deslop-mcp` (`.exe` on Windows). `<platform>` is `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64` or `win32-x64`. Editors built on VS Code keep extensions in their own folder: replace `~/.vscode/extensions` with `~/.vscode-insiders/extensions`, `~/.cursor/extensions`, `~/.windsurf/extensions` or `~/.vscode-oss/extensions`. If more than one version is installed, use the newest; the editor may not have removed the old one yet.

### No extension: install the CLI

macOS and Linux (Homebrew):

```bash
brew install nimblesite/tap/deslop
deslop --version
```

Windows (Scoop):

```powershell
scoop bucket add nimblesite https://github.com/Nimblesite/scoop-bucket
scoop install deslop
deslop --version
```

Both install `deslop`, `deslop-lsp` and `deslop-mcp` onto `PATH` at the same version. Ask before installing anything if your instructions require it. Never use a `cargo build` or a `target/release` copy as an installed tool: it is a test build, and it drifts from the engine the editor runs.

## The MCP server

Every MCP answer comes from the editor's live analysis, which updates as files change. There is no batch step. Each tool describes itself in `tools/list`; this is how they fit together:

| Tool | Use it to |
| --- | --- |
| `find-similar` | Ask, **before writing code**, whether equivalent code already exists. Pass a `snippet` and `language`, or a `path` with `start_byte` and `end_byte`. Read `clusters` and also `existing`: places where the snippet's code already appears once, outside any cluster. An empty `clusters` with a non-empty `existing` means you are about to create a duplicate. |
| `duplicates` | List clusters worst first. Scope with `path` (and `start_byte`/`end_byte`); filter with `languages`, `path_contains`, `severities` or `min_size`. Pages default to 5 clusters; use `detail: "summary"` with a larger `limit` for an overview and page with `offset` until `offset + returned >= total_clusters`. |
| `cluster-by-id` | Get every occurrence of one cluster before you change it. Pages hold 100 occurrences; page with `offset`. |
| `merge-plan` | Get a read-only plan for merging a cluster, including a `WorkspaceEdit`, before you edit by hand. |
| `compare-pair` | Get the engine's evidence for two specific occurrences. Clusters carry no pair measurements. |
| `rescan` | Force a refresh, optionally limited to `paths` you just changed, when the analysis looks behind your edits. |
| `session` | Check the workspace `root`, settings and cache state. Never call `set-embedding-model` unless the user explicitly asks for that model. |
| `schema-doc` | Get the meaning of every field. Call it once per session. |

Responses are capped in size. A response with `truncated: true` or a cluster with `occurrences_truncated: true` is incomplete, not wrong: page with `offset`, lower `limit`, use `detail: "summary"`, or call `cluster-by-id`.

### When the MCP server is not working properly

An error, a timeout or an empty answer from a failing server is not evidence that there is no duplicate code. Switch to the CLI in any of these cases:

| What you see | What it means | What to do |
| --- | --- | --- |
| No Deslop tools, or the connection fails | The MCP client is not configured, or the server did not start. | Use the CLI. Tell the user the MCP server is not connected. |
| `LspNotRunning` | No editor engine is running for this folder. The error names the endpoint it tried. | Check that the editor is open on this repository and that the MCP server's `--root` is the same folder. Meanwhile use the CLI. |
| An error naming two different Deslop versions | `deslop-mcp` and `deslop-lsp` come from different builds. | Use the CLI. Tell the user to reinstall the extension and point the MCP client at the binary inside it. |
| Still analysing, or results that do not reflect your edits | The analysis has not caught up. | Call `rescan` with the `paths` you changed. If it is still behind, use the CLI. |
| An answer that contradicts the code you can read, such as no match for code you can see copied | A possible accuracy defect. | Confirm with the CLI and report the difference to the user with the cluster id or snippet. |

## If the MCP server is unavailable, use the CLI

The CLI runs the same engine and writes the same JSON. It cannot answer "does this unwritten snippet exist?", so for prevention: read the report before the change, make the change, run again and look for your changed paths in `clusters[].occurrences[]`.

### Do not redo work: use what is already there

Pick the cheapest option that answers your question.

**1. The editor has already analysed the repository.** When `.deslop/cache/live-report.json` exists, render it instead of analysing again. This takes well under a second:

```bash
"$DESLOP" --from-report .deslop/cache/live-report.json --no-color
```

This writes `.deslop/deslop-report.txt` and `.html` and copies the JSON to `.deslop/deslop-report.json`. The file holds the editor's last full analysis; it is not rewritten after each edit. Use it to read the current state, not to check a change you just made. If it will not parse, it came from a different Deslop version: use the extension's own `deslop`.

**2. Re-render a report you already have.** `--from-report .deslop/deslop-report.json --nojson` writes a fresh HTML and text report from the last CLI run without analysing anything.

**3. Analyse, reusing the cache.** A plain run reuses `.deslop/cache/fingerprints/`, which is on by default, so only files whose content changed are parsed again. Matching is still repository-wide, so a run takes longer than a re-render. Ask for only the output you need:

```bash
"$DESLOP" . --notext --nohtml --no-color
```

**4. Check only your change.** Scope the report to a diff; the whole tree is still scanned, so new code that copies untouched code is found:

```bash
git diff HEAD | "$DESLOP" . --diff - --only-changed --notext --nohtml --no-color
```

Do not pass `--no-incremental`. It switches the cache off and parses every file again; it exists for diagnosing cache problems.

### Basic options

| Option | Meaning |
| --- | --- |
| `PATH` | Folder to analyse. Defaults to the current folder. There are no subcommands: `deslop duplicates` is an error, not a tool call. |
| `--output <PREFIX>` | Where reports go. `.json`, `.txt` and `.html` are appended. Default `.deslop/deslop-report`. |
| `--nojson` / `--notext` / `--nohtml` | Skip one output. At least one must remain. |
| `--from-report <FILE>` | Skip analysis and render an existing JSON report. |
| `--diff <FILE>` / `--only-changed` | Tag findings against a unified diff (`-` reads stdin) / drop findings outside it. |
| `--compare <PATH:START:END>` | Pass twice to print the engine's evidence for two occurrences as JSON, instead of a report. |
| `--min-nodes <N>` | Smallest code unit considered, in syntax-tree nodes. Default 30. Leave it alone unless the user asks. |
| `--config <FILE>` | Use a specific `.deslop.toml`. |
| `--fail-over <PERCENT>` / `--no-fail-over` | Set or clear the duplication ceiling for this run. |
| `--no-color` | Plain terminal output. Use it; you are not a terminal. |
| `--log-to-console` | Send logs to stderr instead of a file. |

Compare two occurrences, using byte ranges copied from the JSON:

```bash
"$DESLOP" . --compare "src/a.rs:120:480" --compare "src/b.rs:96:455"
```

Exit codes: `0` success, `1` runtime error, `2` invalid arguments, `3` the duplication ceiling was exceeded. On `3` the reports are still written in full; read them.

## Deduplicate: identical clones first

Work through clone kinds in this order. Within a kind, follow report order, which is worst first.

| Order | `kind` | What it is | What to do |
| --- | --- | --- | --- |
| 1 | `identical` | Copies whose text matches apart from whitespace. | Keep one copy and point every caller at it. The least risky change with the clearest payoff. |
| 2 | `nearly_identical` | Copies with renamed identifiers, changed literals or small edits. | Extract one shared function and turn the differences into parameters. Read every difference first. |
| 3 | `loosely_similar` | Substantial copied work with larger edits. | Read both ranges. Merge only when the result is simpler than the copies. |
| 4 | `same_behavior` | A match from the optional meaning-based layer. | Treat as a lead. Similar meaning does not prove the two are interchangeable. |
| — | `structural_only` | Code with the same shape only. Not a clone. | Ignore it. It never counts toward duplication. |

List the identical clusters from a CLI report:

```bash
jq -r '.clusters[] | select(.kind == "identical")
  | "\(.id)  mass \(.mass)  \(.occurrence_count)x  \(.occurrences[0].path):\(.occurrences[0].start_line)"' \
  .deslop/deslop-report.json
```

Over MCP, call `duplicates` with `detail: "summary"` and a larger `limit`, and take the rows whose `kind` is `identical`.

For each cluster:

1. Fetch every occurrence (`cluster-by-id`, or `clusters[].occurrences[]` in the JSON) and read the code at each range. Skip occurrences marked `hidden`; they are generated or excluded from the report on purpose.
2. Over MCP, call `merge-plan` and read its verdict before editing.
3. Choose where the single copy lives: the most general module that every caller can depend on, not the first file you opened.
4. Move the code there and change every caller. Do not leave the old copies behind or add a third copy.
5. Run the project's tests.
6. Confirm the cluster is gone: `duplicates` with the changed `path`, or `rescan` with `paths`; over the CLI, a cached run or a `--diff` run as above.

Also read `literal_findings`: the same literal value repeated across the code. Replace each with one named constant.

Do not make findings disappear by other means. Do not add `exclude` or `report_hide` rules, raise `--min-nodes`, change `[total] include`, or relax a threshold to hide a duplicate. If a duplicate is deliberate, leave it and say why.

## Read the JSON

Parse the JSON, never the text or HTML. The report carries its own field guide in `schema_doc`; over MCP, call `schema-doc` once.

| Field | Meaning |
| --- | --- |
| `metrics.duplication_percent` | Duplicated lines divided by analysed lines, counting only the kinds listed in `metrics.total_includes`. Not a measure of detector accuracy. |
| `metrics.per_kind[]` | Duplicated lines and percentage for each kind, and `included_in_total`: whether that kind counts toward the headline figure. |
| `metrics.threshold.breached` | Whether the configured ceiling was exceeded. The CLI writes the reports and then exits `3`. |
| `clusters` | Clones in descending mass order, then informational findings. |
| `cluster.id` | Stable identifier. Use it to refer to a finding everywhere. |
| `cluster.kind` | The clone kind in the table above. Older reports used `bucket`. |
| `cluster.mass` / `cluster.rank` | How much duplicated code the cluster represents, and its position. Mass ranks impact, not confidence or how safe a merge is. Informational findings have zero mass and no rank. |
| `cluster.severity` | Diagnostic level shown in the editor. Independent of mass and percentage. |
| `cluster.occurrences[]` | `path`, `start_byte`, `end_byte`, `start_line`, `end_line` and `hidden` for each copy. |
| `occurrences[].hidden` | Hidden by report policy. Contributes no duplicated lines. |
| `literal_findings` | Repeated literal values, separate from clusters. |
| `tool_version` | The Deslop version that wrote the report. |

### Byte ranges, not line numbers

Ranges are `[start_byte, end_byte)`: the end is excluded. They refer to the file as it was when analysed. After any edit, analyse again before you reuse an offset. Lines are included for reading, not for editing.

### Cluster IDs are stable

Rank changes as other clusters change; the ID does not, as long as the cloned code itself is unchanged. Editing the cloned code can change the ID.

## Configuring a repository

`.deslop.toml` sits at the repository root. [`exclude`](/docs/configuration/) removes files before analysis. `report_hide` keeps files in the analysis but hides their occurrences, so a copy of generated code is still reported. Built-in rules already skip dependency folders, build output and files with a generator banner. `[threshold] max_duplication_percent` makes the CLI exit `3` above a ceiling; it does not change what counts as a clone. Change configuration only when the user asks.

## Operating rules

- Call `find-similar` before writing any code unit longer than a few lines. On a match, reuse the existing code.
- Read the actual code at every occurrence before editing. A clone kind is not proof that a merge preserves behaviour.
- Never treat a failed or empty answer from a broken MCP server as "no duplicates". Fall back to the CLI.
- Never hide, exclude or weaken thresholds to make a finding go away.
- If neither the MCP server nor the CLI works, tell the user. Do not guess.

See [Accuracy Transparency](/docs/accuracy-transparency/) for what the measurements can and cannot tell you, and the [agent recipe](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md) for a block to paste into a project's `AGENTS.md`.
