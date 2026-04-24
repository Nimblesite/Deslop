---
layout: layouts/docs.njk
title: How It Works
eleventyNavigation:
  key: How It Works
  order: 2
icon: account_tree
---

# How It Works

Deslop is a fixed, deterministic pipeline. No step uses regex on source code. Every step is cache-keyed so an unchanged file is skipped. The output of each stage is small, structured, and auditable.

```
discover → parse → normalize → fingerprint → cluster
           → LSH → embed → fuse → rank → render
```

## Discover

`.gitignore` is honoured. Binary files are skipped by extension and by magic bytes. Symlinks are not followed. Each candidate file is hashed (`blake3(content)`) and that hash becomes the cache key for every downstream stage.

## Parse

Each language ships a grammar via tree-sitter:

| Language | Status |
| --- | --- |
| C# | v1 |
| Rust | v1 |
| Python | v1 |
| TypeScript / JavaScript | roadmap |
| Go | roadmap |

A parser produces an AST. No source-level regex touches this pipeline — ever.

## Normalize

Identical code can differ only in identifiers and literals (Type-2 renaming). Deslop strips:

- identifier names (rewritten to `__id__`)
- string / number / char literals (rewritten to `__lit__`)
- comments, whitespace, trivia

Per-language normalization rules, identical output format across languages. A renamed copy of a method hashes to the same fingerprint as the original.

## Fingerprint

Every subtree with ≥ `--min-nodes` nodes gets a Merkle hash. Subtrees are emitted with byte ranges — line numbers are a render-time concern. Fingerprints are stable across runs; the on-disk cache is keyed by `(content_hash, language, min_nodes)`.

## Cluster

Identical Merkle hashes across files or within the same file form an **identical code** cluster (Type-1 / Type-2) immediately. This pass is O(n) and finds the most expensive duplication without any approximate matching.

## LSH (near-miss)

For **nearly identical code** (Type-3, structurally similar but not identical), Deslop builds a token bag per subtree and applies locality-sensitive hashing (MinHash + banding). Candidate pairs with Jaccard similarity above a floor feed the fusion step. Sub-threshold overlaps survive as **loosely similar code** hints (weak LSH-only).

## Embed (semantic)

Optional. When enabled, each subtree is run through a code-embedding model (local Ollama by default, configurable). Nearest-neighbour search via ANN produces **same behavior, different code** candidates (Type-4) — semantically equivalent but syntactically different code, such as an imperative loop versus a LINQ expression.

## Fuse

Each candidate pair gets three independent scores:

| Signal | Range | Detects |
| --- | --- | --- |
| `structural` | 0..1 | Identical code [Type-1/2] — exact fingerprint match |
| `token_jaccard` | 0..1 | Identical + nearly identical code [Type-2/3] — renamed + near-miss |
| `embedding_cos` | 0..1 | Same behavior, different code [Type-3/4] — semantic |

Pairs are accepted when at least one signal crosses the acceptance floor and the weighted sum exceeds the decision threshold. Defaults are tuned per spec at [`docs/specs/decisions.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/decisions.md).

## Rank

The ranking score is the entire user-visible product:

```
score = clone_size_nodes × clone_count × spanned_LOC
```

Bigger fragments count more. More copies count more. More lines on screen count more. The top of the report is always the largest payoff — not the first cluster found.

## Render

Three renderers read the same materialized view:

- **JSON** — canonical, versioned (`report_schema_version`), strictly-typed.
- **TXT** — ASCII, line-oriented, no ANSI.
- **HTML** — standalone, inlined CSS, zero network dependencies.

Agents consume JSON. Humans read TXT in the terminal or open the HTML in a browser. Every claim the TXT or HTML makes is also present in the JSON.
