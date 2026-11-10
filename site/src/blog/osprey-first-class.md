---
layout: layouts/blog.njk
docFilm: osprey
title: "Osprey in Deslop: Effects, Handlers and Fewer Copies"
date: 2026-10-25
author: Christian Findlay
tags:
  - posts
  - osprey
  - algebraic-effects
  - duplicate-code
category: engineering
description: "Osprey gets native tree-sitter analysis in Deslop: live duplicate code findings in VS Code, MCP checks for coding agents, and the official Osprey icon."
excerpt: "Effects. Reusable handlers. Inferred types. Native compilation. Osprey has our attention—and now it has native duplicate detection with its official visual identity."
heroImage: "/assets/img/blog/osprey-first-class.webp"
heroImageWidth: "1600"
heroImageHeight: "840"
heroImageAlt: "A luminous cyan wing-shaped branching structure suspended in midnight-blue space."
ogImage: "/assets/img/blog/osprey-first-class-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
---

A functional language built around algebraic effects, inferred types and native compilation deserves tools that understand its code. Deslop now reads all three Osprey flavors: default `.osp`, native ML `.ospml`, and explicitly selected Python syntax. It finds copied structure and brings the findings into the editor and the agent workflow.

The language groups and report views use Osprey’s official cyan bird.

{% include "partials/doc-film.njk" %}

## The language worth making a fuss about

Osprey lets application logic say what it needs through an **effect**, while a **handler** supplies the implementation. That makes the boundary between a capability and its provider explicit. Production code and deterministic tests can share the same application logic.

Here is a small example. The function asks for a clock; the handler chooses the answer.

```osprey
effect Clock { now: fn() -> int }

fn announce() = {
    let timestamp = perform Clock.now()
    "Osprey launch at ${timestamp}"
}

let preview = handler Clock {
    now => 42
}

print(preview(announce))
```

`perform` requests the capability. `preview` is a reusable handler value. The function stays focused on the announcement, and the checker infers its result type. This is an exceptionally appealing way to write code whose dependencies remain visible and whose tests stay deterministic.

Osprey compiles through LLVM to native binaries, with WebAssembly and C ABI targets for supported workloads. The language is actively evolving; its [documentation](https://www.ospreylang.dev/docs/) describes the current boundaries.

## Real syntax deserves real analysis

Deslop keeps Osprey's effects, handler arms, patterns, types and expressions as syntax-tree structure. Renaming a function or a binding does not make copied code disappear: identifier spelling normalizes before the structural comparison. Literal spellings normalize too, while their source content remains available to the shared evidence checks.

String interpolation gets proper attention. The code inside `${...}` remains code: a conditional stays a conditional, and addition stays distinct from subtraction. This matters in a language where a concise expression can contain substantial application logic.

Comments and imports do not count as copied implementation. Findings use the same measured ranking as other languages, so the largest duplication problems remain at the top. The evidence determines the score.

## When a useful handler becomes six copies

Imagine a project with a deterministic clock handler for invoice tests. The next feature needs the same setup for receipts, so the handler gets copied. Then reminders need it too. Each copy grows a slightly different wrapper; names change, a literal changes, and finding the shared implementation becomes harder than searching for the original function name.

That is the moment structural analysis earns its place. Deslop compares the parsed implementation, gathers the surviving copies into a cluster and ranks the duplicated work. You can inspect the actual occurrences, decide which differences are meaningful, and extract a shared helper when the tests support it. Similar scaffolding alone is insufficient evidence; the copied work matters.

Effects make a capability explicit. A duplicate report makes repeated implementation explicit. Together they give you a useful question: should these tests share a handler, or do they express different policies that deserve separate code? The comparison helps you decide; a score cannot make that design decision for you.

## A front-row seat in the editor

Open an `.osp` file with Deslop installed. Osprey participates in the live analysis loop, with findings available through the editor, cluster comparisons and the MCP tools used by coding agents. Its language identity appears in grouping, file rows and the report webviews, alongside icons for every other supported language.

For a CLI scan of an Osprey project:

```bash
deslop .
```

Start with the highest-ranked genuine duplicate, compare its occurrences, and decide whether one tested implementation can replace the copies. Osprey gives you reusable capabilities; Deslop helps you spot application logic that deserves the same treatment.

For an agent, the useful moment comes even earlier. Before generating another test helper or service wrapper, ask `find-similar` about the intended implementation. If a suitable helper already exists, reuse it. After a change, inspect `duplicates`, then open the relevant cluster to compare the source ranges. The MCP tools consult the live analysis session, so this fits inside the editing loop.

## Three flavors, clear boundaries

Osprey has default braces syntax, ML layout syntax and a Python-looking surface. Understanding one does not automatically establish correct analysis of the others.

All three flavors have verified CLI fixtures and real LSP/MCP language-label checks. A leading `# osprey: flavor=python` comment, an explicit `python_sources` entry or a Python project entry opts a `.py` file into Osprey. Ordinary Python files keep their Python identity, and Osprey's Python surface does not inherit ordinary Python's automatic extraction actions.

Native ML `.ospml` uses a grammar verified against Osprey's real source corpus and independent structural checks. Layout, interpolation expressions, and flat-versus-curried calls remain visible syntax. The three parser identities stay distinct in caches and reports, even though the interface gives them the same official Osprey icon. Cross-flavor clone equivalence is not claimed.

## Configure the Python flavor

Ordinary `.py` files retain Python identity. The development integration recognises an Osprey flavor marker only in a leading source comment:

```python
# osprey: flavor=python
```

A string containing that text or a comment after code cannot select the flavor. Alternatively, list individual source files in the existing `osprey.toml`:

```toml
[project]
entry = "src/main.py"
python_sources = ["src/domain.py", "src/storage.py"]
```

The Python entry file and the listed files opt into Osprey. Paths are literal files relative to the manifest, not directory-wide or glob selections. Other `.py` files retain ordinary Python identity.

The project `flavor` setting accepts `default`, `ml` or `python` and overrides the syntax for sources claimed by that project. Without an override, a marker that contradicts the extension produces an error. Malformed manifests and unknown flavor values also produce errors. Source roots default to `src` when present, otherwise the project directory; `source_roots` overrides this default. Nested projects take precedence.

## Long live the bird

Effects that make dependencies explicit. Handlers you can keep and reuse. Types the checker works out. Native compilation. Osprey brings a compelling combination, and we want its tooling to match that ambition.

Explore [Osprey](https://www.ospreylang.dev/), then [install Deslop](/docs/) and put it to work.
