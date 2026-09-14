---
layout: layouts/blog.njk
title: "AI-Generated Code and Duplicate Code: What to Check"
date: 2026-04-23
author: Christian Findlay
tags:
  - posts
  - ai-generated-code
  - technical-debt
  - duplicate-code
category: engineering
description: "AI-generated code can create duplicate code and technical debt. Learn what to check with code clone detection and how Deslop audits AI-era codebases."
excerpt: "AI-generated code can multiply duplicate logic before review catches it. This guide explains what to check, why duplicate code becomes technical debt, and where to read the full Deslop research background."
heroImage: "/assets/img/blog/ai-generated-code-duplicate-code-header.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "Header image showing a four-level duplicate-code checklist for AI-generated code."
ogImage: "/assets/img/blog/ai-generated-code-duplicate-code-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
---

AI does not have to generate broken code to make a codebase harder to maintain. It only has to generate the same idea twice, in two slightly different shapes, before anyone notices.

That is the duplicate-code problem in the AI era. The code may compile. The tests may pass. The pull request may look reasonable. But the repository now has two implementations that need the same future fix.

For the full implementation map, see the Deslop docs page: [Research Background](/docs/research-background/). This post is the shorter version for teams trying to decide what to check first.

## Does AI-generated code create technical debt?

It can. The risk is not magic, and it is not unique to AI. Humans have copied code for decades. The change is throughput.

AI coding assistants can produce a plausible repository-shaped answer quickly. When the prompt is similar to a previous task, the answer often has a familiar shape too: another repository class, another validation function, another mapper, another retry wrapper, another test fixture. That is useful in the moment and expensive later.

The research direction is moving the same way:

- [Code Copycat Conundrum](https://arxiv.org/abs/2504.12608) studies repetition in LLM-generated code across character, statement, and block levels.
- [An Empirical Study of Code Clones from Commercial AI Code Generators](https://conf.researchr.org/details/fse-2025/fse-2025-research-papers/111/An-Empirical-Study-of-Code-Clones-from-Commercial-AI-Code-Generators) reports measurable Type-1 and Type-2 clone rates from studied commercial code generators.
- [Debt Behind the AI Boom](https://arxiv.org/abs/2603.28592) studies technical debt introduced by AI-authored commits in production repositories.

None of that means every AI-written line is bad. It means AI-generated code should receive the same repository-level duplication checks as human code.

## What should a duplicate-code check look for?

A useful AI-era duplicate-code check should not stop at exact line matches. It should look for four levels of similarity:

1. **Exact duplicate code**: the same code copied with formatting or comment changes.
2. **Renamed duplicate code**: the same structure with different variable names or constants.
3. **Near-duplicate code**: mostly the same logic with inserted, deleted, or reordered statements.
4. **Same behavior, different code**: two implementations that solve the same problem with different syntax.

Classic code clone detection research calls those Type-1, Type-2, Type-3, and Type-4 clones. Deslop uses those ideas, but the detailed algorithm write-up lives in [Research Background](/docs/research-background/) so this post does not repeat the whole docs page.

## Why line matching is not enough

Line-based duplicate-code tools are good at finding obvious copy-paste. They are weaker when AI changes the surface shape:

- `customerId` becomes `accountId`.
- `foreach` becomes a comprehension.
- a helper is copied but moved into a different class.
- the same validation rule is rewritten with a different branch order.

That is why Deslop starts from parsed syntax trees rather than raw text. It parses each file with tree-sitter, then strips out identifier and literal names so renamed copies still match. It fingerprints the tree structure, widens the net to near-duplicates with sibling windows and MinHash, and can optionally add embeddings for same-behavior matches. The short version: it compares code structure first, not lines first.

The full audit trail is in [How It Works](/docs/how-it-works/) and [Research Background](/docs/research-background/).

## What to do when you find duplicate code

Do not treat every clone as a bug. Treat it as a decision.

**Extract** when the copies are clearly the same abstraction and will change together.

**Reuse** when one implementation is already the better source of truth and the others should call it.

**Accept** when duplication is deliberate: fixtures, generated code, compatibility shims, or two paths that look alike now but are expected to diverge.

The mistake is not accepting duplication. The mistake is accepting it accidentally because no one measured it.

## Why Deslop ranks findings

Deslop ranks clusters by impact so higher-impact findings appear first.

That matters for AI coding agents. An agent does not need a wall of clone data. It needs a small, structured answer:

- which duplicate cluster matters most,
- where the byte ranges are,
- why the cluster was flagged,
- whether the signal came from structure, token similarity, or embeddings.

That is why Deslop is JSON-first and why the LSP/MCP path exists: the same structured findings are available to the editor and the coding agent close to the edit.
