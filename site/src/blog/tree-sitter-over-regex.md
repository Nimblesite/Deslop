---
layout: layouts/blog.njk
title: Why Deslop parses source code with tree-sitter
date: 2026-04-10
author: Christian Findlay
tags: posts
description: Deslop parses source code with tree-sitter and compares normalized syntax trees so formatting and identifier changes do not hide structural duplicates.
excerpt: Deslop uses parsed syntax trees instead of source-text patterns. Here is what that enables and where language-specific normalization fits.
heroImage: "/assets/img/blog/tree-sitter-over-regex-header.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "Header image contrasting regex line matching with tree-sitter AST fingerprinting."
ogImage: "/assets/img/blog/tree-sitter-over-regex-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
---

Deslop parses source files with tree-sitter and compares normalized syntax trees. The parser gives the detector stable structural input without relying on source-text regular expressions.

That one design decision shapes everything downstream: what counts as a copy, which edits hide a copy, how exact the reported locations are, and how quickly the report can refresh while you type. This post explains the decision, what it buys, and where its limits are.

## What raw-text matching misses

Raw-text matching is sensitive to changes that do not alter program structure:

- **Formatting.** Two identical functions formatted differently look like different code.
- **Rename.** Changing `user` to `customer` across a method breaks every match.
- **Position.** A copied function moved to another file, or further down the same file, no longer sits on the lines a line-window comparison expects.
- **Line wrapping.** A long argument list split across three lines no longer lines up with the one-line original.

Each of these is a cosmetic change. None of them changes what the code does, and none of them makes the copy any less of a maintenance problem. A detector that misses them misses the copies people actually leave behind: the ones that were pasted, tidied by a formatter, and renamed to fit their new home.

Regular expressions make this worse rather than better. A pattern that tries to recognise a function by its text has to guess where the function ends, which braces are inside strings, and which tokens are comments. Every guess is a place where a real duplicate can slip through or two unrelated blocks can be glued together. Deslop treats that as an accuracy problem, not a tuning problem, so it does not use text patterns on source code at all.

## What tree-sitter is

[Tree-sitter](https://tree-sitter.github.io/tree-sitter/) is an open-source parser generator and incremental parsing library. Each supported language has a grammar that produces a concrete syntax tree for a source file. The project's own documentation describes it as fast enough to parse on every keystroke and robust enough to return a useful tree even when the file contains syntax errors. GitHub [uses tree-sitter](https://docs.github.com/en/repositories/working-with-files/using-files/navigating-code-on-github) for its code navigation.

For a duplicate code detector, three properties matter:

1. **One tree per file, with exact positions.** Every node knows its byte range, so a finding can point at the exact start and end of each copy.
2. **The same parser everywhere.** The CLI, the VS Code extension and the MCP server all run the same Rust engine over the same grammars, so a finding in a pull request matches the one in your editor.
3. **Error tolerance.** Half-written code still parses into a tree, which matters for live analysis while a developer or an AI coding agent is mid-edit.

## What tree-sitter lets us do

A tree-sitter parser produces an AST for every file in the repo. From that tree we can:

- normalize identifiers and literals to canonical placeholders, so renames collapse to the same fingerprint;
- hash subtrees independently, so the fingerprint of a method is stable regardless of where it lives in the file;
- operate on subtrees rather than lines, so formatting and whitespace are irrelevant;
- return exact byte ranges for each occurrence in the current source snapshot.

Normalization is language-specific because grammars represent identifiers, literals, and boilerplate differently. The normalized trees then enter the shared fingerprinting, clustering, and ranking pipeline.

Today that covers C, C#, Rust, Python, Dart, JavaScript, TypeScript/TSX, PHP, F#, Go and Osprey. Adding a language means adding its grammar and its normalization rules; the rest of the pipeline is shared.

## A small example

Here are two Python functions written weeks apart:

```python
def total_for_user(user, orders):
    total = 0
    for order in orders:
        if order.owner == user.id:
            total += order.amount
    return total
```

```python
def sum_customer_spend(customer, purchases):
    spend = 0
    for purchase in purchases:
        if purchase.owner == customer.id:
            spend += purchase.amount
    return spend
```

A line-based diff sees six different lines. After normalization, both functions have the same shape: a function with two parameters, an accumulator set to a literal, a loop, a comparison and an addition. Their structural fingerprints match.

Matching structure only makes the pair a candidate. Deslop then checks the source content. Here the names change consistently (`user` → `customer`, `orders` → `purchases`) and both functions do the same work, so a pair like this is reported as **Nearly identical code** once it clears the minimum size described below. If the second function multiplied instead of adding, or called a different method on its collaborator, the trees could still look alike, but the content check would see a different operation and reject the pair as a clone. Structure finds candidates; content decides what Deslop reports.

## Why normalization needs a second check

Over-normalizing is how a structural detector produces false positives. If every identifier, literal and operator collapses to a placeholder, many unrelated functions start to look the same: every short loop with an accumulator, every getter, every guard clause.

Deslop handles this in two ways. First, small subtrees never become clone candidates: the default minimum is 30 syntax-tree nodes, configurable with `--min-nodes`. Second, candidates that match in layout but share almost no content are labelled **Same shape, different content**. That label is informational. It adds nothing to clone counts, duplicated mass or the duplication percentage, so a repeated control-flow skeleton does not inflate your technical debt figures. The [category guide](/docs/how-it-works/#read-the-categories) lists every label.

## What this means for you

- **Rename refactors do not hide duplication.** A cluster survives an identifier rename because the fingerprint runs on the normalized AST.
- **Formatting changes do not alter structural fingerprints.** Reformatting a file with `rustfmt` does not change the parsed structure Deslop compares.
- **Locations are exact.** Each occurrence is a byte range in the current file, so the VS Code panel, the HTML report and an AI agent's tool call all point at the same code.
- **Moving code does not hide it.** Subtree fingerprints do not depend on where the code sits, so a copy moved into another file or class still matches.

## Where tree-sitter stops

A parser can only describe what is in the file. In C, the grammar does not expand macros: a `#define` body is an opaque leaf. Deslop therefore does not compare macro bodies, and copied macros are a known false negative rather than a source of false matches. Generated code is parsed like any other file. Deslop hides common generated-file patterns and banners by default, and you can [configure more](/docs/configuration/).

Equivalent behaviour written with different syntax, such as a loop versus a `map` call, may not share a tree shape at all. Optional embeddings can surface those as **Same behavior, different code** candidates. That label asks you to review both versions; it does not prove they are interchangeable.

## Structure first, for clean code reviews

Copied code is a [code smell](https://martinfowler.com/bliki/CodeSmell.html), not automatically a bug. Parsing gives the review a solid starting point: the same logic, found regardless of names and formatting, with exact locations and a clear category. From there the decision is yours: extract, reuse, or keep the copies on purpose. [How It Works](/docs/how-it-works/) covers the full pipeline, and [Research Background](/docs/research-background/) lists the papers behind each stage.
