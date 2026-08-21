---
layout: layouts/blog.njk
title: Why coding agents need fast duplicate-code feedback
date: 2026-04-20
author: Christian Findlay
tags: posts
description: Coding agents can reproduce logic that already exists elsewhere in a repository. Fast LSP and MCP feedback makes that duplication visible during the edit.
excerpt: Coding agents do not automatically know that similar code already exists elsewhere in a repository. This is why duplicate-code feedback belongs in the editing loop.
heroImage: "/assets/img/blog/ai-era-duplication-header.png"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "Header image showing duplicate-code growth outpacing feature review in AI-era repositories."
---

Coding agents can produce a plausible implementation without knowing that similar code already exists elsewhere in the repository. Repeating that pattern across feature work creates multiple implementations that must receive the same future fixes.

The problem is repository context, not intent. A model can generate another repository, validator, or mapper even when the project already contains one under a different name.

## Why feedback timing matters

A duplicate is cheaper to change while the code and its context are still active. Finding it only after CI or review adds another handoff before anyone can decide whether to extract, reuse, or accept it.

Deslop keeps analysis in the editing loop. The LSP watches the workspace and updates the report incrementally; the MCP server exposes that running analysis to a coding agent through `find-similar` and report queries.

## What to do with a finding

A cluster in a Deslop report is a decision, not a verdict. The tool reports; you decide. Broadly there are three paths:

- **Extract.** The fragments are identical enough, and share enough of a call graph, that a shared function is the clear answer. The `action_hints` in the JSON flag these.
- **Reuse.** One of the fragments is the "real" implementation and the others should call into it. Pick the one with the best tests and delete the others.
- **Accept.** Some duplication is intentional — test fixtures, bootstrapping, two things that look alike today but will diverge. Annotate and move on. Deslop does not judge; it just keeps score.
