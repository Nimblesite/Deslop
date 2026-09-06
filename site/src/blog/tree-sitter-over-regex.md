---
layout: layouts/blog.njk
title: Regex on source code is illegal
date: 2026-04-10
author: Christian Findlay
tags: posts
description: Deslop parses every language with tree-sitter — no regex, no line-matching. Why that constraint matters, and how it survives reformatting and identifier renaming.
excerpt: Deslop parses every language with tree-sitter. No regex, no line-matching, no heuristics. Here's why that constraint matters more than any feature the tool ships with.
heroImage: "/assets/img/blog/tree-sitter-over-regex-header.png"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "Header image contrasting regex line matching with tree-sitter AST fingerprinting."
---

Most clone detectors you have used — CPD, Simian, jscpd — are fundamentally line-matchers. They take your source, tokenize or hash it by line, and find runs of matching lines. That approach has two features: it is fast, and it predates anyone writing a parser that is fast enough to not be the bottleneck. Tree-sitter changed the second fact. Deslop refuses to pretend otherwise.

## What line-matching misses

A line-matcher cannot see past:

- **Formatting.** Two identical functions formatted differently look like different code.
- **Rename.** Changing `user` to `customer` across a method breaks every match.
- **Reorder.** Swapping two independent statements produces zero overlap in the tokenizer's world.
- **Sugar.** LINQ versus `foreach`, `async/await` versus callbacks, list comprehension versus loop — all the same code, all invisible to a tokenizer.

You can patch around each individually. CPD normalizes whitespace. jscpd has mode toggles. Simian lets you configure what counts as a match. Every patch is a pile of heuristics that fail at the next edge case. The architecture does not support doing better.

## What tree-sitter lets us do

A tree-sitter parser produces an AST for every file in the repo. From that tree we can:

- normalize identifiers and literals to canonical placeholders, so renames collapse to the same fingerprint;
- hash subtrees independently, so the fingerprint of a method is stable regardless of where it lives in the file;
- operate on subtrees rather than lines, so formatting and whitespace are irrelevant;
- emit byte ranges that survive every kind of source transformation except semantic rewrite.

The pipeline this enables is linear, deterministic, and cheap. No heuristics. No per-language special cases beyond the grammar. Adding a language is: implement the `LanguageParser` trait, pin the grammar, done.

## Why "no regex" is written into the rulebook

The `CLAUDE.md` for this repo says it plainly: **regex on source code is prohibited.** Not "avoid," not "prefer parsers" — illegal. That rule exists because regex-on-source is a slippery slope. The first one handles a niche case a parser cannot easily express. The second one fixes a bug in the first. By the fifth, the codebase has a regex layer shadowing a parser layer and nobody can reason about which one fires first.

Tree-sitter is not a convenience in Deslop — it is the entire foundation. Every clone type the tool detects, every signal it fuses, every byte range it emits comes from the AST. Removing tree-sitter would not cost a feature; it would leave no tool behind.

## What this means for you

- **Rename refactors do not hide duplication.** A cluster survives an identifier rename because the fingerprint runs on the normalized AST.
- **Formatting changes do not create false positives.** Reformatting a file with `rustfmt` does not change what Deslop sees.
- **Language parity is real.** The same fingerprinting logic runs on C#, Rust, Python, Dart, JavaScript, TypeScript, and every language added later. Cross-language comparisons (when they make sense) use the same math.

Line-matching is a 1990s compromise with hardware that no longer exists. Tree-sitter is the upgrade. Deslop ships the upgrade as the baseline, not a premium tier.
