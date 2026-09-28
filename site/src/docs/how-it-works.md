---
layout: layouts/docs.njk
title: How It Works — Finding and Comparing Duplicate Code
description: How Deslop finds exact, renamed and edited copies, separates shape-only matches, and ranks duplicate code by mass.
eleventyNavigation:
  key: How It Works
  order: 2
icon: account_tree
docsGroup: trust
---

# How It Works

Deslop parses source code with tree-sitter, finds candidate copies, checks their content, and groups the surviving findings. The CLI, editor and MCP tools share the same Rust engine.

This guide covers [0.35.0](/releases/).

## Find candidate copies

Deslop supports C#, Rust, Python, Dart, JavaScript, TypeScript/TSX, PHP, F# and Go. It respects `.gitignore` and excludes dependency trees and build artifacts by default. See [Configuration](/docs/configuration/#built-in-rules-always-on) for exclusions.

The engine normalizes syntax trees so changes to names, literals and formatting do not hide candidates. It fingerprints both subtrees and consecutive statement runs, including copies inside a single file. Structural overlap and MinHash similarity help recover edited copies whose enclosing methods differ.

Normalization alone cannot establish a clone. The engine also checks source content, consistent renaming and the operations being performed. Changing a collaborator's name differs from changing which operation it calls.

## Read the categories

| Report label | Meaning |
| --- | --- |
| **Identical code** | Source text matches apart from permitted whitespace differences. Corresponds to Type I; Deslop's identity rule is stricter than the research definition, which also permits comment changes. |
| **Nearly identical code** | Renamed or parameterized copies, or copies with small edits: Type II and close Type III. |
| **Similar code** | Substantial copied work with larger statement or control-flow edits: Type III. |
| **Same behavior, different code** | Optional embedding-based candidates for Type IV. Review both implementations; this is not proof that they are interchangeable. |
| **Same shape, different content** | Matching layout with negligible shared content. Informational, not a clone. |

Shape-only findings contribute nothing to clone counts, duplicated mass or duplication percentages. They appear after clones and have no diagnostic by default. A group label describes its established member relations; a chain of matches does not prove every possible pair matches.

## Compare two occurrences

Similarity evidence belongs to two explicit source ranges. A cluster carries its kind, occurrences, mass and rank; it does not carry one pair's similarity score as a group-wide confidence value.

In VS Code, use **Compare To Canonical**, or **Select for Compare** followed by **Compare with Selected**. The CLI also supports [`--compare`](/docs/configuration/#compare-two-occurrences). The engine recomputes the verdict for the chosen endpoints.

## Rank by duplicated mass

```text
mass = canonical_node_count × max(visible_members − 1, 0)
```

Mass measures the size of a copied syntax tree multiplied by its additional visible copies. Clones are sorted by mass descending, with stable IDs breaking ties. Shape-only findings have zero duplicated mass and no clone rank.

Mass is neither confidence nor a percentage. The [duplication percentage](/docs/accuracy-transparency/) counts covered source lines, with overlapping ranges counted once.

## Optional embeddings

Embeddings are off by default. `--embeddings auto` uses the configured provider if available; `--embeddings required` fails if it cannot be reached. The Ollama provider defaults to `nomic-embed-text`, with a configurable model and endpoint. Embedding similarity adds candidates; it does not prove equivalent behaviour or a safe extraction.

## Keep the report current

Cached parse and signature work can be reused for unchanged files. A live session watches edits, updates the analysis and broadcasts the fresh report to the editor. MCP queries consult that session; the CLI performs a separate scan for CI or a one-off review.

JSON, text and HTML expose the same report. See [Report output](/docs/configuration/#report-output), [Accuracy Transparency](/docs/accuracy-transparency/) and [Research Background](/docs/research-background/) for the contracts and evidence.
