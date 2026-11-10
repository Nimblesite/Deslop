---
layout: layouts/blog.njk
title: "Finding Duplicate Dart Code in Flutter Projects"
date: 2026-06-05
updated: 2026-11-11
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
heroImage: "/assets/img/blog/deduplicating-dart-code-ai-flutter-header.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "Header image showing Flutter widget trees, cloned Dart cards, and a find-similar gate."
ogImage: "/assets/img/blog/deduplicating-dart-code-ai-flutter-og.jpg"
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

Here is a typical pair. A `CustomerCard` widget was copied to build an `AccountCard`:

```dart
class CustomerCard extends StatelessWidget {
  const CustomerCard({super.key, required this.customer});
  final Customer customer;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: ListTile(
        leading: CircleAvatar(child: Text(customer.initials)),
        title: Text(customer.name),
        subtitle: Text(customer.email),
        trailing: const Icon(Icons.chevron_right),
      ),
    );
  }
}
```

`AccountCard` has the same `build` body with `account` in place of `customer`. When a design change adds a badge to one card, the other falls out of step. Copied `build` bodies like this stay in the report even when the widget names change.

## What Deslop does not report in Flutter code

Flutter has a lot of necessary repetition, and reporting it would bury the real copies. Deslop leaves out:

- **Widget shells.** A `StatelessWidget`, `StatefulWidget` or `State<T>` declaration whose body only constructs widgets and returns them is framework scaffolding. Any local variables, control flow, arithmetic, mutation or closures make it ordinary code again, and copied `build` bodies are still reported.
- **Renamed signatures alone.** Two declarations that share only a signature shape are not evidence of a copied body.
- **Generated Dart.** Files ending in `.g.dart`, `.freezed.dart`, `.gr.dart`, `.config.dart`, `.gen.dart`, `.mocks.dart` and the protobuf `.pb*.dart` family are analysed but hidden from the report, along with files that open with a generator banner such as `// GENERATED CODE - DO NOT MODIFY BY HAND`.
- **Tooling and dependencies.** `.dart_tool` is always excluded, and `.pub-cache` is excluded unless you opt in.

Hidden generated files still count when *your* code duplicates them. If a hand-written mapper copies what `json_serializable` already generates, that cluster survives. See [Configuration](/docs/configuration/#built-in-rules-always-on) for the full list.

## What a Dart duplicate-code check should look for

A useful check does not stop at exact line matches. It should find four levels of similarity:

1. **Exact duplicate code** — the same Dart copied with only formatting or comment changes.
2. **Renamed duplicate code** — the same structure with different identifiers: a `CustomerCard` widget cloned into `AccountCard`, `customerId` swapped for `accountId`.
3. **Near-duplicate code** — mostly the same logic with statements inserted, deleted, or reordered: the same form validation with one extra branch.
4. **Same behavior, different code** — two widgets or functions that solve the same problem with different syntax (a `for` loop versus a `map().toList()`).

Clone-detection research calls these Type I through Type IV. Deslop's current labels and the distinction between clones and shape-only information are explained in [How It Works](/docs/how-it-works/#read-the-categories).

## Why line matching is not enough for Dart

Line-based tools catch literal copy-paste but are sensitive to surface changes:

- `CustomerCard` becomes `AccountCard`, every identifier renamed.
- a helper is copied into a different class and re-indented.
- `setState` logic is rewritten as a `Riverpod` notifier that does the same thing.
- the same validation rule is rebuilt with the branches in a different order.

That is why Deslop starts from the parsed syntax tree, not the text. It parses each `.dart` file with **tree-sitter**, strips out identifier and literal names so renamed copies still match, fingerprints the tree *structure*, widens the net to near-duplicates with sibling windows and MinHash, and can optionally add embeddings for same-behaviour matches. Structure finds candidates; a content check then confirms that the copies perform the same operations, so two widgets that merely share a layout are not reported as clones. The full audit trail is in [How It Works](/docs/how-it-works/).

## Check before writing

Deslop's MCP server can move the check into the coding agent's editing loop.

If you use a coding agent for Flutter work, configure it to call Deslop's `find-similar` tool **before it writes a new widget, repository, mapper, or test setup**. A strong match gives the agent a chance to reuse or extend the existing implementation. See [AI Integration](/docs/ai-integration/) for setup.

## Ranking: the worst offender is line one

Deslop ranks clusters by duplicated mass: the syntax-tree size of the reference copy multiplied by its additional visible copies. A 200-node `build` method copied into three screens outranks a short helper copied once. The formula and its reasoning are in [How Deslop ranks duplicate-code clusters](/blog/ranking-formula/) and [How It Works](/docs/how-it-works/#rank-by-duplicated-mass).

The JSON report gives agents the cluster order, byte ranges and category without a separate representation, and the VS Code cluster panel shows the same order.

## What to do when you find duplicate Dart

Do not treat every clone as a bug. Treat it as a decision.

- **Extract** when the copies are clearly the same abstraction and will change together — pull the repeated layout into a custom widget, lift the shared styling into `ThemeData`.
- **Reuse** when one implementation is already the better source of truth and the others should call it.
- **Accept** when the duplication is deliberate: generated code, fixtures, platform shims, or two paths that look alike today but are expected to diverge.

## Keep new copies out of pull requests

Cleaning up existing duplicates is a one-off job. Keeping new ones out takes a check in the workflow:

- **In the editor.** The VS Code extension updates its findings as you edit, so a copied widget appears in the cluster panel before you commit.
- **In the agent.** Claude Code, Cursor, Codex and other MCP clients can call `find-similar` before generating a widget or repository class.
- **In CI.** The [GitHub Action](/docs/github-action/) runs the same CLI on every pull request. Use `--diff` with `--only-changed` to judge only the lines a pull request adds, so existing debt cannot fail a new change.

Together, these checks keep a Flutter codebase consistent as AI-assisted work speeds up: one card widget, one repository per resource, one validation rule per field.
