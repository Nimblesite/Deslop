---
layout: layouts/blog.njk
title: "MCP for Coding Agents: Stop Duplicate Code Before It Lands"
date: 2026-06-12
updated: 2026-06-12
author: Christian Findlay
tags:
  - posts
  - mcp
  - lsp
  - ai-coding-agents
  - duplicate-code
category: engineering
description: "How Deslop's live MCP + LSP server lets coding agents call find-similar before writing code, preventing duplicate code while the repo is still changing."
keywords: "Model Context Protocol, MCP server, AI coding tools, coding agent, duplicate code, code duplication, duplicate-code analysis, technical debt, AI-generated code, LSP server, find-similar"
excerpt: "MCP gives coding agents tools. Deslop gives them live repository memory: call find-similar before writing, then reuse the code that already exists."
heroImage: "/assets/img/blog/live-mcp-lsp-duplicate-code-prevention-header.png"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "Header image showing a live MCP and LSP sidecar intercepting duplicate code before it reaches a repository."
ogImage: "/assets/img/blog/live-mcp-lsp-duplicate-code-prevention-og.png"
ogImageWidth: "1200"
ogImageHeight: "630"
ogImageAlt: "Deslop live MCP and LSP sidecar preventing duplicate code before it lands."
---

By the time duplicate code shows up in CI, the agent has already written it, the pull request already contains it, and a reviewer has to ask why the same rule now exists in two places.

Deslop moves that check earlier. When a coding agent is about to add a mapper, validator, widget, or test helper, it can ask the live workspace whether that shape already exists.

## MCP is now the agent tool layer

The Model Context Protocol gives AI coding tools a standard way to reach outside the model. That is useful, but MCP alone does not make a tool trustworthy. A stale duplicate-code report is still stale, even when a coding agent can call it through an MCP server.

Deslop's use of MCP is deliberately specific: expose live duplicate-code analysis to the agent at the moment it is deciding what to write. That turns duplicate code from an after-the-fact technical debt report into a prevention loop.

That matches the protocol landscape. The official [Model Context Protocol introduction](https://modelcontextprotocol.io/docs/getting-started/intro) defines MCP as an open standard for connecting AI applications to external systems. The [MCP tools specification](https://modelcontextprotocol.io/specification/2025-06-18/server/tools) says tools let models invoke external systems. VS Code's own documentation now explains how to [add and manage MCP servers](https://code.visualstudio.com/docs/agent-customization/mcp-servers), and OpenAI's Agents SDK has [MCP support](https://openai.github.io/openai-agents-python/mcp/) for connecting agents to MCP servers.

For Deslop, the product question is simple: can the agent check the live repo before it writes the second copy?

## MCP alone is not repository memory

An MCP server can expose a tool. That does not mean the tool is fresh, cheap, or safe to call often.

For duplicate-code prevention, freshness is the entire product. An AI coding agent changes files quickly. A human changes another file in the editor. A formatter runs. A test generator adds a fixture. A one-shot CLI report taken at the start of the session is already stale after the first edit.

Deslop's architecture is deliberately split:

- The **LSP server** lives with the editor, watches the workspace, and keeps the current duplicate-code report hot.
- The **MCP server** exposes agent-facing tools and delegates reads and compute calls to that running LSP process.
- The **CLI** remains the cold-cache fallback for CI gates and one-shot audits.

That split follows the shape of the two protocols. The [Language Server Protocol specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/) standardizes JSON-RPC messages between editors and language servers. VS Code's [language server guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide) describes language servers as the way to power editor features like diagnostics, jump-to-definition, and language intelligence across clients. MCP, by contrast, is the agent's tool interface.

LSP is how the human sees the duplicate. MCP is how the agent avoids writing it.

## The new loop: ask before writing

The keystone Deslop tool is `find-similar`.

The rule is simple:

```text
Before writing a new function, class, helper, widget, mapper, or test setup,
call find-similar with the snippet or range you are about to create.
If a high-similarity cluster already exists, reuse the canonical occurrence
instead of creating another copy.
```

That sounds small. It is not. It moves duplicate-code detection from "audit the mess later" to "query the workspace before the copy exists."

The loop looks like this:

1. The agent plans a new block of code.
2. It calls Deslop's MCP `find-similar` tool with either a file range or an in-memory snippet plus language.
3. Deslop parses the candidate with [tree-sitter](https://tree-sitter.github.io/tree-sitter/), normalizes identifiers and literals, fingerprints the structure, probes the live index, and returns matching clusters.
4. The agent either reuses the canonical implementation, extracts shared logic, or writes the new code because no meaningful match exists.
5. The LSP watcher sees the resulting file change and updates the editor surfaces and MCP report generation.

The point is not to make the agent timid. The point is to give it the same repo-level memory a careful maintainer has, at the exact moment that memory matters.

## Why this matters for AI-generated code

AI-generated code does not have to be broken to become expensive. It only has to repeat a business rule in two places.

The risk is now visible in the public literature. GitClear's [2025 AI Copilot Code Quality study](https://www.gitclear.com/ai_assistant_code_quality_2025_research) reports a rise in duplicate code blocks and a decline in moved lines, the signal that code was consolidated rather than copied. The [Code Copycat Conundrum](https://arxiv.org/abs/2504.12608) studies repetition in LLM-generated code across character, statement, and block levels. Google's DORA team frames AI-assisted development as an amplifier in the [State of AI-assisted Software Development 2025 report](https://dora.dev/dora-report-2025/): teams get the strongest results when the underlying engineering system is already healthy.

That is the operational lesson. Do not ask a coding agent to "be careful" in prose and hope it remembers. Put the repository memory in the tool loop.

## What Deslop returns to the agent

An agent cannot use a vague warning like "there might be duplication." It needs structured data with stable identifiers and a bounded context cost.

Deslop's MCP surface returns duplicate clusters worst-first, with the fields an agent can act on:

- a stable cluster id,
- the clone bucket, such as identical code or nearly identical code,
- the ranking score,
- occurrence paths and byte ranges,
- structural, token, and embedding signals where available,
- an interpretation string,
- action hints.

The report is deliberately JSON-first. The human-readable text and HTML views are renderings of the same schema, not a second truth. The docs page on [Output Formats](/docs/configuration/#report-output) explains the consumer contract, and [How It Works](/docs/how-it-works/) explains the parse, normalize, fingerprint, cluster, and rank pipeline.

The MCP tools also carry an occurrence budget. Real repositories can have clusters with dozens of locations, and dumping every occurrence into an agent context window is not helpful. Deslop tools such as `top-offenders`, `report-for-file`, `report-for-range`, and `find-similar` accept a `max_occurrences` budget so the result stays useful instead of becoming a transcript flood. When an agent truly needs the full cluster, it can follow up with `cluster-by-id`.

That small constraint matters. Agent tools should be designed for the agent's context budget, not merely exposed because the protocol allows it.

## Why the MCP delegates to the LSP

The tempting design is to let the MCP server scan the repository itself. That looks simpler until you care about correctness.

If the LSP and MCP each run their own analysis, you now have two watchers, two caches, two reports, and two chances to disagree. If the MCP reads an old file report from disk, the agent can act on a cluster the editor has already removed. If the MCP starts a cold scan on every tool call, the agent stops calling it because it is too slow.

Deslop uses one live engine. The LSP owns the analysis session and watches the workspace. The MCP asks the running LSP for the current report over local IPC. The project specs describe this in [the MCP shell design](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/mcp.md) and [the live analysis design](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/live.md): the MCP is a transport adapter, not a second analysis engine.

That is the difference between a tool that can answer and a tool an agent can safely call mid-generation.

## What changed for real-world workspaces

The live path has picked up several features that matter when coding agents are not just demos.

**External MCP clients use the VSIX-bundled binary.** Claude Code, Claude Desktop, Cursor, Continue, Codex, and other external MCP clients should point at the `deslop-mcp` binary bundled inside the installed VS Code extension, unless the user installed the CLI through Homebrew or Scoop. That avoids a local `target/release` binary drifting away from the extension's wire contract. The setup path is in [AI Integration](/docs/ai-integration/).

**Windows no longer needs Unix sockets.** On Unix-like systems, the LSP exposes a local Unix socket. On Windows, Deslop uses a token-gated TCP loopback endpoint published in `.deslop/cache/deslop.port`, so the MCP and LSP still speak the same JSON-RPC protocol without pretending Windows has Unix sockets. The implementation is described in the live spec's [TCP loopback transport](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/live.md#live-ipc-tcp).

**The editor and the agent share the same report.** The VSIX surfaces clusters through the editor UI, while MCP tools expose the same live data to the agent. The [VS Code cluster panel docs](/docs/vscode-cluster-panel/) cover the human side. The MCP tools cover the agent side. The important property is that both are reading the same engine.

## What to put in your agent instructions

If your repo uses Deslop, the best instruction is direct and mechanical:

```text
Before writing any new function, class, helper, widget, mapper, repository,
or test setup, call Deslop's find-similar MCP tool. If a high-similarity
cluster exists, reuse the canonical implementation instead of creating
another copy.
```

That instruction is stronger than "avoid duplication." It names the tool, the timing, and the expected action.

For teams that already maintain an `AGENTS.md`, `CLAUDE.md`, or equivalent coding-agent instruction file, this is where Deslop belongs. The tool is most valuable before the agent writes code, not after a reviewer asks why the same validation rule appears in four files.
