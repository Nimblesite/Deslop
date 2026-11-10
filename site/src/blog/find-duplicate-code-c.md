---
layout: layouts/blog.njk
title: "How to Find Duplicate Code in C: Functions and Headers"
date: 2026-11-08
author: Christian Findlay
tags:
  - posts
  - c
  - duplicate-code
  - ai-generated-code
category: engineering
description: "Find duplicate C functions and copied cleanup code across .c and .h files with tree-sitter analysis. Review matches in VS Code or from the command line."
excerpt: "When two C functions share a copied error path, a fix in one can leave the other leaking a file handle. Deslop now helps you find both copies."
heroImage: "/assets/img/blog/c-structural-dedup.webp"
heroImageWidth: "1600"
heroImageHeight: "840"
heroImageAlt: "Two matching graphite components become one silver component on a charcoal surface with a crimson accent."
ogImage: "/assets/img/blog/c-structural-dedup-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
ogImageAlt: "Duplicate graphite components consolidated into one silver component, in Deslop charcoal and crimson."
---

A bug fix in one C file can leave the same bug in another. Imagine two functions that each open a file, allocate a buffer, and read it. You fix a missing `fclose` on the first function’s short-read path. The other function still leaks a file handle.

Deslop now finds duplicate code across `.c` and `.h` files. It can find repeated functions and smaller blocks, even when someone has renamed variables or changed a constant.

## Why duplicate code is riskier in C

In most languages a copied function is a maintenance cost. In C it is often a correctness risk. C has no destructors, no `defer` and no garbage collector, so every function that acquires a resource has to release it by hand on every exit path. Error-handling blocks are long, repetitive and easy to copy. They are also where bugs hide, because the failure branches run least often in testing.

When the same open–allocate–read–cleanup sequence exists in several places, every fix to that sequence has to land everywhere:

- a missing `free` on an early return,
- a double `fclose` after a partial read,
- a length check that should be `<=` instead of `<`,
- a return code that one caller checks and another ignores.

Static analysis tools and sanitizers can catch some of these as they happen. A duplicate code detector answers a different question: *where else does this code exist?* Once you have fixed a bug, that is the question that decides whether the fix is complete.

## Find copied C error handling in VS Code

In a two-file example, an audio loader and a frame reader both allocate a buffer and read from a file. Deslop flags the similar error handling across the two files. Once you open the frame reader, there’s another repetition worth reviewing. The short-read branch and the normal close function both free the buffer, close the file, and free the wrapper.

Here is the kind of block that tends to repeat:

```c
if (fread(buf->data, 1, size, fp) != size) {
    free(buf->data);
    fclose(fp);
    free(buf);
    return NULL;
}
```

Compare both paths in VS Code and check which resources each one owns. If a shared cleanup function fits, keep each caller’s logging and return value intact. Test the short-read path as well as a successful read, then scan again.

The cluster panel lists every copy with its file and line range. **Compare To Canonical** opens a side-by-side view against the reference copy, and **Select for Compare** followed by **Compare with Selected** lets you pick any two copies yourself. The [cluster panel guide](/docs/vscode-cluster-panel/) walks through each action.

## What Deslop treats as the same C code

Deslop parses each file with the tree-sitter C grammar and compares normalized syntax trees, not lines of text. In C that means:

- **Renames do not hide a copy.** Function names, struct members, typedefs, `goto` labels and variable names are normalized, so `read_frame` and `load_audio` can still match.
- **Type swaps do not hide a copy.** Changing `int` to `long` in a copied function keeps its structure.
- **Changed constants do not hide a copy.** Numbers, characters and strings are normalized, so a copy with a different buffer size or error message still matches.
- **Comments and formatting are ignored.** Comments drop out of the C syntax tree, and whitespace and brace style never enter the comparison.

Matching structure only makes a pair a candidate. Deslop then checks the source content: consistent renaming keeps a copy, but calling a different function or performing a different operation is a different piece of code. Two loops that look alike but do different work are reported as **Same shape, different content**, which is informational and does not count towards duplication figures.

## What to expect from a scan

Repeated includes, function declarations, and ordinary loop headers are common in C. Deslop won’t report those on their own, and it does not compare macro bodies. If generated files dominate your report, [configure which findings appear](/docs/configuration/).

More precisely:

- **`#include` and `#define` lines are scaffolding.** They are never evidence of a clone. Conditional blocks such as `#ifdef` contain real code and are still compared.
- **Function prototypes are not clones.** Every short prototype has the same shape once names are normalized, so a header that lists three unrelated functions would otherwise match any other header. The function *body* is the evidence. A header made only of an include guard and prototypes reports nothing.
- **Function pointer tables are compared.** A declaration like `int (*on_open)(const char *, int);` is a variable, not a prototype, so a copied table of hooks is reported like any other copied code.
- **Macro bodies are not expanded.** The C grammar keeps a `#define` body as an opaque token, so two copied macros are a known blind spot rather than a source of false matches.
- **C++ is not yet supported.** `.h` files are analysed as C.

These rules keep normal C noise out of the report so the clusters that remain are copied code you can actually act on.

## Scan your C project

[Install Deslop](/docs/), then run this from the root of your project:

```bash
deslop .
```

Open the HTML report or use the VS Code extension to jump to each copy. Start with the highest-ranked finding and ask whether those copies should receive the same fixes.

Findings are ranked by [duplicated mass](/blog/ranking-formula/): the size of the copied syntax tree multiplied by its additional copies. A long copied error-handling block that appears four times ranks above a short helper that appears twice.

## Keep new copies out

Once the existing duplicates are triaged, the cheaper habit is catching new ones before they merge. Two options work well for C projects:

- **A CI gate.** The [GitHub Action](/docs/github-action/) runs the same CLI in a pull request. Set a [duplication threshold](/docs/configuration/#threshold--the-ci-gate) to fail the job when duplication rises, or measure without gating while you clean up.
- **A coding agent check.** If you use Claude Code, Codex or another MCP client on a C codebase, have the agent call Deslop's `find-similar` tool before it writes a new cleanup path. If a matching function exists, the agent can call it instead of adding another copy. [AI Agents](/docs/ai-integration/) shows the setup.

A copied cleanup path is easy to fix once you know where all its copies are.
