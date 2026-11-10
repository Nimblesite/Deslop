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

## Find copied C error handling in VS Code

In a two-file example, an audio loader and a frame reader both allocate a buffer and read from a file. Deslop flags the similar error handling across the two files. Once you open the frame reader, there’s another repetition worth reviewing. The short-read branch and the normal close function both free the buffer, close the file, and free the wrapper.

Compare both paths in VS Code and check which resources each one owns. If a shared cleanup function fits, keep each caller’s logging and return value intact. Test the short-read path as well as a successful read, then scan again.

## What to expect from a scan

Repeated includes, function declarations, and ordinary loop headers are common in C. Deslop won’t report those on their own, and it does not compare macro bodies. If generated files dominate your report, [configure which findings appear](/docs/configuration/).

## Scan your C project

[Install Deslop](/docs/), then run this from the root of your project:

```bash
deslop .
```

Open the HTML report or use the VS Code extension to jump to each copy. Start with the highest-ranked finding and ask whether those copies should receive the same fixes.
