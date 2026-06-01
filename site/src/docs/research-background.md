---
layout: layouts/docs.njk
title: Research Background — Code-clone detection algorithms
description: Deslop's research lineage — Baxter 1998, Chilowicz 2009 Merkle fingerprints, Broder 1997 MinHash, Indyk-Motwani 1998 LSH, SSCD 2024 HNSW. Each pinned to a real file.
eleventyNavigation:
  key: Research Background
  order: 5
icon: science
---

# Research Background

Deslop is a clone-analysis system for codebases where duplication can grow faster than humans can review it. The design draws from classic code-clone detection research, but the implementation is concrete and auditable: discovery, tree-sitter parsing, AST normalization, Merkle fingerprinting, sibling-window matching, MinHash LSH, optional embedding search, pair fusion, transitive closure, ranking, and report rendering all live in `deslop-core`.

This page distinguishes two things:

- **Research background**: the literature and algorithm families that shaped the tool.
- **Implemented behavior**: what the current code actually does, with file/function pointers that auditors can verify.

## Why AI changes the clone problem

Code clones were already a maintenance risk before AI coding assistants. The established concern is not that every duplicate is automatically wrong; it is that copied logic must be kept consistent across fixes, security patches, and feature changes. AI-assisted development changes the economics: the cost of generating another similar implementation drops, so repeated logic can enter the repository during ordinary feature work rather than only through deliberate copy-and-paste.

Recent research supports that risk model:

- **LLM code repetition**: [*Code Copycat Conundrum: Demystifying Repetition in LLM-based Code Generation*](https://arxiv.org/abs/2504.12608) studies 19 code LLMs and reports that repetition appears across character, statement, and block levels, including structurally redundant code. The paper also evaluates a repetition-mitigation technique in open-source and industrial settings.
- **LLM-generated clones**: [*Unveiling the potential of large language models in generating semantic and cross-language clones*](https://arxiv.org/abs/2309.06424) evaluates GPT-3's ability to generate semantic and cross-language clone variants, which is directly relevant to Type-4 and cross-language duplicate detection.
- **Commercial AI code generators**: [*An Empirical Study of Code Clones from Commercial AI Code Generators*](https://dl.acm.org/doi/10.1145/3729397) (FSE 2025) reports Type-1 and Type-2 clone rates up to 7.50% for studied commercial generators, and discusses copyright, bug propagation, and vulnerability propagation risks.
- **AI-era clone detection**: [*Are Classical Clone Detectors Good Enough For the AI Era?*](https://arxiv.org/abs/2509.25754) evaluates nine clone detectors on GPTCloneBench and traditional clone benchmarks, highlighting why normalization and semantic variation matter when clones are AI-generated.
- **Technical debt in production repositories**: [*Debt Behind the AI Boom: A Large-Scale Empirical Study of AI-Generated Code in the Wild*](https://arxiv.org/abs/2603.28592) analyzes verified AI-authored commits and tracks static-analysis issues introduced by those commits. It is broader than duplication, but it supports treating AI-generated code as a technical-debt source that needs repository-level appraisal.

The business claim Deslop makes is therefore conservative: duplicate logic is a compounding maintenance liability, and AI can increase the production rate of that liability. Deslop measures the liability so teams and agents can decide whether to extract, reuse, hide, or explicitly accept a clone.

## Clone taxonomy

Deslop follows the standard clone taxonomy used throughout the code-clone literature:

| Clone class | Meaning | Deslop signal |
| --- | --- | --- |
| Type-1 | Exact copied text except layout or comments | Structural hash after parsing and normalization |
| Type-2 | Same structure with renamed identifiers or changed literals | Structural hash after identifier/literal collapse |
| Type-3 | Near-miss clone with inserted, deleted, or changed statements | Sibling-window fingerprints and token MinHash LSH |
| Type-4 | Similar behavior with different syntax or structure | Optional embedding cosine similarity |

The public report buckets are implemented in `crates/deslop-core/src/buckets.rs`. The code maps signal triples to four wire labels: `identical`, `nearly_identical`, `loosely_similar`, and `same_behavior`. The `same_behavior` bucket is only reachable when the embedding signal is strong enough.

## Algorithm foundations

Each row pairs a research line with the file that implements it (✅), the plan that tracks it (⏳), or notes that we surveyed but did not adopt it (🚫).

| Research line | What Deslop takes from it | Status & implementation pointer |
| --- | --- | --- |
| [Baxter et al. 1998 — AST clone detection](https://ieeexplore.ieee.org/document/738528) | Parse code into syntax trees, normalize irrelevant spelling, and compare tree structure rather than raw text. | ✅ `crates/deslop-core/src/lang/shared.rs`, `lang/csharp.rs`, `lang/rust_lang.rs`, `lang/python.rs`, `lang/dart.rs` (registered through `pipeline/corpus.rs::default_parsers`) |
| Chilowicz et al. 2009 — syntax-tree fingerprinting | Hash subtrees so exact structural clones become equal fingerprints; extend coverage with sibling sequences for near-miss clones. | ✅ Bottom-up BLAKE3 Merkle in `crates/deslop-core/src/fingerprint.rs::collect_non_boilerplate_fingerprints` and width-2..8 sibling windows in `crates/deslop-core/src/sibling.rs::collect_non_boilerplate_sibling_fingerprints` |
| [SourcererCC (Sajnani et al. 2016)](https://arxiv.org/abs/1512.06448) | Token k-grams and Jaccard similarity for scalable near-miss detection. | ✅ Adapted to **normalized AST-kind k-grams** rather than raw source tokens: `crates/deslop-core/src/tokens.rs`, `crates/deslop-core/src/pipeline/signatures.rs` |
| [MinHash (Broder 1997)](https://ieeexplore.ieee.org/document/666900) | Estimates Jaccard from compact signatures. | ✅ 128-value signatures in `crates/deslop-core/src/lsh.rs::minhash_signature`; Jaccard estimated by `estimate_jaccard` |
| LSH banding (Indyk & Motwani 1998) | Bucket similar fingerprints in sub-linear time. | ✅ 32 bands × 4 rows in `crates/deslop-core/src/lsh.rs::band_collisions` |
| In Defense of MinHash Over SimHash (Shrivastava & Li 2014) | Use MinHash, not SimHash, for binarized features. | ✅ MinHash chosen; SimHash and Winnowing not used |
| Neural semantic clone detection (CodeBERT, GraphCodeBERT, UniXCoder) | Use embeddings as a recall layer for Type-4 clones. | ✅ `EmbeddingProvider` trait in `crates/deslop-core/src/embedding/provider.rs`; Ollama provider in `embedding/ollama.rs`; default model `nomic-embed-text` |
| [SSCD (Ahmed et al., Wiley 2024) — BERT + ANN at scale](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3355) | HNSW ANN over BERT-style embeddings as the Type-3/4 recall path. | ✅ `instant-distance` HNSW with deterministic seed, top-k retrieval, cosine threshold 0.80: `crates/deslop-core/src/embedding/pairs.rs` |
| [Ensemble-LLM 2025 (arXiv 2510.15480) — max/sum fusion](https://arxiv.org/abs/2510.15480) | Average hurts; max and sum help when fusing signals. | ✅ Three-signal max-normalized sum in `crates/deslop-core/src/pair.rs::PairScore::fused`, clamped to `[0,1]`. Multi-model embedding ensembling is **not** implemented (single embedding model today; provider trait keeps it open) |
| Hybrid clone detection (no pure-RAG paper recommends pure embeddings) | Union structural + token + embedding pairs, fuse, cluster. | ✅ `crates/deslop-core/src/pair.rs::candidate_pairs`, transitive closure in `crates/deslop-core/src/cluster.rs` |
| Boilerplate filtering (mature-tool convention) | Drop import / namespace / decorator clones before fingerprinting; re-surface as low-noise hints. | ✅ `crates/deslop-core/src/boilerplate.rs` and `report_boilerplate.rs` |
| HyClone 2025 (arXiv 2508.01357) — execution-validated Type-4 | Generate test inputs and validate semantically equivalent pairs. | 🚫 Not implemented; Python-specific in the original paper |
| Rator 2025 (Springer) — node degrees-of-freedom encoding | Encode subtrees by node DoF; fine-grained Top-2/Top-3 localization. | 🚫 Not implemented; LSH covers our recall budget today |
| Autofix `refactor.extract` (LSP code action) | Rewrite Type-1 clusters into a single shared method. | ⏳ Tracked in [`docs/plans/autofix-extract-method-plan.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/plans/autofix-extract-method-plan.md); blocked on the Type-1/Type-2 bucket split (gh #42) |
| AI-assisted Extract (MCP `extract-method-plan` / `-apply`) | Type-2/3 extraction with agent assistance. | ⏳ Tracked in [`docs/plans/autofix-extract-ai-plan.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/plans/autofix-extract-ai-plan.md); blocked on the Type-1 path landing |

## Implemented pipeline

The batch CLI and live services both run through `PipelineSession`. The batch entry point in `crates/deslop-core/src/pipeline/run.rs` delegates to `PipelineSession::initialise`, and the current corpus is rendered by `PipelineSession::render` in `crates/deslop-core/src/pipeline/session.rs`.

### 1. File discovery

`crates/deslop-core/src/discover.rs::discover_files` walks the target root with the `ignore` crate, applies standard ignore filters, does not follow symlinks, filters by registered language extensions, applies `.deslop.toml` exclusion rules, and registers surviving files in a `FileRegistry`.

Supported language parsers are currently registered by `crates/deslop-core/src/pipeline/corpus.rs::default_parsers`: C#, Rust, Python, and Dart. TypeScript, JavaScript, Go, and other languages are not registered in the current core pipeline.

### 2. Parse and normalize

Each language parser uses tree-sitter through the shared machinery in `crates/deslop-core/src/lang/shared.rs`. `build_normalised_root` walks named tree-sitter nodes and produces a `NormalizedNode` tree rooted at `__file__`.

Normalization is language-specific:

- C#: `crates/deslop-core/src/lang/csharp.rs`
- Rust: `crates/deslop-core/src/lang/rust_lang.rs`
- Python: `crates/deslop-core/src/lang/python.rs`
- Dart: `crates/deslop-core/src/lang/dart.rs`

The shared constants are `__ident__` for identifier-like nodes and `__literal__` for literal-like nodes. Comments and trivia are dropped by returning `None` from the language-specific normalizer. This is the mechanism that makes renamed Type-2 clones hash together.

### 3. Boilerplate suppression

Import and prologue-like structures are suppressed before clone fingerprints are emitted. The filter lives in `crates/deslop-core/src/boilerplate.rs`, while the structural and sibling collectors call it from `crates/deslop-core/src/fingerprint.rs` and `crates/deslop-core/src/sibling.rs`.

This matters for generated and AI-assisted repositories because repeated imports, namespace wrappers, and prologue scaffolding can otherwise dominate the candidate set. The report still exposes boilerplate hints through `Report.boilerplate_hints`.

### 4. Merkle structural fingerprints

`crates/deslop-core/src/fingerprint.rs::collect_non_boilerplate_fingerprints` walks each normalized AST bottom-up. For every subtree with at least `--min-nodes` nodes, it emits:

- a BLAKE3 Merkle hash over the normalized node kind and child hashes,
- the source file id,
- the exact byte range,
- the subtree node count.

Pairs sharing a Merkle hash enter the candidate set with `structural = 1.0`. The implementation uses byte ranges internally; line and column labels are render-time projections.

### 5. Sibling-window fingerprints

`crates/deslop-core/src/sibling.rs::collect_non_boilerplate_sibling_fingerprints` emits fingerprints for contiguous sibling windows of width 2 through 8 when their combined node count meets `--min-nodes`. The synthetic hash prefix is `__sibling_window__`.

This is the first Type-3 extension: two methods can differ by surrounding structure while still sharing a repeated run of statements.

### 6. Token MinHash and LSH

For every fingerprint, `crates/deslop-core/src/tokens.rs` extracts a pre-order stream of normalized node kinds and builds 5-wide k-grams. `crates/deslop-core/src/lsh.rs` then builds 128-value MinHash signatures, split into 32 bands of 4 rows.

`band_collisions` returns candidate pairs whose signatures collide in at least one band. Jaccard similarity is estimated from full-signature agreement by `estimate_jaccard`. LSH buckets use a star topology rather than full N squared enumeration so large duplicate buckets stay tractable.

### 7. Optional embeddings

Embeddings are controlled by `EmbeddingMode` in `crates/deslop-core/src/embedding/mode.rs` and executed by `crates/deslop-core/src/pipeline/embedding_pass.rs`.

Current CLI behavior is important:

- `crates/deslop/src/main.rs` sets `--embeddings` default to `off`.
- `auto` probes the provider and continues without embeddings if the provider is unavailable.
- `required` propagates provider failures so the CLI exits non-zero.
- The default provider key is `ollama`.
- `crates/deslop-core/src/embedding/ollama.rs` sets the default endpoint to `http://127.0.0.1:11434` and the default model to `nomic-embed-text`.

When embeddings run, snippets are cached under `.deslop-cache/embeddings/...`; HNSW nearest-neighbour search is implemented in `crates/deslop-core/src/embedding/pairs.rs`. The pair generator uses top-k neighbours and a minimum cosine threshold of 0.80.

### 8. Pair fusion and clustering

`crates/deslop-core/src/pair.rs::candidate_pairs` unions three sources:

- structural hash-bucket pairs,
- LSH band-collision pairs,
- embedding nearest-neighbour pairs.

Each pair carries:

- `structural`,
- `token_jaccard`,
- `embedding_cos`,
- `fused`.

`PairScore::fused` sums the three signals and clamps the result to `[0.0, 1.0]`. `FUSED_THRESHOLD` is `0.85`. LSH-only pairs have additional guards: `token_jaccard >= 0.90` and both endpoints must have at least 40 AST nodes.

Cross-language comparison is off by default. `crates/deslop-core/src/config.rs` initializes `allow_cross_language_comparison` to `false`, and `crates/deslop-core/src/pair.rs::candidate_pairs_for_language_policy` drops cross-language pairs unless configuration enables them.

`cluster_by_transitive_closure` forms connected components from surviving candidate pairs. This means A matches B and B matches C can produce one cluster even if A and C were not directly paired.

### 9. Ranking

Clusters are materialized and sorted in `crates/deslop-core/src/cluster.rs`. Overlapping occurrences in the same file are collapsed before report output. The rank formula is:

```text
weight = clone_node_count * (cluster_size - 1) * log2(1 + spanned_bytes)
```

This is not a LOC formula. LOC is used for repository metrics, but rank weight currently uses node count, cluster size, and spanned bytes.

### 10. Report rendering

The canonical report is `Report` in `crates/deslop-core/src/report.rs`. `render_report` applies `report_hide`, computes repository metrics, includes embedded schema documentation from `docs/specs/REPORTING-CONTEXT.md`, and attaches action hints and embedding provenance.

Repository-wide metrics are computed in `crates/deslop-core/src/report_metrics.rs`. `duplicated_loc` is derived from non-hidden clone occurrence line ranges and deduplicated per file so overlapping sibling-window ranges do not inflate the numerator.

The JSON report is the canonical output. Text and HTML renderers are derived views over that report.

## Incremental and live analysis

The same `PipelineSession` owns both full and incremental analysis state. `PipelineSession::update_files` reparses or drops changed files, then reruns signature building, LSH, optional embeddings, pair fusion, clustering, and report rendering over the current in-memory corpus.

On-disk fingerprint caching is opt-in in the CLI. `crates/deslop/src/main.rs` exposes `--incremental`; when it is set, `crates/deslop-core/src/fpcache.rs` stores normalized trees and fingerprints under `.deslop-cache/fingerprints/...` keyed by language, tool version, `min_nodes`, and content hash.

Live analysis is implemented under `crates/deslop-core/src/live/`:

- `session.rs` owns the live `AnalysisSession`.
- `scheduler.rs` serializes debounced file-change work.
- `debouncer.rs` uses a 250 ms quiet window and a 2000 ms cap.
- `watcher.rs` filters filesystem events by parser extension and exclusions.
- `api.rs` exposes report, range, cluster, embedding, and configuration operations.

The LSP server in `crates/deslop-lsp/src/backend.rs` wraps `LiveService` and exposes diagnostics, hover, code lens, and custom `deslop/*` methods. LSP embeddings start in `EmbeddingMode::Off` unless explicitly configured by the client.

The MCP server in `crates/deslop-mcp/src/` exposes JSON-RPC tools over stdio and protects filesystem inputs with `crates/deslop-mcp/src/safety.rs::resolve_within_root`. `find-similar` calls are forwarded over a Unix-domain socket (`.deslop-cache/deslop.sock`) to the LSP's live `AnalysisSession`, so snippet matching runs against the running corpus rather than a stale state file. Plain report queries (`top-offenders`, `report-get`, `report-for-file`, `report-for-range`) are served from the in-memory cache the MCP keeps over `.deslop-cache/live-report.json`; the MCP reloads that cache and pushes `resources/updated` + `deslop/reportChanged` whenever the LSP rewrites the file.

## Auditor verification map

| Claim | Verify in code | Useful tests |
| --- | --- | --- |
| C#, Rust, Python, and Dart are registered today. | `crates/deslop-core/src/pipeline/corpus.rs::default_parsers` | `cargo test -p deslop --test cli detects_type2_clone_in_csharp_fixture`, `cargo test -p deslop --test cli detects_type2_clone_in_rust_fixture`, `cargo test -p deslop --test cli detects_type2_clone_in_python_fixture` |
| Type-2 normalization collapses identifiers and literals. | `crates/deslop-core/src/lang/shared.rs`, language parser files | `cargo test -p deslop --test cli debug_ast_dump_matches_committed_golden` |
| Structural clones are BLAKE3 Merkle subtree hashes. | `crates/deslop-core/src/fingerprint.rs` | `cargo test -p deslop --test sibling_dedup` |
| Type-3 recall uses sibling windows and MinHash LSH. | `crates/deslop-core/src/sibling.rs`, `crates/deslop-core/src/tokens.rs`, `crates/deslop-core/src/lsh.rs` | `cargo test -p deslop --test sibling_ranking` |
| Embeddings are optional and off by default in the CLI. | `crates/deslop/src/main.rs`, `crates/deslop-core/src/pipeline/embedding_pass.rs` | `cargo test -p deslop --test cli default_run_records_embeddings_off_provenance` |
| Embedding neighbours are filtered by HNSW cosine threshold. | `crates/deslop-core/src/embedding/pairs.rs` | `cargo test -p deslop-core --test embedding_pairs` |
| Fused score is bounded. | `crates/deslop-core/src/pair.rs::PairScore::fused` | `cargo test -p deslop --test fused_score_bounds` |
| Cross-language comparison is disabled unless configured. | `crates/deslop-core/src/config.rs`, `crates/deslop-core/src/pair.rs::candidate_pairs_for_language_policy` | `cargo test -p deslop --test cross_language` |
| Live/LSP paths use `LiveService` over `PipelineSession`. | `crates/deslop-core/src/live/`, `crates/deslop-lsp/src/backend.rs` | `cargo test -p deslop-lsp --test notifications` |
| MCP tools are stdio JSON-RPC wrappers with root safety checks. | `crates/deslop-mcp/src/server.rs`, `crates/deslop-mcp/src/tools/mod.rs`, `crates/deslop-mcp/src/safety.rs` | `cargo test -p deslop-mcp --test cli` |

For a broad local audit, run the repository's CI target:

```bash
make ci
```

For a targeted code review, these searches show the core algorithm path:

```bash
rg -n "fn render|candidate_pairs_for_language_policy|cluster_by_transitive_closure|build_ranked_fused_clusters|render_report" crates/deslop-core/src
rg -n "default_parsers|EmbeddingMode::Off|DEFAULT_OLLAMA_MODEL|rank_weight" crates/deslop-core/src crates/deslop/src
rg -n "find_similar_snippet|find_similar_for_snippet|resolve_within_root" crates/deslop-mcp/src crates/deslop-core/src/live
```

## References

- Baxter, Yahin, Moura, Sant'Anna, and Bier, *Clone Detection Using Abstract Syntax Trees*, 1998. [PDF](https://leodemoura.github.io/files/ICSM98.pdf)
- Chilowicz, Duris, and Roussel, *Syntax Tree Fingerprinting for Source Code Similarity Detection*, 2009. [PDF](https://igm.univ-mlv.fr/~chilowi/research/syntax_tree_fingerprinting/syntax_tree_fingerprinting_ICPC09.pdf)
- Sajnani, Saini, Svajlenko, Roy, and Lopes, *SourcererCC: Scaling Code Clone Detection to Big Code*, 2016. [arXiv](https://arxiv.org/abs/1512.06448)
- Roy and Cordy, *NICAD: Accurate Detection of Near-Miss Intentional Clones Using Flexible Pretty-Printing and Code Normalization*, 2008. [ResearchGate entry](https://www.researchgate.net/publication/221219568_The_NiCad_clone_detector)
- Shrivastava and Li, *Densifying One Permutation Hashing via Rotation for Fast Near Neighbor Search*, 2014. [PDF](http://proceedings.mlr.press/v33/shrivastava14.pdf)
- Roy, Alam, Al-omari, Roy, Roy, and Schneider, *Unveiling the potential of large language models in generating semantic and cross-language clones*, 2023. [arXiv](https://arxiv.org/abs/2309.06424)
- Roy et al., *GPTCloneBench: A comprehensive benchmark of semantic clones and cross-language clones using GPT-3 model and SemanticCloneBench*, 2023. [arXiv](https://arxiv.org/abs/2308.13963)
- Eagal, Stolee, and Ore, *Analyzing the dependability of Large Language Models for code clone generation*, 2025. [Journal of Systems and Software](https://www.sciencedirect.com/science/article/pii/S0164121225002171)
- Liu et al., *Code Copycat Conundrum: Demystifying Repetition in LLM-based Code Generation*, 2025. [arXiv](https://arxiv.org/abs/2504.12608)
- Alam et al., *Are Classical Clone Detectors Good Enough For the AI Era?*, 2025. [arXiv](https://arxiv.org/abs/2509.25754)
- Wu et al., *An Empirical Study of Code Clones from Commercial AI Code Generators*, 2025. [FSE 2025 entry](https://conf.researchr.org/details/fse-2025/fse-2025-research-papers/111/An-Empirical-Study-of-Code-Clones-from-Commercial-AI-Code-Generators)
- Liu, Widyasari, Zhao, Irsan, and Lo, *Debt Behind the AI Boom: A Large-Scale Empirical Study of AI-Generated Code in the Wild*, 2026. [arXiv](https://arxiv.org/abs/2603.28592)
