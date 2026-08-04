---
layout: layouts/docs.njk
title: For AI — how coding agents use Deslop to stop duplicate code
description: Addressed to the agent. Call find-similar before writing code; when MCP is unavailable, fall back to the deslop CLI. Thresholds, JSON fields, byte ranges.
keywords: deslop, ai agent, coding agent, find-similar, mcp, cli fallback, duplicate code, claude code, cursor, codex
eleventyNavigation:
  key: For AI
  order: 4
icon: terminal
---

# For AI

**This page is addressed to you, the coding agent.** Everything below is an instruction you can act on directly — not a description of a product for someone else to evaluate. If you are a human wiring an MCP client up, you want [AI Agents](/docs/ai-integration/) instead; come back here to point your agent at a single URL.

Deslop exists to stop a duplicate from ever being written. You are the one writing the code, so you are the one who has to check.

## The one law: check before you write

Before you author any new code unit — function, method, class, helper, fixture, test setup, parser branch, error type, route handler, view model — call `find-similar` with the proposed snippet (or a `path` + `start_byte` + `end_byte` range) and read `signals.fused` in the response:

| `signals.fused` | Bucket | What you do |
| --- | --- | --- |
| `≥ 0.85` | `identical` / `nearly_identical` | **Do not write the copy.** Reuse the canonical occurrence the tool returns. Extract a shared helper if neither call site fits as-is. |
| `0.6 – 0.85` | any | Read the canonical occurrence before you decide. Bias toward reuse. |
| `< 0.6` | any, or empty | Author it. |
| any | `structural_only` | Shape-only match — often sibling boilerplate. Read the canonical occurrence before concluding anything. |

Do not skip the call because the code "looks new." Most duplicates are written by someone certain the code was new.

`find-similar` is the **authoring** tool. When you are cleaning up duplication that already exists, start at `top-offenders` and then pull `cluster-by-id` for the cluster you are about to merge.

The paste-ready rule block for a project's `AGENTS.md` / `CLAUDE.md` is in the [agent recipe](https://github.com/Nimblesite/Deslop/blob/main/docs/snippets/agents-md-recipe.md).

## If the MCP server is unavailable, use the CLI

**Do not skip the check because a tool call failed, and do not fall back to memory.** A duplicate that lands because the gate was down is the exact failure Deslop exists to prevent. Work down this ladder.

### 1. Diagnose which failure you have

| What you see | What it means |
| --- | --- |
| `LSP is not running — start deslop-lsp to enable this tool.` | The MCP server is wired correctly, but the editor server that holds the live analysis is not up. The error names the absolute socket path it tried. |
| The same error, and a `deslop-lsp` **is** running | MCP was launched against a different `--root` than the LSP. Compare the socket path in the error against the workspace you are editing. |
| No `find-similar` tool exists at all | No MCP server is configured for this session. |
| The tool call times out or the transport errors | Treat it as unavailable and drop to the CLI. |

### 2. Try to restore the live path

If the workspace is open in an editor with the Deslop extension, the editor server starts on its own — open a supported source file and retry the tool call. If MCP and the LSP disagree about the root, the fix is the MCP client's `--root` argument, not a workaround.

### 3. Otherwise, drop to the CLI

The `deslop` CLI runs the identical pipeline and emits the identical JSON schema. Run it from the repository root:

```bash
deslop . --notext --nohtml --no-color
```

That writes the canonical report to `.deslop/deslop-report.json` — the only file you should parse. `--notext --nohtml` skips the two human renderers you do not need; `--no-color` keeps the stderr summary clean for a log.

**Be honest with yourself about what this costs you.** There is no snippet query in the CLI — `find-similar` is an MCP tool, and the CLI cannot evaluate code you have not written yet. The CLI loop is therefore *catch immediately*, not *prevent*:

1. Run `deslop .` once before you start, so you have a baseline.
2. Before authoring, scan the baseline `clusters[]` for the file and the neighbouring files you are about to touch. If a cluster already covers the pattern you were going to add, reuse its canonical occurrence — that is the CLI's version of prevention, and it catches the common case.
3. Write the change.
4. Re-run `deslop . --notext --nohtml`. The fingerprint cache is on by default, so this re-parses only the files you actually touched — the cost is proportional to your change, not to the repository. Run it per change, not per session.
5. Search `clusters[].occurrences[]` for the path you just wrote. If your new code appears in a cluster whose `signals.fused ≥ 0.85`, you just wrote a duplicate. Collapse it now, while the change is still in your working set.
6. Re-run and confirm the cluster is gone or smaller.

A run exits `3` when repo-wide duplication crosses a configured ceiling; the report is still written on a breach, so parse it either way. Full table in [Exit codes](/docs/configuration/#exit-codes).

If neither MCP nor the CLI is available, say so and stop. Do not guess.

## Read the JSON

`deslop-report.json` is canonical and the **only** file you should parse — `.txt` and `.html` are renderers over it. Every report begins with an embedded `schema_doc` describing its own shape, so you do not need a separate reference to read the payload. Over MCP, call `schema-doc` **once** per session, never per response.

```json
{
  "tool_version": "0.0.0-dev",
  "schema_doc": "…inline description of every field…",
  "metrics": {
    "analysed_loc": 1832044,
    "duplicated_loc": 48120,
    "duplication_percent": 2.63,
    "clusters_total": 142,
    "duplicated_files": 318,
    "threshold": { "percent": 5.0, "breached": false, "source": "config" }
  },
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
        { "path": "UserRepository.cs", "start_byte": 3104, "end_byte": 4820, "start_line": 120, "end_line": 180, "hidden": false }
      ]
    }
  ]
}
```

`summary` and `interpretation` are written for you: they state what was found, where, and — when the signals agree — whether it is safe to extract. Repository-level guidance is in the top-level `action_hints`, keyed by `bucket`, and is derived from the signals rather than guessed.

| Field | How to act on it |
| --- | --- |
| `metrics.duplication_percent` | The repo-wide headline number a CI gate compares against. |
| `metrics.threshold.breached` | `true` → the run exited `3` and the gate failed. `source` is `cli`, `config`, or `none`. |
| `clusters` | Sorted by `weight` **descending** — `clusters[0]` is always the worst offender. Work top-down; do not start in the middle. |
| `bucket` | `identical` / `nearly_identical` → extract a shared definition. `structural_only` → shape matched with no token or semantic evidence; verify it is a real duplicate before extracting. `loosely_similar` → parametrise the difference. `same_behavior` → reconcile two implementations of one behaviour (requires embeddings). |
| `signals.fused` | Unit-bounded confidence. `≥ 0.85` is the act-now line — the same threshold as the law above. |
| `occurrences[].hidden` | `true` marks a `report_hide` match — usually a hand-written clone of generated code. |

### Byte ranges, not line numbers

Deslop's source of truth is `[start_byte, end_byte)`. Line numbers are derived at render time for humans. Slice by byte range when you edit — line-based edits drift as soon as surrounding code moves.

### Cluster IDs are stable

A cluster ID is the first 8 bytes of the cluster's smallest-member BLAKE3 hash, rendered as 16 hex characters (e.g. `0362505641efe3c7`). It carries no timestamp, so the same repository analysed by the same binary twice produces the same IDs. Reference a cluster by ID across runs, in issues, and in your own notes — not by rank, which is a render-time position and moves as the repository changes.

## Configuring a repository

If you are setting Deslop up rather than consuming it, three things decide almost everything, and all are in the [Configuration reference](/docs/configuration/):

- **[`exclude` vs `report_hide`](/docs/configuration/#exclude-vs-report_hide--the-core-idea)** — `exclude` drops a file before analysis; `report_hide` analyses it but keeps it out of the headline, so "hand-written code duplicates generated code" still surfaces.
- **[Built-in rules](/docs/configuration/#built-in-rules-always-on)** — `node_modules`, `target`, `dist`, generated-code suffixes and banners are already covered. Do not re-add them.
- **[`[threshold]`](/docs/configuration/#threshold--the-ci-gate)** — the opt-in CI gate. Commit the ceiling so local runs, CI, and agents share one number.

To gate a build, use the [GitHub Action](/docs/github-action/); it wraps the same exit-code contract.

## Rules you do not get to break

- **Do not silence a finding to make it go away.** Widening the threshold, adding a `report_hide` pattern to bury your own code, or splitting a duplicate into trivially different shapes are all failures, not fixes.
- **Do not treat a flag as noise until you have shown it is noise.** If Deslop reports it, read both occurrences first.
- **Do not merge a `same_behavior` match blindly.** That bucket comes from semantic embeddings. Read both locations; the code looks different for a reason often enough to matter.
- **Some duplication is deliberate.** Test fixtures and bootstrapping code are the usual honest exceptions. Accepting a duplicate is a legitimate outcome — accepting it silently is not. Say which cluster you accepted and why.
- **Deslop does not rewrite your code.** It finds, ranks, compares, and prevents. The extraction is yours to write, and yours to get right.
