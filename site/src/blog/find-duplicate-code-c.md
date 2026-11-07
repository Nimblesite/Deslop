---
layout: layouts/blog.njk
title: "Find Duplicate Code in C: Deslop Adds C Support"
date: 2026-11-08
author: Christian Findlay
tags:
  - posts
  - c
  - duplicate-code
  - ai-generated-code
category: engineering
description: "Find duplicate code in C with tree-sitter structural analysis: renamed functions, copied error-path ladders and cleanup tails, ranked worst first, with live LSP and MCP feedback."
excerpt: "Deslop now understands C. It parses .c and .h syntax trees, sees through renamed identifiers and swapped types, treats macros and prototypes as scaffolding, and reports the copied error-path ladders that hand-written runtimes grow."
heroImage: "/assets/img/blog/find-duplicate-code-c.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "The C language icon beside the words C gets structural eyes, on Deslop's language spotlight card."
ogImage: "/assets/img/blog/find-duplicate-code-c-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
ogImageAlt: "Deslop language spotlight card announcing duplicate detection for .c and .h files."
---

C support runs through the same structural pipeline as every other Deslop language: tree-sitter parsing instead of regex, identifier and literal normalization instead of raw line matching, worst-offender-first ranking, and live LSP and MCP feedback while the code is changing. `.c` and `.h` files are discovered, parsed with [`tree-sitter-c`](https://github.com/tree-sitter/tree-sitter-c), and fed into the shared fingerprint, sibling-window, token and ranking stages described in [How It Works](/docs/how-it-works/).

## Why C, and why now

The [Osprey](/docs/osprey/) compiler ships a hand-written C runtime: around 190 files and 46,000 lines, compiled with warnings as errors. Its maintainers wanted the same duplication gate over that runtime that they already had over the Osprey and Python sources, and they named the shape they cared about most: copied error-path and cleanup blocks. A loader opens a file, allocates a buffer, reads it, and at each step has an `if (x == NULL) { fprintf(...); free(...); return NULL; }` rung. The next loader is written by copying the first. One of them later gets the fix.

The first scan of that runtime measured 7.5% duplication, mostly repeated test set-up, and it also measured two false positives that are now pinned by fixtures. Both are described below, because what a detector refuses to report matters as much as what it finds.

## What the C parser sees

The normalized tree keeps structure and drops spelling:

- `identifier`, `field_identifier`, `type_identifier`, `statement_identifier` and `primitive_type` collapse to one marker, so renaming a function, a struct member or a typedef, or swapping `int` for `long`, never hides a copy;
- numbers, characters, string contents, escape sequences, `<system.h>` paths, `true`, `false` and `NULL` collapse to one literal marker, while string and character wrappers stay structural so concatenated strings keep their own shape;
- comments disappear;
- pointer, array and function declarators, casts, `sizeof`, compound literals, initializer lists, designators, storage classes, type qualifiers and every statement form remain structural.

A copied function with every local renamed and a buffer size changed is still the same tree.

## What the C parser refuses to count

**Macros.** The grammar never sees what a macro expands to. A `#define` body is one opaque leaf, so two unrelated macros would fingerprint identically. `#include`, `#define` and function-like macro definitions are therefore scaffolding, like `using` in C# or `import` in Python: never clone evidence, never a cluster on their own. Six files that share the same include block and constants and nothing else report no clusters. Duplicated macro bodies are a documented false negative, not a tolerated false positive.

**Prototypes.** After normalization every one-argument prototype is the same shape. A header declaring `sqrt`, `exp` and `log` would otherwise match any header declaring three unrelated functions, and Osprey's `math.h` shim did exactly that against an unrelated test header. A prototype is a promise about a body that lives elsewhere; the body is the evidence. A header made of an include guard and prototypes reports nothing.

**Loop headers.** In C the initializer, condition and update of a `for` sit directly beside the body, so a run of neighbouring nodes can be just `for (int i = 0; i < n; i++)`. That is an idiom every loop shares, not a copy. Deslop no longer forms a sibling window over a flat `for` header in any language that lays it out that way.

## What it does report

The fixture written for Osprey's request holds two loaders in two files. Each opens a handle, allocates, reads, and checks every step with a copied log-free-return rung. Deslop reports the shared rung across the two files as **Nearly identical code**, and inside one file it reports the short-read error path and the destructor releasing the same resources in the same order as **Identical code**. Those are the two findings a reviewer wants: the copy that will drift between files, and the cleanup sequence that should be one function.

## Prevention beats the cleanup sprint

Deslop's MCP server gives coding agents a `find-similar` tool backed by the live workspace. Before an agent writes another loader, another error ladder or another test fixture, it submits the proposed shape and asks whether the repository already contains it. A strong match turns "write a new implementation" into "reuse or extend the canonical one". See [AI Integration](/docs/ai-integration/) for the MCP setup and [For AI](/docs/for-ai/) for the agent workflow.

## Generated C

Generated parsers are the biggest C files in many repositories, and the tree-sitter parser generator writes no "generated" banner, so Deslop analyses them like any other source. Osprey's own generated parser is 86,000 lines and 29% self-similar, enough on its own to breach a repository gate. The right answer is [`report_hide`](/docs/configuration/) for that directory, not `exclude`: the generated code stays in the analysis, so a hand-written file that duplicates it is still reported, while the generated file duplicating itself stays out of the headline number.

## Try it on a C repository

Install Deslop from the [Getting Started guide](/docs/), open a C workspace, and run:

```bash
deslop .
```

The CLI writes JSON, text and HTML reports. The VS Code extension adds live findings and a worst-first cluster view. Start with the first cluster: that is the duplicate with the largest measured impact, not merely the first one the scanner happened to encounter.
