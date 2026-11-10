---
layout: layouts/blog.njk
title: Why coding agents need fast duplicate-code feedback
date: 2026-04-20
updated: 2026-11-11
author: Christian Findlay
tags: posts
description: Coding agents can reproduce logic that already exists elsewhere in a repository. Fast LSP and MCP feedback makes that duplication visible during the edit.
excerpt: Coding agents do not automatically know that similar code already exists elsewhere in a repository. This is why duplicate-code feedback belongs in the editing loop.
heroImage: "/assets/img/blog/ai-era-duplication-header.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "Header image showing duplicate-code growth outpacing feature review in AI-era repositories."
ogImage: "/assets/img/blog/ai-era-duplication-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
---

Coding agents can produce a plausible implementation without knowing that similar code already exists elsewhere in the repository. Repeating that pattern across feature work creates multiple implementations that must receive the same future fixes.

The problem is repository context, not intent. A model can generate another repository, validator, or mapper even when the project already contains one under a different name.

## How AI agents end up writing the same code twice

Claude Code, Cursor, GitHub Copilot and Codex all work from the context they are given: the open files, the prompt, whatever the agent searched for, and whatever its tools return. A large repository rarely fits in that window. When the agent needs a date formatter or a retry wrapper, it has two options: search for an existing one, or write a new one. Writing is often the faster path to passing tests.

The result is rarely a verbatim copy. It is the same idea with new names: `parseOrderDate` next to an existing `toInvoiceDate`, a second HTTP client wrapper with a slightly different timeout, a validation rule rebuilt with its branches reordered. Each version compiles. Each passes its own tests. The duplicate code only matters later, when a bug fix or a security patch lands in one copy and not the other.

[Vibe coding](https://en.wikipedia.org/wiki/Vibe_coding), where developers accept generated code mostly on whether it seems to work, makes this more likely. The faster code is accepted, the less chance a reviewer has to recall that the same logic already lives three folders away.

## What the research shows

The risk is documented, not just anecdotal:

- [*Code Copycat Conundrum*](https://arxiv.org/abs/2504.12608) studies 19 code LLMs and reports repetition at character, statement and block level, including structurally redundant code.
- [*An Empirical Study of Code Clones from Commercial AI Code Generators*](https://conf.researchr.org/details/fse-2025/fse-2025-research-papers/111/An-Empirical-Study-of-Code-Clones-from-Commercial-AI-Code-Generators) (FSE 2025) measures Type-1 and Type-2 clone rates in the output of commercial code generators: exact and renamed copies, broadly what Deslop reports as **Identical code** and **Nearly identical code**.
- [*Debt Behind the AI Boom*](https://arxiv.org/abs/2603.28592) tracks static-analysis issues introduced by verified AI-authored commits in real repositories.

None of this means AI-written code is worse than human code. Humans copy and paste too. It means the volume of new code has gone up, and duplicate-code checks should keep pace with it. [Research Background](/docs/research-background/) collects the full reading list.

## Why feedback timing matters

A duplicate is cheaper to change while the code and its context are still active. Finding it only after CI or review adds another handoff before anyone can decide whether to extract, reuse, or accept it.

Consider the usual paths a copy can take:

| When the duplicate is found | What it costs |
| --- | --- |
| Before the agent writes it | One tool call. The agent reuses the existing function. |
| While the file is open | A quick edit. The author still has the context. |
| In AI code review or CI | A new round trip: comment, revise, re-run. |
| Months later | A bug fixed in one copy and missed in the other. |

The earlier rows are cheaper because nobody has to rebuild context. That is why Deslop puts its analysis in the editing loop instead of only in a nightly job.

## What Deslop does in the loop

Deslop keeps analysis in the editing loop. The LSP watches the workspace and updates the report incrementally; the MCP server exposes that running analysis to a coding agent through `find-similar` and report queries.

The [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) is how editors such as VS Code receive diagnostics. Deslop's language server holds the current report in memory and refreshes it as files change, so the cluster panel and diagnostics follow your edits.

The [Model Context Protocol](https://modelcontextprotocol.io/) is how AI agents call external tools. Deslop's MCP server reads the same live report rather than starting a fresh scan. The tool that matters most during authoring is `find-similar`: before writing new code, the agent passes in its proposed snippet and gets back any existing code with the same structure. Read-only queries such as `duplicates`, `cluster-by-id` and `compare-pair` cover review and cleanup. [AI Agents](/docs/ai-integration/) shows how to connect Claude Code, Codex, Claude Desktop and other MCP clients.

A practical agent instruction looks like this:

```text
Before writing a new function, call find-similar with the proposed code.
If it returns a match, reuse or extend the existing implementation.
After editing, call duplicates to confirm no new cluster appeared.
```

The CLI covers everything outside the editor: a one-off scan, a pre-merge check scoped to a diff with `--diff`, or a CI gate with `--fail-over`. The CLI, editor and MCP server run the same Rust engine, so a finding means the same thing in each.

## What to do with a finding

A cluster in a Deslop report is a decision, not a verdict. The tool reports; you decide. Broadly there are three paths:

- **Extract.** The copies are the same abstraction and will change together, so a shared function is the clear answer. Classic [refactoring techniques](https://martinfowler.com/books/refactoring.html) such as Extract Function apply directly. The JSON report includes `action_hints`, a short playbook for each category, so an agent can tell an identical copy from a looser match before it edits anything.
- **Reuse.** One of the copies is the "real" implementation and the others should call into it. Pick the one with the best tests and delete the others.
- **Accept.** Some duplication is intentional: test fixtures, bootstrapping, two things that look alike today but will diverge. Use [`report_hide` or `exclude`](/docs/configuration/#exclude-vs-report_hide--the-core-idea) to keep deliberate copies out of the headline. Deslop does not judge; it just keeps score.

A cluster that lists the copies, their exact locations and a category gives AI code review something specific to discuss instead of a vague "this looks duplicated" comment.

## Keep the signal small

An agent does not need a wall of clone data in its context. Deslop ranks clusters by [duplicated mass](/blog/ranking-formula/), so the largest copied code comes first, and the MCP tools return a short summary before any detail. The agent reads the top finding, decides, edits, and asks again. The report is already up to date when it does.

That is the point of fast duplicate-code feedback: keeping technical debt from building up while the code is being written, before anyone has to schedule a cleanup.
