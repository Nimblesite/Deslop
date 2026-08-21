---
layout: layouts/blog.njk
title: "Finding Duplicate Dart Code in Flutter Projects"
date: 2026-06-05
author: Christian Findlay
tags:
  - posts
  - dart
  - flutter
  - ai-generated-code
  - duplicate-code
category: engineering
description: "How structural analysis finds renamed and near-duplicate Dart code in Flutter projects, and how coding agents can check before writing another copy."
excerpt: "Flutter projects repeat widget trees, repositories, mappers, and test setup. Here is how Deslop compares their structure and reports the highest-impact duplicates first."
heroImage: "/assets/img/blog/deduplicating-dart-code-ai-flutter-header.png"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "Header image showing Flutter widget trees, cloned Dart cards, and a find-similar gate."
ogImage: "/assets/img/blog/deduplicating-dart-code-ai-flutter-og.png"
ogImageWidth: "1200"
ogImageHeight: "630"
ogImageAlt: "Deslop — deduplicating Dart code when AI writes your Flutter app. Live LSP and MCP duplicate-code server, worst-offenders ranked."
---

AI-assisted Flutter work can reproduce the same widget, repository, or validation rule in slightly different forms. The code can compile and pass tests while leaving multiple implementations that need the same future fix.

Flutter widget trees and `build` methods can be long, so partially duplicated structure does not always stand out in review.

For the full algorithm write-up, see [Research Background](/docs/research-background/). This post is the Dart-and-Flutter-specific version.

## AI-assisted code and duplication

GitClear's [2025 AI Copilot Code Quality study](https://www.gitclear.com/ai_assistant_code_quality_2025_research) reports increased copy/pasted lines and duplicated blocks in the repositories it studied. [Code Copycat Conundrum](https://arxiv.org/abs/2504.12608) examines repetition in LLM-generated code at character, statement, and block levels.

This does not make AI-written Dart inherently bad. It does make repository-level duplicate checks useful during AI-assisted work.

## Where Flutter duplication commonly appears

Common candidates include:

- widget trees and repeated layout fragments inside `build` methods;
- repositories, data mappers, and validation paths that differ mainly by names;
- `copyWith` methods, retry wrappers, and repeated `*_test.dart` setup.

## What a Dart duplicate-code check should look for

A useful check does not stop at exact line matches. It should find four levels of similarity:

1. **Exact duplicate code** — the same Dart copied with only formatting or comment changes.
2. **Renamed duplicate code** — the same structure with different identifiers: a `CustomerCard` widget cloned into `AccountCard`, `customerId` swapped for `accountId`.
3. **Near-duplicate code** — mostly the same logic with statements inserted, deleted, or reordered: the same form validation with one extra branch.
4. **Same behaviour, different code** — two widgets or functions that solve the same problem with different syntax (a `for` loop versus a `map().toList()`).

Classic clone-detection research calls these Type-1 through Type-4. Deslop's implementation and research references are documented in [Research Background](/docs/research-background/).

## Why line matching is not enough for Dart

Line-based tools catch literal copy-paste but are sensitive to surface changes:

- `CustomerCard` becomes `AccountCard`, every identifier renamed.
- a helper is copied into a different class and re-indented.
- `setState` logic is rewritten as a `Riverpod` notifier that does the same thing.
- the same validation rule is rebuilt with the branches in a different order.

That is why Deslop starts from the parsed syntax tree, not the text. It parses each `.dart` file with **tree-sitter**, strips out identifier and literal names so renamed copies still match, fingerprints the tree *structure*, widens the net to near-duplicates with sibling windows and MinHash, and can optionally add embeddings for same-behaviour matches. The short version: it compares structure first, lines never. The full audit trail is in [How It Works](/docs/how-it-works/).

## Check before writing

Deslop's MCP server can move the check into the coding agent's editing loop.

If you use a coding agent for Flutter work, configure it to call Deslop's `find-similar` tool **before it writes a new widget, repository, mapper, or test setup**. A strong match gives the agent a chance to reuse or extend the existing implementation. See [AI Integration](/docs/ai-integration/) for setup.

## Ranking: the worst offender is line one

Deslop ranks clusters by measured impact using AST node count, additional copies, and logarithmically dampened byte span. The exact formula is documented in [How It Works](/docs/how-it-works/#rank).

The JSON report gives agents the cluster order, byte ranges, bucket, and signals without a separate representation.

## What to do when you find duplicate Dart

Do not treat every clone as a bug. Treat it as a decision.

- **Extract** when the copies are clearly the same abstraction and will change together — pull the repeated layout into a custom widget, lift the shared styling into `ThemeData`.
- **Reuse** when one implementation is already the better source of truth and the others should call it.
- **Accept** when the duplication is deliberate: generated code, fixtures, platform shims, or two paths that look alike today but are expected to diverge.
