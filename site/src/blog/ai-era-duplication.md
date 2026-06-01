---
layout: layouts/blog.njk
title: Duplication is the tax LLMs charge for speed
date: 2026-04-20
author: Christian Findlay
tags: posts
description: Coding agents duplicate code faster than humans can review it. Why Deslop's live LSP + MCP server treats duplication as the defining problem of the AI era.
excerpt: Coding agents generate plausible code faster than humans can review it. They also duplicate it faster. Here's why Deslop treats that as the defining problem of the era, not a side-effect.
---

Every coding agent I have worked with has the same pathology. Given a request that resembles a request it has fulfilled before, it will cheerfully reach for the same shape of code — even when that shape already exists in the repo under a different name. Multiply that instinct across a team of three engineers and four agents, and you arrive at the condition that defines the current era of software: **repositories where the duplication rate grows faster than the feature rate**.

This is not a story about sloppy agents. The pattern-matching that makes an LLM useful is the same pattern-matching that makes it duplicate. A transformer does not know your repo has a `UserRepository` already. It knows that the training distribution contains a shape called "repository," and it reproduces that shape. Three times, in three files, under three slightly-different names.

## Why existing tools are not the answer

The clone-detection field has thirty years of literature and a handful of production tools — CPD, Simian, jscpd, Sonar CPD. They all share two assumptions that no longer hold:

1. **Duplication is an occasional bug.** These tools surface duplication as a list, sorted by discovery order or file name. That worked when the occasional duplication was hand-crafted. It does not work when duplication is the default state of the repo.
2. **Humans are the primary reader.** Output formats assume a developer will squint at a table, pick a cluster to investigate, and clean it up at their leisure. An agent cannot squint. An agent needs byte ranges, stable IDs, and a schema.

Deslop rebuilds from both assumptions. Output is ranked by the weighted impact of each cluster so the top row is always where the largest payoff lives. Output is JSON first, with text and HTML as views over the same schema. The audience is dual: the human who installs it and the agent who queries it.

## Fast feedback is the entire product

The feature that matters most is not the breadth of languages, not the precision of same-behavior detection (Type-4), not the cleverness of the fusion. It is **time to first useful signal**. A duplicate that surfaces three commits after it lands is a duplicate you will not refactor. A duplicate that surfaces while the agent is still holding the file open is a duplicate you fix before the next message.

So the entire pipeline is tuned for that. The cache is keyed so unchanged files are free, so a warm pass only re-parses the files you just touched. The ranking is cheap — two multiplications and a logarithm per cluster. The LSP shell ships today and lights duplication up in the editor at the speed of a spellchecker; the MCP shell exposes the same live analysis to Claude, Cursor, and Copilot before the agent even types the duplicate.

Speed is not a feature of Deslop. Speed is the whole point.

## What to do with a finding

A cluster in a Deslop report is a decision, not a verdict. The tool reports; you decide. Broadly there are three paths:

- **Extract.** The fragments are identical enough, and share enough of a call graph, that a shared function is the clear answer. The `action_hints` in the JSON flag these.
- **Reuse.** One of the fragments is the "real" implementation and the others should call into it. Pick the one with the best tests and delete the others.
- **Accept.** Some duplication is intentional — test fixtures, bootstrapping, two things that look alike today but will diverge. Annotate and move on. Deslop does not judge; it just keeps score.

The only wrong move is to ignore the top of the report. That is where the money is.

## Where this goes

Deslop today is the live server. Two cooperating processes — a file watcher and LSP shell in one, an MCP shell in the other, talking over a local IPC socket — plus twelve MCP tools the agent can call mid-generation. Same pipeline, same schema, same cache as the CLI — the CLI is now the cold-cache fallback for CI gates. The VS Code extension bundles all of it (LSP, MCP, CLI) in a single VSIX. JetBrains is next.

The primary user of the server is not you. It is the agent you are pair-programming with. Which is as it should be: agents generate duplication; agents should fix it — and, with `find-similar` in their inner loop, agents should prevent it.

Install today. Open your messiest repo. Read line one.
