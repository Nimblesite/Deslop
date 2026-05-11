---
layout: layouts/docs.njk
title: How It Works — Tree-sitter ASTs, MinHash LSH, HNSW embeddings
description: Deslop's pipeline — tree-sitter parse, AST normalization, Merkle fingerprints, MinHash + LSH, HNSW embeddings, fused 0.85 threshold, worst-offenders ranking. 250 ms reactive loop.
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

Every stage maps to a research line; the file pointers are in [Research Background](/docs/research-background/) and the spec's [implementation-status table](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/SPEC.md#algorithm-implementation-status).

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

Every subtree with ≥ `--min-nodes` nodes gets a **bottom-up BLAKE3 Merkle hash** combining its node kind with the ordered hashes of its children. This is Chilowicz 2009's syntax-tree fingerprinting, applied to tree-sitter ASTs. A second pass — sibling-window fingerprints of width 2 to 8 — extends **nearly identical code** [Type-3] recall by hashing contiguous statement runs whose parent doesn't share structure (`crates/deslop-core/src/sibling.rs`). Subtrees are emitted with byte ranges — line numbers are a render-time concern. The on-disk cache is keyed by `(content_hash, language, tool_version, min_nodes)`.

## Cluster

Identical Merkle hashes across files or within the same file form an **identical code** cluster (Type-1 / Type-2) immediately. This pass is O(n) and finds the most expensive duplication without any approximate matching. Surviving pairs from the LSH and embedding passes are unioned in and clustered by **transitive closure** ([`crates/deslop-core/src/cluster.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/cluster.rs)) — A↔B and B↔C produce one cluster even when A and C never paired directly.

## LSH (near-miss)

For **nearly identical code** (Type-3, structurally similar but not identical), Deslop builds a 5-wide k-gram stream of normalized AST kinds per subtree, computes a **128-value MinHash signature** (Broder 1997), and groups them into **32 bands of 4 rows** for [Indyk-Motwani locality-sensitive hashing](https://en.wikipedia.org/wiki/Locality-sensitive_hashing). Candidate pairs are bands that collide; Jaccard is then estimated from full-signature agreement. SourcererCC's bag-of-tokens design is the inspiration, but Deslop runs its k-grams over normalized AST kinds rather than raw source tokens. Implementation lives in [`crates/deslop-core/src/lsh.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/lsh.rs) and [`crates/deslop-core/src/tokens.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/tokens.rs).

## Embed (semantic)

Optional, **off by default** — opt in with `--embeddings auto` (probe and fall back with a warning) or `--embeddings required` (hard-fail if the provider is unreachable). When enabled, each subtree is run through a code-embedding model (local Ollama by default — `nomic-embed-text` out of the box, any Ollama embedding model selectable via `--embedding-model`). Nearest-neighbour search runs over an **HNSW** index (`instant-distance`, pure Rust, deterministic seed) at the cosine threshold defined in [`crates/deslop-core/src/embedding/pairs.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/embedding/pairs.rs). This produces **same behavior, different code** candidates (Type-4) — semantically equivalent but syntactically different code, such as an imperative loop versus a LINQ expression. SSCD (Wiley 2024) validated HNSW + ANN as the right recall layer at scale; Deslop adopts the same shape and pairs it with the structural and LSH passes per [`fusion.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/fusion.md).

The embedding cache is keyed by `(content_hash, provider_id, model_id, model_version)` so switching models invalidates only the embedding layer — structural and LSH caches survive.

## Fuse

Each candidate pair gets three independent scores:

| Signal | Range | Detects | Source |
| --- | --- | --- | --- |
| `structural` | 0 / 1 | Identical code [Type-1/2] — exact Merkle bucket | `pair.rs::collect_structural_pairs` |
| `token_jaccard` | 0..1 | Nearly identical code [Type-3] — MinHash band collisions | `lsh.rs::band_collisions` + `tokens.rs` |
| `embedding_cos` | 0..1 | Same behavior, different code [Type-3/4] — HNSW top-k | `embedding/pairs.rs` |

Per the ensemble-LLM 2025 finding (averaging hurts; sum/max help), the fused score is `clamp(structural + token_jaccard + embedding_cos, 0, 1)` ([`pair.rs::PairScore::fused`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/pair.rs)). Pairs survive when the fused score crosses `FUSED_THRESHOLD = 0.85`. LSH-only pairs carry a stricter information-content floor (`token_jaccard ≥ 0.90` and both endpoints ≥ 40 AST nodes) so noisy near-misses can't ride the LSH bus into a cluster. Cross-language pairs are dropped unless `.deslop.toml` opts in.

## Rank

The ranking score is the entire user-visible product. The implementation in [`crates/deslop-core/src/cluster.rs::rank_weight`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/cluster.rs) is:

```
weight = clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes)
```

Bigger fragments count more (`clone_node_count`). More copies count more (`cluster_size − 1`, so a single-member cluster scores zero). The `log2(1 + spanned_bytes)` term grows with payoff but flattens for very large spans, so a 50-line method copied four times outranks a 5000-line file copied once. The top of the report is always the largest payoff — not the first cluster found.

## Render

Three renderers read the same materialized view:

- **JSON** — canonical and strictly typed. Carries the embedded `schema_doc`, `action_hints`, repo-wide `metrics`, and `embedding_provenance`.
- **TXT** — ASCII, line-oriented, no ANSI. Pipeable into `head`, `grep`, `awk`.
- **HTML** — standalone, inlined CSS, zero network dependencies. Default mode embeds source snippets with tree-sitter-driven syntax highlighting; `--human=off` falls back to terse byte-offset HTML.

Agents consume JSON. Humans read TXT in the terminal or open the HTML in a browser. Every claim the TXT or HTML makes is also present in the JSON.

## Live = reactive

Everything above also runs incrementally inside the LSP server (`crates/deslop-core/src/live/`). A file watcher with a 250 ms debounce and a 2 s cap drives `PipelineSession::update_files`; the LSP atomically writes `.deslop-cache/live-report.json`, broadcasts `deslop/reportChanged` over the LSP wire, and exposes a Unix-domain IPC socket so the bundled MCP server can `find-similar` against the running corpus without re-parsing. The CLI is the cold-cache fallback for CI gates; every other surface — VS Code bubble, Top Offenders tree, status bar, hover, code lens, agent MCP queries — reflects the new state in the same microtask the watcher fires.
