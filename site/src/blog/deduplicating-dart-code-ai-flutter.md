---
layout: layouts/blog.njk
title: "Deduplicating Dart Code When AI Writes Your Flutter App"
date: 2026-06-05
author: Christian Findlay
tags:
  - posts
  - dart
  - flutter
  - ai-generated-code
  - duplicate-code
category: engineering
description: "AI writes Dart faster than you can review it, and Flutter's verbose widget trees hide the duplication. How to detect and deduplicate cloned Dart code."
excerpt: "Coding agents generate Flutter widgets, repositories, and state-management classes at a pace no reviewer can keep up with. Flutter's verbosity hides the copies. Here is what to check, why line-based tools miss it, and how to deduplicate Dart structurally."
ogImage: "/assets/img/blog/deduplicating-dart-code-ai-flutter-og.png"
ogImageWidth: "1200"
ogImageHeight: "630"
ogImageAlt: "Deslop — deduplicating Dart code when AI writes your Flutter app. Live LSP and MCP duplicate-code server, worst-offenders ranked."
---

If you searched for "Flutter code duplication", "deduplicate Dart code", or "AI-generated Flutter technical debt", the short answer is this: AI does not need to write *wrong* Dart to hurt a Flutter codebase. It only needs to write the same widget, the same repository, or the same validation rule twice — in two slightly different shapes — faster than anyone can notice.

Flutter is unusually good at hiding that. A widget tree is verbose by design, so a forty-line `build` method that is 80% identical to one three files away does not stand out in review. The diff looks like normal Flutter. The app runs. And the codebase now has two implementations that need the same future fix.

For the full algorithm write-up, see [Research Background](/docs/research-background/). This post is the Dart-and-Flutter-specific version.

## AI increased how fast code gets copied, not how carefully

The throughput change is now measured, not anecdotal. GitClear's [2025 AI Copilot Code Quality study](https://www.gitclear.com/ai_assistant_code_quality_2025_research) analysed 211 million changed lines and found that copy/pasted lines rose from 8.3% in 2021 to 12.3% in 2024, with an eight-fold increase in duplicated blocks of five or more lines. In 2024, copy/pasted code exceeded *moved* code for the first time on record — and "moved" code, the signal that someone consolidated logic into something reusable, fell below 10%, a 44% year-on-year drop.

The framing is consistent across the industry. LeadDev's coverage of [how AI-generated code accelerates technical debt](https://leaddev.com/technical-direction/how-ai-generated-code-accelerates-technical-debt) quotes API evangelist Kin Lane: "I don't think I have ever seen so much technical debt being created in such a short period of time during my 35-year career in technology." The research direction agrees too — the [Code Copycat Conundrum](https://arxiv.org/abs/2504.12608) measures repetition in LLM-generated code at the character, statement, and block level.

None of this means AI-written Dart is bad. It means AI-written Dart deserves the same repository-level appraisal as hand-written Dart — only faster, because it arrives faster.

## Why Flutter codebases duplicate more than most

Flutter has a few structural traits that make AI-generated duplication easier to create and harder to spot:

- **Widget trees are verbose.** A coding agent will happily inline a card, a list tile, or a form field directly into a `build` method instead of extracting a reusable widget. Ten screens later you have ten near-identical card layouts, each lightly tweaked.
- **`build` methods are long by default.** Length is normal in Flutter, so a long, partly-duplicated method does not trip the usual "this file is too big" instinct.
- **State management gets mixed.** AI tools frequently blend patterns from `Provider`, `Riverpod`, and `Bloc` within one project, producing several differently-shaped solutions to the same state problem.
- **The repeating layers are repetitive on purpose.** Repositories, data mappers, `copyWith` methods, retry wrappers, and `*_test.dart` setup blocks all follow predictable shapes — exactly the shapes an LLM reproduces from one task to the next.

Flutter has good answers for all of this. You can extract a custom widget, lift shared styling into [`ThemeData`](https://docs.flutter.dev/cookbook/design/themes), [reduce widget boilerplate with hooks](https://medium.com/@remirousselet/flutter-reducing-widgets-boilerplate-3e635f10685e), or lean on [code generation](https://codewithandrea.com/articles/dart-flutter-code-generation/) for the boilerplate layers. The problem is not a lack of tools. The problem is *noticing* that two pieces of Dart are the same abstraction before the second copy lands.

## What a Dart duplicate-code check should look for

A useful check does not stop at exact line matches. It should find four levels of similarity:

1. **Exact duplicate code** — the same Dart copied with only formatting or comment changes.
2. **Renamed duplicate code** — the same structure with different identifiers: a `CustomerCard` widget cloned into `AccountCard`, `customerId` swapped for `accountId`.
3. **Near-duplicate code** — mostly the same logic with statements inserted, deleted, or reordered: the same form validation with one extra branch.
4. **Same behaviour, different code** — two widgets or functions that solve the same problem with different syntax (a `for` loop versus a `map().toList()`).

Classic clone-detection research calls these Type-1 through Type-4. The same taxonomy applies to Dart — token-based and structural clone detectors for Dart exist in the literature, including a [token-based clone detector built specifically for the Dart language](https://techniumscience.com/index.php/technium/article/view/6732) and the cross-language structural work in [Out of Step](https://www.sciencedirect.com/science/article/pii/S0167642324000352), which reports over 80% similarity detection on Dart and Kotlin feature sets.

## Why line matching is not enough for Dart

Line-based tools catch literal copy-paste. They go blind the moment an agent changes the surface shape — which it does constantly:

- `CustomerCard` becomes `AccountCard`, every identifier renamed.
- a helper is copied into a different class and re-indented.
- `setState` logic is rewritten as a `Riverpod` notifier that does the same thing.
- the same validation rule is rebuilt with the branches in a different order.

That is why Deslop starts from the parsed syntax tree, not the text. It parses each `.dart` file with **tree-sitter**, strips out identifier and literal names so renamed copies still match, fingerprints the tree *structure*, widens the net to near-duplicates with sibling windows and MinHash, and can optionally add embeddings for same-behaviour matches. The short version: it compares structure first, lines never. The full audit trail is in [How It Works](/docs/how-it-works/).

## Prevention beats cleanup: ask before you write

Deduplicating after the fact is what every static analyzer already does. Deslop's edge is being **live in the agent's inner loop**, so the second copy never lands.

If you drive Flutter work through a coding agent (Claude Code, Cursor, Copilot, Codex, Continue), wire it to call Deslop's `find-similar` tool **before it writes a new widget, repository, mapper, or test setup**. If a high-similarity match already exists, the agent reuses the canonical implementation instead of authoring copy number two. Quality stops being a periodic audit and becomes a continuous loop — the same direction [the Dart/Flutter tooling community is moving with MCP-driven code quality](https://dcm.dev/blog/2025/08/25/agentic-code-quality-dcm-mcp/). See [AI Integration](/docs/ai-integration/) for the setup.

## Ranking: the worst offender is line one

A duplicate-code report with 200 unordered findings is just another backlog. Deslop ranks clusters by impact — clone size × clone count × spanned lines — so the first item is the highest-payoff target, not the alphabetically-first one.

That matters more for an agent than a human. An agent does not need a wall of clone data; it needs a small, structured answer: which cluster matters most, where the byte ranges are, why it was flagged, and whether the signal came from structure, tokens, or embeddings. That is why Deslop is JSON-first and why the LSP and MCP surfaces exist — AI creates duplicate Dart quickly, so the feedback has to sit right next to the edit.

## What to do when you find duplicate Dart

Do not treat every clone as a bug. Treat it as a decision.

- **Extract** when the copies are clearly the same abstraction and will change together — pull the repeated layout into a custom widget, lift the shared styling into `ThemeData`.
- **Reuse** when one implementation is already the better source of truth and the others should call it.
- **Accept** when the duplication is deliberate: generated code, fixtures, platform shims, or two paths that look alike today but are expected to diverge.

The mistake is not accepting duplication. The mistake is accepting it *accidentally*, because no one measured it.

## FAQ

### Does Deslop support Dart?

Yes. Dart is a first-class language today, parsed with tree-sitter alongside C#, Rust, and Python. TypeScript and Go are on the roadmap.

### Is duplicate code always technical debt?

No. Some duplication is intentional and cheaper than the abstraction that would replace it. Deslop surfaces the evidence; it does not force a refactor.

### Can't I just ask the LLM to review its own Flutter code for duplication?

Sometimes. But a deterministic report is far easier to audit — it points to files, byte ranges, signal scores, and schema fields. The agent reads that and *then* makes a refactor plan.

### Is this only an AI problem?

No. The clone-detection literature predates modern LLMs by decades, and Flutter has been verbose since 1.0. AI matters because it raises how quickly duplicate Dart appears — and, per the GitClear data, how rarely it gets consolidated afterwards.

AI-generated Dart is not automatically bad Dart. But if it produces the same widget faster than your team reviews it, the Flutter maintenance bill is real. Measure it while the code is still fresh.
