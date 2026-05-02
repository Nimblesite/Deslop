---
layout: layouts/docs.njk
title: Research Background
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

- **LLM code repetition**: *Code Copycat Conundrum: Demystifying Repetition in LLM-based Code Generation* studies 19 code LLMs and reports that repetition appears across character, statement, and block levels, including structurally redundant code. The paper also evaluates a repetition-mitigation technique in open-source and industrial settings.
- **LLM-generated clones**: *Unveiling the potential of large language models in generating semantic and cross-language clones* evaluates GPT-3's ability to generate semantic and cross-language clone variants, which is directly relevant to Type-4 and cross-language duplicate detection.
- **Commercial AI code generators**: *An Empirical Study of Code Clones from Commercial AI Code Generators* reports Type-1 and Type-2 clone rates up to 7.50% for studied commercial generators, and discusses copyright, bug propagation, and vulnerability propagation risks.
- **AI-era clone detection**: *Are Classical Clone Detectors Good Enough For the AI Era?* evaluates nine clone detectors on GPTCloneBench and traditional clone benchmarks, highlighting why normalization and semantic variation matter when clones are AI-generated.
- **Technical debt in production repositories**: *Debt Behind the AI Boom: A Large-Scale Empirical Study of AI-Generated Code in the Wild* analyzes verified AI-authored commits and tracks static-analysis issues introduced by those commits. It is broader than duplication, but it supports treating AI-generated code as a technical-debt source that needs repository-level appraisal.

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

| Research line | What Deslop takes from it | Implementation pointer |
| --- | --- | --- |
| Baxter-style AST clone detection | Parse code into syntax trees, normalize irrelevant spelling, and compare tree structure rather than raw text. | `crates/deslop-core/src/lang/shared.rs`, `crates/deslop-core/src/lang/csharp.rs`, `crates/deslop-core/src/lang/rust_lang.rs`, `crates/deslop-core/src/lang/python.rs` |
| Syntax tree fingerprinting | Hash subtrees so exact structural clones become equal fingerprints; extend coverage with sibling sequences for near-miss clones. | `crates/deslop-core/src/fingerprint.rs`, `crates/deslop-core/src/sibling.rs` |
| SourcererCC-style token similarity | Use token k-grams and Jaccard similarity to find near-miss candidates without comparing every pair exactly. | `crates/deslop-core/src/tokens.rs`, `crates/deslop-core/src/lsh.rs`, `crates/deslop-core/src/pipeline/signatures.rs` |
| Locality-sensitive hashing | Use MinHash signatures and banding to produce scalable candidate pairs. | `crates/deslop-core/src/lsh.rs::minhash_signature`, `crates/deslop-core/src/lsh.rs::band_collisions` |
| Neural semantic clone detection | Use embeddings as an optional recall layer for behaviorally similar but syntactically different code. | `crates/deslop-core/src/embedding/*`, `crates/deslop-core/src/pipeline/embedding_pass.rs` |
| Hybrid clone detection | Union independent candidate sources, fuse scores, and cluster surviving pairs. | `crates/deslop-core/src/pair.rs`, `crates/deslop-core/src/cluster.rs` |

## Implemented pipeline

The batch CLI and live services both run through `PipelineSession`. The batch entry point in `crates/deslop-core/src/pipeline/run.rs` delegates to `PipelineSession::initialise`, and the current corpus is rendered by `PipelineSession::render` in `crates/deslop-core/src/pipeline/session.rs`.

### 1. File discovery

`crates/deslop-core/src/discover.rs::discover_files` walks the target root with the `ignore` crate, applies standard ignore filters, does not follow symlinks, filters by registered language extensions, applies `.deslop.toml` exclusion rules, and registers surviving files in a `FileRegistry`.

Supported language parsers are currently registered by `crates/deslop-core/src/pipeline/corpus.rs::default_parsers`: C#, Rust, and Python. TypeScript, JavaScript, Go, and other languages are not registered in the current core pipeline.

### 2. Parse and normalize

Each language parser uses tree-sitter through the shared machinery in `crates/deslop-core/src/lang/shared.rs`. `build_normalised_root` walks named tree-sitter nodes and produces a `NormalizedNode` tree rooted at `__file__`.

Normalization is language-specific:

- C#: `crates/deslop-core/src/lang/csharp.rs`
- Rust: `crates/deslop-core/src/lang/rust_lang.rs`
- Python: `crates/deslop-core/src/lang/python.rs`

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

The MCP server in `crates/deslop-mcp/src/` exposes JSON-RPC tools over stdio and protects filesystem inputs with `crates/deslop-mcp/src/safety.rs::resolve_within_root`.

One current MCP limitation is worth auditing carefully: `crates/deslop-mcp/src/backend.rs::find_similar_snippet` parses a snippet and checks that its normalized tree reaches `min_nodes`, but then returns the current top-N report clusters. It does not yet perform the same snippet-hash matching as `crates/deslop-core/src/live/session.rs::find_similar_for_snippet`. Auditors should treat MCP snippet search as a coarse report query until that function changes.

## Auditor verification map

| Claim | Verify in code | Useful tests |
| --- | --- | --- |
| Only C#, Rust, and Python are registered today. | `crates/deslop-core/src/pipeline/corpus.rs::default_parsers` | `cargo test -p deslop --test cli detects_type2_clone_in_csharp_fixture`, `cargo test -p deslop --test cli detects_type2_clone_in_rust_fixture`, `cargo test -p deslop --test cli detects_type2_clone_in_python_fixture` |
| Type-2 normalization collapses identifiers and literals. | `crates/deslop-core/src/lang/shared.rs`, language parser files | `cargo test -p deslop --test cli debug_ast_dump_matches_committed_golden` |
| Structural clones are BLAKE3 Merkle subtree hashes. | `crates/deslop-core/src/fingerprint.rs` | `cargo test -p deslop --test sibling_dedup` |
| Type-3 recall uses sibling windows and MinHash LSH. | `crates/deslop-core/src/sibling.rs`, `crates/deslop-core/src/tokens.rs`, `crates/deslop-core/src/lsh.rs` | `cargo test -p deslop --test sibling_ranking` |
| Embeddings are optional and off by default in the CLI. | `crates/deslop/src/main.rs`, `crates/deslop-core/src/pipeline/embedding_pass.rs` | `cargo test -p deslop --test cli default_run_records_embeddings_off_provenance` |
| Embedding neighbours are filtered by HNSW cosine threshold. | `crates/deslop-core/src/embedding/pairs.rs` | `cargo test -p deslop-core --test embedding_pairs` |
| Fused score is bounded. | `crates/deslop-core/src/pair.rs::PairScore::fused` | `cargo test -p deslop --test fused_score_bounds` |
| Cross-language comparison is disabled unless configured. | `crates/deslop-core/src/config.rs`, `crates/deslop-core/src/pair.rs::candidate_pairs_for_language_policy` | `cargo test -p deslop --test cross_language` |
| Live/LSP paths use `LiveService` over `PipelineSession`. | `crates/deslop-core/src/live/`, `crates/deslop-lsp/src/backend.rs` | `cargo test -p deslop-lsp --test notifications` |
| MCP tools are stdio JSON-RPC wrappers with root safety checks. | `crates/deslop-mcp/src/server.rs`, `crates/deslop-mcp/src/tools.rs`, `crates/deslop-mcp/src/safety.rs` | `cargo test -p deslop-mcp --test cli` |

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
