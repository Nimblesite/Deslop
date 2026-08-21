---
layout: layouts/blog.njk
title: Why Deslop parses source code with tree-sitter
date: 2026-04-10
author: Christian Findlay
tags: posts
description: Deslop parses source code with tree-sitter and compares normalized syntax trees so formatting and identifier changes do not hide structural duplicates.
excerpt: Deslop uses parsed syntax trees instead of source-text patterns. Here is what that enables and where language-specific normalization fits.
heroImage: "/assets/img/blog/tree-sitter-over-regex-header.png"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "Header image contrasting regex line matching with tree-sitter AST fingerprinting."
---

Deslop parses source files with tree-sitter and compares normalized syntax trees. The parser gives the detector stable structural input without relying on source-text regular expressions.

## What raw-text matching misses

Raw-text matching is sensitive to changes that do not alter program structure:

- **Formatting.** Two identical functions formatted differently look like different code.
- **Rename.** Changing `user` to `customer` across a method breaks every match.

## What tree-sitter lets us do

A tree-sitter parser produces an AST for every file in the repo. From that tree we can:

- normalize identifiers and literals to canonical placeholders, so renames collapse to the same fingerprint;
- hash subtrees independently, so the fingerprint of a method is stable regardless of where it lives in the file;
- operate on subtrees rather than lines, so formatting and whitespace are irrelevant;
- return exact byte ranges for each occurrence in the current source snapshot.

Normalization is language-specific because grammars represent identifiers, literals, and boilerplate differently. The normalized trees then enter the shared fingerprinting, clustering, and ranking pipeline.

## What this means for you

- **Rename refactors do not hide duplication.** A cluster survives an identifier rename because the fingerprint runs on the normalized AST.
- **Formatting changes do not alter structural fingerprints.** Reformatting a file with `rustfmt` does not change the parsed structure Deslop compares.
