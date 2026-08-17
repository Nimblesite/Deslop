# Pipeline stages (v1, hybrid by default)

### [PIPELINE-LANG-TRAIT] Language plugin trait
The single extension point. Implementations live in `deslop-core::lang::<name>`. Each implementation provides: (a) tree-sitter grammar factory, (b) file-extension filter, (c) per-language node-kind normalization rules that collapse identifier / literal / trivia nodes into their structural kind. The trait output type (`NormalizedNode`) is identical across languages so downstream stages are language-agnostic. v1 ships with `csharp` (`tree-sitter-c-sharp`), `rust` (`tree-sitter-rust`), `python` (`tree-sitter-python`), `dart` (`tree-sitter-dart`), `javascript` (`tree-sitter-javascript`), `typescript`, `tsx` (`tree-sitter-typescript`), `php` (`tree-sitter-php`), `fsharp` (`tree-sitter-fsharp`, source grammar `LANGUAGE_FSHARP` for `.fs`/`.fsx`), and `go` (`tree-sitter-go`). Adding a language = one `LanguageParser` impl + pinning the grammar version in `Cargo.toml`. Shared walking / interning plumbing lives in `lang::shared`; JavaScript, TypeScript, and TSX additionally share `lang::ecmascript` so their normalisation surface stays aligned.

### [PIPELINE-DISCOVER-FILES] File discovery
Walk the target path with the `ignore` crate, respecting `.gitignore` and Git's standard ignore rules. Filter by the set of file extensions contributed by registered `LanguageParser`s. Additionally drop paths matching built-in or configured `[EXCLUSION-CONFIG]` `exclude` patterns — those files are never parsed. Every surviving path is registered with [STATE-FILE-REGISTRY] and downstream code traffics in `FileId`, never `Path`.

### [PIPELINE-NORMALIZE-AST] AST normalization
For each file, parse with the selected language's tree-sitter grammar and walk the resulting tree bottom-up, producing `NormalizedNode { kind: &'static str, children: Vec<Self>, byte_range, file_id }`. Identifier / literal / comment / whitespace nodes are collapsed to their structural kind so Type-2 clones (renamed identifiers) hash identically. Byte ranges are preserved and are the source of truth for any later rendering — line numbers are derived.

The synthetic `__file__` root spans the nodes normalization kept, not the tree-sitter parse root. The parse root covers leading and trailing trivia — a licence header, a comment block — that normalization has already dropped, so inheriting it reports bytes contributing zero nodes to the match: a whole-file occurrence opens on comments instead of the code it duplicates, and its start offset stops tracking edits that move that code. Real nodes keep their own span, because a declaration's braces belong to the duplication even when a comment sits between them. A file that normalizes to nothing keeps the parse root's span. Pinned by `crates/deslop-mcp/tests/issue_153_rescan_freshness.rs` ([LIVE-RESCAN-FRESHNESS]).

### [PIPELINE-BOILERPLATE-FILTER] Boilerplate-only clone filtering
Language front-ends classify syntax-only scaffolding before fingerprinting. Import declarations, C# `using` directives, namespace/package headers, Python decorators such as FastAPI route declarations, and equivalent module prologues are treated as **boilerplate carriers**, not business logic. A subtree or sibling window made only from these carriers is excluded from structural fingerprints, sibling-extension windows, token LSH input, and embedding input by default.

Rationale: clone detection literature and mature tools normalize or filter irrelevant syntactic features before comparison. Repeated import blocks produce high-copy false positives that drown out actionable duplication. They are still useful style signals, so the renderer may emit a low-noise action hint rather than a clone warning.

C# special case: if the same non-static `using` directives appear across many files in the same project, the human-facing hint is `Consider moving repeated usings to a global using file` and links to the affected namespaces. This is not a clone cluster and does not contribute to `weight` or `duplicated_loc`. The JSON/AI report may carry the suppressed byte ranges as `boilerplate_hints` so an agent can propose a safe `GlobalUsings.cs` or project-file `<Using Include="..." />` change.

Configuration:

- Default: boilerplate-only clones are suppressed and no hint is emitted (`boilerplate.imports = "suppress"`).
- Opt-in diagnostic mode: `.deslop.toml` can set `boilerplate.imports = "report"` under `[defaults]` or `[language.<id>]` to include them as low-severity hints for teams that explicitly want import hygiene audits. This mode does not restore import-only clone warnings; it emits structured `boilerplate_hints` instead.
- No mode may rank import/using-only ranges above executable or declarative code clones.

### [PIPELINE-FINGERPRINT-MERKLE] Structural fingerprint (Merkle)
Bottom-up Merkle hash over `NormalizedNode`. Each node's hash combines its own `kind` string with the ordered hashes of its children using `blake3`. Each node stores `(hash, subtree_node_count, byte_range, file_id)`. Nodes whose subtree size is below `--min-nodes` are excluded from clustering per [DECISION-MIN-NODES].

### [PIPELINE-CLUSTER-EXACT] Exact subtree clustering
Group `NormalizedNode` fingerprints by `hash`. Every bucket with ≥ 2 entries is a candidate clone cluster. Covers Type-1 and normalized Type-2 deterministically in O(n). Candidate pairs are language-scoped by default per [CONFIG-CROSS-LANGUAGE]; the exact same hash may still be compared across languages when `.deslop.toml` opts into cross-language comparison.

### [PIPELINE-CLUSTER-SUBSUME] Cross-cluster subsumption
One physical duplication is fingerprinted at several AST depths, so it can produce several clusters covering the same bytes — a duplicated method, and the run of single-statement clones inside it. Publishing both shows the user one duplicate twice and double-counts it in `clusters_total` and the duplication metric. Two independent questions decide the outcome.

**Are these one duplication?** Bidirectional coverage by per-occurrence containment: every occurrence of *each* cluster contains, or is contained by, an occurrence of the other in the same file. Pinned by `crates/deslop-core/tests/cluster_subsumption.rs`. Three weaker predicates each fail in a different direction:

- Requiring the whole occurrence *set* to nest misses the crossed case, where the depth difference falls on opposite sides in each file and neither set nests inside the other.
- Accepting bare *intersection* deletes findings: two duplicated regions sharing the single byte where one ends and the next begins are two findings, and no other cluster reports the one that is dropped.
- Accepting coverage in *either* direction alone deletes findings too. A wide cluster whose occurrences each happen to contain one member of a larger, differently-scoped cluster satisfies it — and a pair of byte-identical generated functions is then replaced by the one-line statement family nested inside them, which also reaches a file the functions never mention.

**Which view survives?** File coverage first, then physical enclosure, then precision.

- **A view that names a file the survivor does not name is never dropped.** No other cluster reports that file's duplication, so the finding disappears rather than moving. When each view names a file the other does not, both are published.
- Between views over one file set, the *enclosing* view is the duplication and the nested view re-describes it. Ranking weight must not decide: the fine-grained view always ranks heavier because it contributes one occurrence per statement, so weight-based selection renders a duplicated 60-statement method as 120 one-line occurrences and drops the method itself.
- Between views over one file set at the same nesting, the structurally more precise view wins. An embedding-dominant view survives a more precise structural rival: it carries semantic evidence over the same bytes that the rival cannot express.

### [PIPELINE-DETERMINISM] Cross-run determinism
Two runs of the pipeline over an unchanged corpus produce bit-identical deterministic output: identical MinHash signatures (blake3 XOF, fixed k-gram ordering), identical fused signal scores (`token_jaccard` compared bit-for-bit), identical candidate sets, cluster ids, and ranking. Determinism is what makes persisted processing ([PIPELINE-INCREMENTAL]) sound and cluster ids stable across sessions. The embedding/ANN layer is the only approximate stage and is bounded separately ([FUSION-EMBED-PROVIDER]); a missed ANN neighbour only loses recall, never changes existing cluster content.

Determinism holds over corpus *state*, not edit history: identical paths and bytes produce an identical report whatever sequence of edits got there. Every pipeline ordering is therefore keyed by workspace-relative path (with the registration id only as a tie-breaker), never by `FileId` alone — ids are append-only, so removing and restoring a byte-identical file would otherwise reorder the corpus, move the LSH star centre, and change rendered ranges and metrics for identical source. Rendered occurrence order follows the same path-ordered corpus. Pinned by the LSP `history_determinism` suite, which cycles a config exclusion over live files and asserts the restored report is field-for-field identical.

### [PIPELINE-INCREMENTAL] Persisted processing — the parse store
Deslop persists processing to disk and re-derives only what changed. Each stored artefact is a computation result addressed by exactly the content that determines it, and [PIPELINE-DETERMINISM] makes the stored result bit-identical to recomputing it — a content-addressed store with correctness invariants, not a discardable accelerator hint. "Cache" survives in the surface names (`.deslop/cache/`, `cache_stats`, `FingerprintCache`) and in hit/miss vocabulary; the semantics are the ones this section states.

The parse store holds one blob per `(language_id, tool_version, min_nodes, source_byte_hash)`. The hash is `blake3` over the file's **raw bytes** — never over a decoded string. A lossy decode collapses every maximal invalid UTF-8 subsequence to one U+FFFD, making the key non-injective: byte-distinct files share one entry and the second is served the first's tree and fingerprints (gh #382, pinned by `crates/deslop/tests/cache_key_lossy_utf8_collision.rs`). A hit rehydrates the structural fingerprints, the normalised AST, and the per-fingerprint MinHash signatures ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]) from a compact little-endian binary blob, so unchanged files skip tree-sitter *and* signature construction entirely; a miss parses the file, builds fingerprints and signatures, and persists the bundle. Before a hit is served, the fingerprints are re-derived from the rehydrated tree and compared with the stored records: any disagreement voids the blob — the stored signatures are positionally bound to the stored fingerprint list, so they are unattributable the moment that list cannot be reproduced — and the file takes the miss path, whose store self-heals the blob. Any mismatch on the key — tool upgrade, grammar pin, `--min-nodes` change, source edit — degrades gracefully to a miss; stale blobs never leak into a run.

**Activation.** On by default on every surface. Incremental analysis is a first-class path, not a bolt-on: the LSP runs on it permanently, and a batch CLI run is just "incremental starting from an empty store". `deslop --no-incremental` opts out per invocation for callers that must not write to the tree at all; `[analysis] incremental = false` in `.deslop.toml` ([CONFIG-INCREMENTAL-OPTOUT]) opts the whole workspace out on every surface without a flag. Stats land on every report as `cache_stats { hits, misses }` at top level. Text renderer surfaces them as `cache: N hit / M miss`.

**[PIPELINE-INCREMENTAL-INVALIDATION] Invalidation is addressing, not bookkeeping.** The store cannot serve a stale parse, because a stale parse is *unaddressable*: the blob's filename is `blake3(file contents)`, under path segments for language, tool version, and `min_nodes`. Edit a file with nothing watching — an agent writing, a `git checkout`, an editor with the LSP stopped — and its content hash changes, so the lookup lands on a path that does not exist and the file is re-parsed from disk. There is no mtime heuristic, no watcher-maintained index, and no invalidation step that could be skipped or get out of sync.

Two further properties make persistence-on-by-default safe for the CLI:

- **Corpus membership never comes from the store.** Every run performs a fresh discovery walk, so files added or deleted while nothing was watching are picked up regardless of store state. A deleted file's blob is orphaned — never consulted, kept as revert reuse until [PIPELINE-INCREMENTAL-RETENTION] prunes it.
- **A warm run and a cold run agree.** Wiping `.deslop/cache/` changes the `cache_stats` counters and nothing else about the report. The store never defines results — the source tree is the only source of truth; persistence only decides how much of it must be re-derived.

The one artefact that *can* go stale is the live state file `live-report.json` ([LIVE-STATE-FILE]) — a whole-report snapshot, not a content-addressed entry. The CLI never reads it. The LSP seeds from it for instant warm-start and immediately runs a cold pass that replaces it, reporting `Running` until that pass installs ([LIVE-CACHE-SEED]).

**Layout.** `<root>/.deslop/cache/fingerprints/<language_id>/<tool_version>/<min_nodes>/<source_byte_hash>.bin`. Shares `.deslop/cache/` with the embedding cache from [FUSION-EMBED-PROVIDER]; the two layers invalidate independently.

**Format.** `u32` magic, then a 32-byte **binding digest** ([PIPELINE-INCREMENTAL-INTEGRITY]), then the payload: a recursive `NormalizedNode` tree (`u32 kind_len`, kind UTF-8 bytes, `u64 start`, `u64 end`, `u32 child_count`, children...), then `u64 fingerprint_count` followed by one `{ [u8;32] hash, u64 start, u64 end, u64 node_count }` record per fingerprint, then `u64 signature_count` followed by one 128×`u64` MinHash signature per fingerprint, positionally 1:1 with the fingerprint records. Decode rejects any blob whose signature count disagrees with its fingerprint count, and any blob whose payload does not consume the file exactly. No serde, no schema drift: the magic + tool-version path segment bracket every format change — the pre-signature and pre-digest layouts' magics decode as a plain miss and their blobs are rewritten in the current format.

### [PIPELINE-INCREMENTAL-INTEGRITY] A blob is bound to its address

The filename alone proves nothing about the bytes inside it: a corrupted payload keeps its filename, and a blob moved, swapped, or copied to another valid address decodes cleanly there. Both were reproduced serving wrong reports — corrupted MinHash payloads flipped `token_jaccard`, and a two-blob swap exchanged two files' rendered spans and buckets — so blob trust is part of the accuracy surface, not an optimisation detail.

Every blob therefore carries a BLAKE3 **binding digest** over its payload and the full address that wrote it: language id, `min_nodes`, source-byte hash, the layout revision (the magic), the signature width, and a **semantic epoch** — a constant bumped when parsing, normalisation, fingerprinting, or signature construction changes meaning without changing layout, deliberately independent of the reused `0.0.0-dev` package version in the directory path. A lookup recomputes the digest from its *own* address before decoding anything. Corruption anywhere in the file, a blob under the wrong source hash, a blob copied across a language or `min_nodes` partition, trailing bytes, and a stale epoch all fail identically: a plain miss that re-parses from source and self-heals the blob, with the next pass hitting cleanly.

Corrupt bytes may never crash the run either. Every decode-side length field is proven against the bytes actually remaining before it sizes an allocation, and the blob file's own length is bounded before it is read — a corrupt count degrades to `InvalidData`, never a capacity-overflow abort. After the digest verifies, the served hit is still cross-checked by re-deriving fingerprints from the rehydrated tree ([PIPELINE-INCREMENTAL]), defence in depth against an encoder bug the digest would faithfully sign.

Pinned end-to-end by `crates/deslop/tests/cache_blob_integrity.rs` (tampered signature payload, same-partition blob swap, cross-language blob copy, truncation / trailing garbage / zeroed interior — each asserting exact miss accounting, truth-report equality, and clean healing) and at the unit level by `crates/deslop-core/src/fpcache/tests.rs` (wrong-address bindings, superseded magics, count and length bombs with valid digests, oversized files).

**Failure modes.**

- Corrupt, truncated, misaddressed, or oversized blob → treated as a miss, logged at `warn!`, overwritten by the next successful parse.
- Cache directory unavailable (permissions, read-only fs) → `FingerprintCache::open` fails, the pipeline falls back to the full parse path for the affected language, logs `warn!`, keeps running.
- Blob write fails (e.g. disk full) → `warn!`, return the in-memory result, pipeline continues.

Zero-zero stats indicate the pass ran without the store (`--no-incremental` passed, the `[analysis] incremental = false` config opt-out ([CONFIG-INCREMENTAL-OPTOUT]) applied, or discovery yielded nothing). Any non-zero counter proves the store was consulted.

**Scope.** [PIPELINE-INCREMENTAL] governs persistence for the parse stage and for the per-fingerprint MinHash signatures stored beside it ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]) — the dominant cost of the LSH block. Everything further downstream — band collision enumeration, candidate pairing, clustering, ranking, metrics, rendering — recomputes in full on every pass regardless of how many files changed. Making that remaining cost track the size of the change is the rest of [PIPELINE-INCREMENTAL-ANALYSIS].

### [PIPELINE-INCREMENTAL-RETENTION] The store prunes itself after every full pass

A full pass is the one moment the live blob set is exactly known — every admissible file was read, so every blob the corpus can address is enumerable. Retention runs there and only there: never on a single-file change pass, and never when the store is disabled (the opt-out leaves the store untouched, [CONFIG-INCREMENTAL-OPTOUT]).

- **Stale tool-version partitions are always removed.** A `<language>/<version>` directory for another tool version is unaddressable by construction and can never hit again.
- **Orphans are kept while the store is under budget.** A blob in the current partition whose source bytes left the corpus is exactly the content-addressed reuse set for a revert or a branch switch — [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] asserts a revert full-hits the store — so eager orphan removal would be a recall regression against that contract.
- **The budget is 2 GiB** over the whole fingerprint store (~11× the pinned tokio benchmark's 185.8 MiB store). Over budget, eviction is provable-orphans first, then oldest modification time, path as the deterministic tie-break, stopping the moment the store fits. Blobs under other `min_nodes` partitions are never provably orphaned — a different invocation may still address them — and are age-ranked only. Evicting any blob is correctness-free: the next pass that addresses it misses, rebuilds from source, and self-heals ([PIPELINE-INCREMENTAL-INVALIDATION]).
- Only `.bin` blobs are retention's to manage; foreign files are never touched. Every step is best-effort — an unremovable entry is skipped, never an error. The sweep logs counts only: `fingerprint store swept { stale_partitions, orphan_blobs, evicted_blobs, store_bytes }`.

Pinned end-to-end by `crates/deslop/tests/cache_retention.rs` (stale-partition removal with live blobs byte-unchanged, an edit cycle whose kept orphan lets the revert full-hit, disabled-store passes leaving the store untouched) and at the unit level by `crates/deslop-core/src/fpcache/retention/tests.rs` (orphan-before-live eviction order, oldest-first fallback, budget stop condition, foreign-file safety, other-partition non-provability).

### [PIPELINE-INCREMENTAL-ANALYSIS] Incremental analysis
⏳ **Partially implemented.** Signature reuse ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]) is implemented and pinned by `crates/deslop/tests/signature_reuse.rs`; the equivalence contract is enforced end-to-end by `crates/deslop/tests/incremental_equivalence.rs`, and across six languages sharing one store by `crates/deslop/tests/incremental_multilang_golden.rs` (committed cold golden, warm reproduction) and `crates/deslop/tests/incremental_multilang_matrix.rs` (per-language touch, delete, revert, edit-chain and parser-partition scenarios). The remaining downstream stages are tracked by gh #383, designed in [`plans/incremental-analysis-plan.md`](../plans/incremental-analysis-plan.md).

An **incremental pass** is one that is given the set of files whose content changed since a previous pass over the same corpus, and is permitted to reuse work from that pass. A **cold pass** reuses nothing.

**[PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] An incremental pass owes the cold report.** For any corpus state reachable by any sequence of edits, the report produced by an incremental pass must equal the report a cold pass produces for that same state — field for field: cluster ids, occurrence paths and byte ranges, bucket, every signal, ranking order, `metrics`, and `clusters_hidden`. `cache_stats` is the sole permitted difference. This follows from [PIPELINE-DETERMINISM] holding over corpus *state* rather than edit history: if two paths to the same state can produce different reports, the incremental path is wrong, not the cold one. A performance gain that costs equivalence is not a gain.

**[PIPELINE-INCREMENTAL-ANALYSIS-REUSE] What may be reused.** Any value that is a pure function of content that did not change. Concretely: a MinHash signature is determined by one subtree's normalised token k-grams; a pair's structural and token-Jaccard scores are determined by its two subtrees. Neither depends on the rest of the corpus, so neither needs recomputing when the rest of the corpus is untouched. Values that depend on corpus-wide state — ranking weights, repo metrics, the duplication percentage — are derived from the assembled cluster set and are recomputed every pass.

*Implemented for per-language MinHash signatures.* Each file's signatures are built once at parse/load time and persisted in its parse-store blob, positionally 1:1 with its fingerprints ([PIPELINE-INCREMENTAL] Format); a warm pass attaches them instead of rebuilding, on both the batch and the live splice path. The reuse is observable as the `signatures_built` / `signatures_reused` structured fields on the `fingerprint corpus built` tracing event — a fully-warm pass reports `signatures_built=0` with `signatures_reused` equal to the fingerprint count. Cross-language signatures ([CONFIG-CROSS-LANGUAGE]) are opt-in audit state and stay render-time.

**[PIPELINE-INCREMENTAL-ANALYSIS-ADDRESSING] Reuse is addressed, not bookkept.** Reused artefacts follow [PIPELINE-INCREMENTAL-INVALIDATION]: each is stored under a key derived from the content that determines it, so a stale artefact is *unaddressable* rather than merely unused. No mtime heuristics, no watcher-maintained validity index, no invalidation step that could be skipped or drift. A key derived from anything other than that content — a version string, a path, a lossy transform of the bytes — does not satisfy this and is a defect, not an optimisation. The key's language component is load-bearing in the same way: two byte-identical files routed to different parsers must occupy two entries, or the second is served a tree built under a grammar it was never parsed with. `crates/deslop/tests/incremental_multilang_matrix.rs` pins that partition against a mixed six-language corpus.

**Corpus membership never comes from reuse.** As with the parse cache, every pass performs a fresh discovery walk. Files added or removed while nothing was watching are picked up regardless of what state was carried forward.

### [PIPELINE-RANK-WORST-FIRST] Ranking: worst offenders first
Before ranking, each cluster's occurrences are reduced to one member per **transitively overlapping run** per file. Fingerprinting emits one subtree per AST node, so a duplicated region yields a nest of overlapping windows over the same bytes; publishing more than one inflates the occurrence count, the cluster size, and the duplication percentage. Overlap is transitive, so the run's frontier is tracked separately from its representative: for `[0,100]`, `[90,110]`, `[105,200]` the bridging window is the narrowest and loses the width contest, and a sweep that tests the next window against the representative alone reports one region as two. The widest window of each run is the reported location; a cluster left with one location is not a duplicate and is dropped. Pinned by `crates/deslop-core/tests/cluster_overlap_collapse.rs`.

`weight = clone_node_count × (cluster_size − 1) × log2(1 + total_spanned_loc)`. Clusters are sorted by weight descending. A cluster with one member (no duplication) scores zero by construction. Later stages multiply in the fusion score from [FUSION-STRATEGY-BOUNDED-MAX]. For rendered (visible) ordering, `cluster_size` counts only non-hidden occurrences, so a mixed cluster's [EXCLUSION-CONFIG] `report_hide` members do not push it above fully-actionable clusters. The final ranking weight is multiplied by the clone-category coefficient from [RANK-CATEGORY] before the visible sort, so a data-table cluster ranks below comparable logic clones.

### [RANK-CATEGORY] Clone category and the ranking policy
Every cluster carries a **clone category** that is orthogonal to the similarity bucket of [taxonomy.md §CLONE-BUCKETS](taxonomy.md#clone-buckets). The canonical category table (seven values including the literal family) lives at [taxonomy.md §CLONE-CATEGORY-REGISTRY](taxonomy.md#clone-category-registry); this section governs the two fragment-clone categories. The bucket answers *"how similar are these copies?"*; the category answers *"is this repetition extractable logic or un-refactorable data?"*:

- `logic` — ordinary duplicated code. Full ranking weight. The default.
- `data` — a data-structure literal repeated across sibling elements (e.g. a top-level `List<Model>` of near-identical constructor literals). Real repetition, but the constructor's purpose *is* to enumerate per-row fields; at best a user hoists a builder with defaults or moves the rows to a JSON/CSV/asset. Detected by [CLONE-NOISE-DART-DATA-TABLE-LITERAL].

The category drives a **three-way ranking policy** configured in `.deslop.toml` under `[ranking]` (see [CLONE-NOISE-DART-DATA-TABLE-LITERAL] for the keys):

- **keep** — both categories rank at full weight. Restores pre-category ordering.
- **demote** (default) — `data` clusters are multiplied by `data_clone_weight` (default `0.15`, strictly in `(0.0, 1.0]`) so they rank below comparable `logic` clones but remain in the report, labelled `category="data"`. The multiplier is never zero, so a pathologically large verbatim blob can still rise.
- **ignore** — `data` clusters are dropped from the report entirely (reuses the [EXCLUSION-CONFIG] cluster-hide path) and counted under `clusters_hidden`.

`data` clusters carry a category-specific action hint ("consider a builder with default args, or move the rows to a JSON/CSV/asset") instead of the "extract the duplicate" hint. The category and its label travel on the JSON `ReportCluster.category` field so every downstream surface — text, HTML, and the VSIX tree — orders and labels identically from one source of truth ([OUTPUT-SCHEMA-JSON]).

### [RANK-STRUCTURAL-ONLY] Structural-only evidence and the ranking policy

`StructuralOnly` is the [taxonomy.md §CLONE-BUCKETS](taxonomy.md#clone-buckets) bucket for clusters whose **only positive evidence is the normalized AST shape** — `structural ≥ 0.99` with token and embedding support both below `STRUCTURAL_ONLY_MAX_SUPPORT` (0.05). Normalization strips identifiers and literals, so a sibling method family (REST CRUD endpoints, settings getters, builders) collides into one shape; the exact-structural pass also leaves `token_jaccard` *unscored* at `0.0`, so the triple alone cannot distinguish boilerplate from a true Type-2 rename. The history (#134 → #154 → #169 → #197) shows why shape-specific suppressions alone never closed the hole: each fix allowlisted one geometry (≥3-file scaffolding; single-file declaration families; Dart field registries) and every other geometry kept full `NearlyIdentical`-grade weight.

This section closes the hole structurally:

1. **One predicate.** `deslop-core::buckets::is_structural_only_signals` is the single source of truth, shared by the bucket routing (the wire label) and the ranking demotion. A cluster labelled `structural_only` is by construction the cluster the policy demotes — the #197 label/ranking divergence cannot recur.
2. **Weight policy.** The `[ranking]` section gains `structural_only = "demote" | "ignore" | "keep"` (default **demote**) and `structural_only_weight` (default `0.15`, strictly in `(0.0, 1.0]`), exactly parallel to [RANK-CATEGORY]'s data knobs and validated by the same rule. The multiplier folds into the visible re-rank next to the category coefficient, so a shape-only family sinks below comparable token- or semantics-supported clones regardless of its file spread or declaration shape.
3. **Existing suppressions stay.** Cross-file ≥3-member/≥3-file scaffolding still demotes to `LooselySimilar` (hidden, #134); single-file sibling-declaration families are still hidden by the AST pass (#197); Dart data registries stay with [RANK-CATEGORY] (#169). The weight policy catches everything those shapes miss (e.g. two-file method families split by Dart `part`/extension idioms).
4. **Editor override.** The VS Code setting `deslop.ranking.structuralOnly` (`default` | `demote` | `ignore` | `keep`, [VSIX-SETTINGS-RANKING]) feeds `deslop-lsp --ranking-structural-only`, recorded once at startup in the central state module (`deslop-core::state`) and consulted by every config load — the editor channel wins over `.deslop.toml`; `default` defers to it.
5. **Filterable.** The MCP `duplicates` filter block ([MCP-TOOL-FILTERS]) derives its `buckets` enum from `ClusterKind::all()`, so `structural_only` is filterable by agents (#195/#197).

### [RANK-STRUCTURAL-ONLY-FORWARDING] Proving a declaration is family noise

The single-file hide in [RANK-STRUCTURAL-ONLY] needs a proof that a member is
scaffolding. Two window shapes qualify, and both are AST facts about what the
fingerprint window *covers* — never a count of cluster members and never a count
of statements:

1. **Plural siblings.** The window intersects two or more named members of one
   declaration container (`class_body`, `declaration_list`, …). Container members
   are counted rather than per-language declaration node kinds, because
   tree-sitter-dart has no `method_declaration` at all: a Dart class member is a
   generic node identified by the `function_body` it carries.
2. **One forwarding declaration.** The window covers exactly one member whose
   body is `return <expr>;`, `<binding> = <expr>; return <reads binding>;`, or a
   bare arrow expression; consists **only** of nodes from a closed declarative
   allowlist — calls, member access, awaits, literals, payload collections,
   casts and adapter lambdas; and in which **every call is transport**.

   A call is transport when it **delegates to a collaborator** — a member-access
   call whose receiver identifier names an instance field of the enclosing
   container (a declared field or a `this.x` constructor parameter) or one of the
   member's own formal parameters — or when it **only consumes what a delegation
   produced**: it reaches a delegation at all, and every argument it passes
   derives from one. Delegated data is the result of a delegating call, a local
   binding whose initialiser reaches one, and the parameters of any callback
   declared in the body.

   Both halves are load-bearing. Containing a call is not forwarding: a pair of
   one-line siblings that hand different literals to a *sibling helper on the
   same class* is parameterisable business logic, and a bare identifier callee, a
   `this.method(...)` self-call and a static `Type.factory(...)` all fail the
   receiver resolution. Nor is one delegating call a licence for the rest of the
   body: `client.fetch(order)` followed by `applyMarkup(gross, "standard", 100)`
   delegates and still hides a liftable pair, because the literals the sibling
   helper receives are what parameterising it would absorb. A literal, a member
   parameter, or a bare sibling reference in argument position is the class
   computing on its own inputs.

Shape 2 is what the real #197 surface is. Its `resetX` wrappers are one statement
each, so every window covers one declaration and shape 1 can never reach them.
They also show why a **call count** cannot stand in for the transport test: every
one of them makes two calls — `_getTask(http.deleteMethod(route))` wraps the
client call in a sibling helper, and `IndexSettings.fromMap(response.data!)`
decodes what came back. Both consume only the client's response, so both are
transport, and requiring a single call per body would convict the family this
filter exists for.

Branches, loops, arithmetic, comparisons, mutation and every node kind outside the
allowlist — including a parse `ERROR` — disprove forwarding. The predicate
therefore fails open: a body the walk does not fully understand keeps its cluster
visible. That direction is mandatory for a filter that deletes output.

**A statement count may not stand in for this.** "One or two statements means
scaffolding" convicts a short body carrying a loop and an accumulator, and acquits
a wrapper that spends two statements on a temporary response. The proof matches
the data-flow shape instead, which is what separates a REST wrapper from a
parameterisable method whose body binds locals, calls several collaborators and
branches.

Two further guards bound the hide. Content evidence must show the members differ
in substance ([FUSION-CONTENT-GATE]). And **no two proven wrappers may share a
body**: two sibling wrappers forwarding to the same route are a copy-paste bug —
one of those calls is dead or misaimed — and one shared body disqualifies the
suppression for the whole family. The comparison is on bodies, not on reported
windows: sibling declarations differ in their method names, so their windows
never compare equal even when the duplication is exact.

Languages without a wired grammar table return false for shape 2, so their
single-declaration windows are never hidden.

### [RANK-LITERAL-FAMILY] Literal-family weight formula and policy

Literal-family clusters ([CLONE-CATEGORY-REGISTRY] categories `magic_literal`,
`shadowed_constant`, `constant_duplicate`, `constant_drift`, `constant_alias`) interleave in the
**same** worst-first list as fragment clones — no separate section, no second sort; the facets
([FACET-MODEL]) are how users isolate them. Their base weight is a deliberate, documented fork of
[PIPELINE-RANK-WORST-FIRST], because `clone_node_count` degenerates at 1 for a bare literal token:

`weight = max(visible_occurrences − occurrence_floor + 1, 1) × length_factor × log2(1 + distinct_files)`

where `occurrence_floor` is the category's minimum (3 for `magic_literal`, 2 elsewhere) — the
`max(…, 1)` keeps the term positive when `report_hide` hides occurrences after the trigger
counted them; `length_factor = min(content_chars(normalized_value), 40)` with `content_chars` =
Unicode scalar count of the normalised value (escape sequences counted as spelled; for numbers,
the canonical normalised spelling; for `constant_drift`, the longest variant value); and
`distinct_files` counts files with visible occurrences. A literal-family cluster with fewer than 2
visible occurrences is dropped and counted in `clusters_hidden`, matching the fragment-clone rule.
The linear occurrence term follows Sonar's per-occurrence remediation model; the file-spread log
term mirrors the existing `spanned_loc` factor and rewards the cross-file repetition that the
literature ties to faults. Micro-findings are **not** down-ranked for being small — micro-clones
are measurably more bug-prone than regular clones
([reading-list.md](reading-list.md#read-list-literals)); a 40-site magic URL
belongs in top offenders.

Policy knobs in `[ranking]` (same `keep | demote | ignore` enum, validation, and
`clusters_hidden` accounting as the data/structural-only knobs; editor channel per
[LITERAL-CONFIG]):

- `magic_literals = "keep"` (default) with `magic_literal_weight = 0.3` applied only under
  `demote`. Default-keep is the evidence call: noise is controlled at detection time
  ([LITERAL-NOISE], [LITERAL-CENSUS]), not by hiding confirmed findings.
- `constant_findings = "keep"` (default) with `constant_findings_weight = 0.5` under `demote` —
  one knob covering `constant_duplicate`, `constant_alias`, and `shadowed_constant`.
- `constant_drift = "keep"` (default) | `"ignore"` — **no demote**: same-name-conflicting-values is
  a correctness risk, never quietly down-weighted. Drift clusters stamp `nearly_identical`
  ([LITERAL-WIRE]), which resolves to the warning tier under the default bucket-keyed severity
  maps — severity has no category channel; it is keyed by bucket only (#177 tracks a category-keyed
  override as out of scope here).

### [RANK-UNUSED-PUBLIC] Unused-public-constant boost (monorepo)

When the [LITERAL-UNUSED-MARKER] (literals.md) fires for **every** declaration occurrence in a
constant-family cluster, the cluster's weight is multiplied by `unused_public_weight` — an
**up**-weight, mirroring the `ClonePolicy` pattern with an inverted validation range:

- `[ranking] unused_public = "boost"` (default) | `"ignore"`.
- `unused_public_weight = 1.5`, finite, validated in `[1.0, 10.0]`.

The boost (and the marker itself) only activates in a monorepo with a non-publishable declaring
package (`[workspace] monorepo`, [LITERAL-UNUSED-MARKER]) — published public constants are exempt by
design. Conservative 1.5× because confidence caps at 90: the marker raises a duplicate constant's
priority, it never asserts deletability.

### [STATE-FILE-REGISTRY] File registry (the only global state)
`deslop-core::state::FileRegistry` maps `FileId ↔ PathBuf`. This is the *only* place mutable state associated with a pipeline run may live. Instances are per-run (not process-global) so a future long-running daemon can keep multiple analyses side-by-side.

### [OUTPUT-SCHEMA-JSON] Canonical JSON schema
JSON is the canonical report format ([PRINCIPLES-AUDIENCE-AGENT]). Text and HTML are derived from it — nothing lives in two places. Text is terse and AI-readable (ASCII, line-oriented, no colour). HTML is single-file, inline-CSS, human-readable, and embeds the same `action_hints` the JSON carries so a human opening the file cold understands what they are looking at; its schema-reference section renders only when the report carries a non-empty `schema_doc`, which the CLI's rendered reports do not (see below).

Top level:

- `tool_version: String` — producer binary version.
- `min_nodes: u32` — subtree size floor used for the run.
- `files_analysed: usize` — count of files actually parsed.
- `clusters_hidden: usize` — clusters that existed but were suppressed from `clusters` because every occurrence matched a [EXCLUSION-CONFIG] `report_hide` pattern. Surfaces the volume of ignored duplication without leaking the content.
- `cache_stats: { hits: usize, misses: usize }` — incremental fingerprint-cache telemetry per [PIPELINE-INCREMENTAL]. Both zero when `--no-incremental` was passed; otherwise `hits + misses == files_analysed` for files whose language has a registered parser.
- `metrics: RepoMetrics` — repo-wide duplication totals per [METRICS-REPO]. Always populated; zero when no duplication exists.
- `schema_doc: String` — markdown explaining every field, signal, threshold, ranking formula, byte-range convention, and clone taxonomy, sourced via `include_str!` from `REPORTING-CONTEXT.md` so it cannot drift from the schema. The CLI ships the field **present but empty** in every rendered report (#110/#111) — inlining ~13 KB into each report drowns the actual content — and the document is served on demand instead (`schema-doc` tool, `deslop://schema` resource per [MCP-*]). Pinned by the committed report golden.
- `action_hints: Vec<ActionHint>` — short playbook entries ("high structural + high jaccard → extract shared function", etc.) agents can consult before deciding how to act.
- `embedding_provenance: Option<EmbeddingProvenance>` — provider/model identity plus `attempted_subtrees`, `indexed_subtrees`, and `failed_subtrees` so embedding coverage is visible. Duplicate successful snippets collapse before ANN indexing, and provider-rejected subtrees are omitted from the embedding ANN input; they are never represented as zero vectors.
- `clusters: Vec<ReportCluster>` — ranked worst-offenders-first per [PIPELINE-RANK-WORST-FIRST].

`ReportCluster`:

- `id`, `weight`, `size`, `canonical_node_count`, `signals { structural, token_jaccard, embedding_cos, fused }`, `summary` — as in v1.
- `interpretation: String` (new in v2) — one-line synthesis computed from the signal combination ("Type-1 exact clone, safe to extract", "Type-3 near-miss, review before merging", "Low-information LSH-only match, treat as hint"). Derived, so rendering is deterministic.
- `occurrences: Vec<ReportOccurrence>` — each with `path`, `start_byte`, `end_byte`, and `hidden: bool` (true when the occurrence matched a `report_hide` pattern per [EXCLUSION-CONFIG]).

`--from-report <file.json>` skips analysis and re-renders the text + HTML views from a canonical JSON report. Keeps the rendering pipeline testable in isolation and makes re-formatting a cached report free.

The default invocation writes all three formats to disk (`.deslop/deslop-report.{json,txt,html}` under the scan root per [OUTPUT-DIR], or `<path>.{json,txt,html}` when `--output <path>` is given). `--nojson`, `--notext`, `--nohtml` suppress individual formats; at least one must remain enabled.

### [OUTPUT-DIR] Workspace output directory
Everything Deslop writes for a scanned workspace lands under a single `.deslop/` directory at the **scan root**, so a user has exactly one path to gitignore, inspect, or delete, and the three surfaces never disagree about where a workspace's artefacts live:

```text
<scan-root>/
  .deslop.toml                     # config — user-authored, tracked, NOT output
  .deslop/                         # everything Deslop writes
    deslop-report.{json,txt,html}  # rendered reports ([OUTPUT-SCHEMA-JSON])
    deslop-report.delta.json       # generation delta, when `--rerun-touch` ran ([LIVE-DELTA])
    logs/deslop-<unix-seconds>.log # tracing sink ([UX-LOG-CONSOLE])
    cache/                         # derived state — safe to delete, always rebuildable
      fingerprints/                # [PIPELINE-INCREMENTAL]
      embeddings/                  # [FUSION-EMBED-PROVIDER]
      live-report.json             # [LIVE-STATE-FILE]
      deslop.sock deslop.port      # [LIVE-IPC-SOCKET], [LIVE-IPC-TCP]
```

`deslop-core::paths` is the single source of truth for this layout; the CLI, LSP, and MCP all resolve through it rather than joining path literals of their own. Three consequences are normative:

- **The scan root, not the working directory, anchors the default.** `deslop /other/repo` writes into `/other/repo/.deslop/`, matching where the LSP and MCP already read and write for that workspace. A CLI run therefore never litters the directory the operator happened to be standing in.
- **`--output <prefix>` overrides the report base, and the logs follow it** into `<prefix-dir>/logs/`. It is the only knob: the cache stays at `<scan-root>/.deslop/cache` because the LSP and MCP must locate it from the scan root alone, with no flags to consult.
- **Logs get their own subdirectory.** Report file names are fixed and few; log file names are timestamped and accumulate, so they never bury the three files a user actually opens.

`.deslop/` is dot-prefixed, so the discovery pass's hidden-directory prune keeps Deslop's own artefacts out of the corpus it analyses. The `.gitignore` entry the VSIX offers to write is the directory-only form `.deslop/` ([VSIX-CACHE-IGNORE]), which leaves the sibling `.deslop.toml` config file tracked.

### [OUTPUT-HUMAN-HTML] Human-readable HTML mode

The default HTML renderer embeds, for each occurrence, the source bytes covered by `[start_byte, end_byte)` inside a collapsible `<details>` panel with line numbers and tree-sitter-driven syntax highlighting (server-side, no JS). Snippets are computed at render time from the source tree — not added to the JSON schema. `--human=off` falls back to the terse byte-offset-only HTML.

#### [OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS] Per-language sections

`[report] split_by_language` in `.deslop.toml` (default `false`, with a `--split-by-language` CLI mirror) divides the report body into one `<section>` per language instead of the single "Duplicate groups" section. In both modes cluster cards group into per-bucket expanders within each section ([FACET-HTML]). With the flag **off** the output is byte-identical to the single-section form — a hard no-regression invariant. With it **on**, `write_clusters` groups clusters by their canonical occurrence's `language_for_path(...)`, emits one `<h2>` per language carrying the language's display name and its group count, preserves worst-first order within each section, and orders sections by their worst cluster weight. The intro summary line ([OUTPUT-HUMAN-HTML]) gains a per-language breakdown. Each cluster is single-language ([CONFIG-CROSS-LANGUAGE]), so every group lands in exactly one section.

### [METRICS-REPO] Repo-wide duplication metrics

Two honest numbers, computed deterministically from the same visible cluster set the report already carries, living at `Report.metrics` and driving the fail-over gates in [EXIT-CODES]: the **mechanical** percentage below — pure line coverage, never weighted, the default gate — and the **evidence-weighted** companion of [METRICS-REPO-WEIGHTED].

`RepoMetrics` fields:

- `analysed_loc: u64` — physical lines across every file in `files_analysed`. Counted once per file, regardless of clustering. Lines are `\n`-terminated plus the trailing partial line if any; empty files contribute zero.
- `duplicated_loc: u64` — lines covered by **≥ 2 clone occurrences across the whole corpus**, deduplicated per file so overlapping sibling-extension ranges do not double-count. Computed by projecting every `ReportOccurrence` from every non-hidden cluster onto a per-file `BTreeSet<line>`, unioning, and summing set sizes. Hidden occurrences (`[EXCLUSION-CONFIG]` `report_hide`) are **excluded** so a noisy generated-code tier cannot inflate the metric. Literal-family clusters ([RANK-LITERAL-FAMILY]) are **excluded** from `duplicated_loc` / `duplication_percent` — the headline percentage keeps meaning fragment-clone duplication; `clusters_total` still counts them. Every visible bucket counts here at equal weight: a `structural_only` line is the same one line as a byte-proven `identical` line. That is deliberate — this is the coverage measure — and it is also why the metric can overstate actionable duplication (gh #344, #355); the bucket-sensitive view is [METRICS-REPO-WEIGHTED], never this field.
- `duplication_percent: f64` — `100.0 × duplicated_loc / analysed_loc`, clamped into `[0.0, 100.0]`. Zero when `analysed_loc == 0`. Rounded to two decimals in text + HTML; carried at full `f64` precision in JSON.
- `clusters_total: usize` — count of non-hidden clusters carried in `clusters`, literal-family included; always equals `clusters.len()` but is carried explicitly so downstream consumers don't re-derive it. Only fragment-clone clusters contribute lines to `duplicated_loc` — [RANK-LITERAL-FAMILY] clusters are excluded from the line projection, not from this count.
- `duplicated_files: usize` — count of files containing at least one non-hidden clone occurrence. Upper-bounded by `files_analysed`.
- `per_file: Vec<FileMetric>` — per-file breakdown, one `FileMetric { path, analysed_loc, duplicated_loc, duplication_percent }` per analysed file (clean files included with `duplicated_loc == 0` so percentage denominators stay exact). Same per-file line-set computation as the repo aggregate, scoped to one file; `duplication_percent` uses that file's own `analysed_loc` as the denominator. Sorted by `duplication_percent` desc, path tiebreaker. **`path` is rendered relative to the scan root**, the same form `ReportOccurrence.path` carries, so a consumer that opens a `FileMetric` must resolve it against the workspace exactly as it resolves an occurrence — treating it as absolute names a file that does not exist. **Folders are not carried on the wire** — per-folder rollups are derived by consumers (the VSIX [VSIX-METRICS-PANEL], the HTML report) by summing the `analysed_loc` and `duplicated_loc` of every file under a path prefix, which keeps both numerator and denominator exact. Powers the per-folder/per-file breakdown in [VSIX-METRICS-PANEL].

#### [METRICS-REPO-WEIGHTED] Evidence-weighted duplication percentage

> **Status: specified, not shipped.** Lands with gh #344 per [weighted-metrics-plan.md](../plans/weighted-metrics-plan.md). Until it ships, `Report.metrics` carries only the mechanical fields above, and [EXIT-CODES-WEIGHTED] is unreachable.

The mechanical numerator treats every visible line identically, so a repo full of shape-only boilerplate breaches a `--fail-over` gate exactly like a repo full of verbatim copy-paste (gh #344; gh #355 is a measured instance). Detection evidence is not uniform across clone classes — benchmark precision degrades as syntactic similarity falls ([Bellon et al. 2007](reading-list.md#read-list-metrics), [Svajlenko & Roy 2015](reading-list.md#read-list-metrics)), and case-studied shape-level cloning is frequently deliberate, benign boilerplate ([Kapser & Godfrey 2008](reading-list.md#read-list-metrics)). The weighted metric prices that evidence in; the mechanical metric stays the industry-comparable, exactly-reproducible default (unweighted duplicated-line density is the established CI gate — SonarQube's `duplicated_lines_density`).

**Mechanism.** Same visible cluster set, same non-hidden occurrence projection, same literal-family exclusion, same per-file line sets as the mechanical metric — nothing about cluster selection changes. Each covered line then takes the weight of the strongest evidence covering it:

- `line_weight = max` over covering occurrences of `bucket_weight(cluster.bucket) × category_weight(cluster.category)`. **Max, never sum**: overlapping clusters cannot push a line past `1.0`, and provably-duplicated lines are not diluted by a coincident weak cluster.
- `weighted_duplicated_loc: f64 = Σ line_weight` per file, summed as the mechanical union is.
- `weighted_duplication_percent = clamp(100 × weighted_duplicated_loc / analysed_loc, 0, 100)` — same denominator, same clamp, same zero-corpus rule.

**Weights follow measured evidence class, not academic type number.** Deslop already routes weak evidence into its own buckets ([CLONE-BUCKETS-ROUTING]), so the discount attaches to the bucket, and to the category axis exactly as [RANK-CATEGORY] does. Defaults:

| Key | Default | Rationale |
|---|---|---|
| `bucket_weights.identical` | `1.0` | Byte-equivalence proof ([CLONE-BUCKETS-IDENTICAL]). |
| `bucket_weights.nearly_identical` | `1.0` | Token/anchor-proven Type-3 — the routing thresholds already demand strong content evidence. |
| `bucket_weights.same_behavior` | `0.5` | Semantic evidence only, no syntactic proof; the WT3/T4 band is where benchmark agreement collapses (Svajlenko & Roy 2015). Visible in the gate, but an embedding model's judgment alone cannot fail CI. |
| `bucket_weights.structural_only` | `0.15` | Shape is the only positive signal; equals [RANK-STRUCTURAL-ONLY]'s demote multiplier so ranking and metric tell one story. |
| `bucket_weights.loosely_similar` | `0.0` | "Hint, not a directive" ([CLONE-BUCKETS]); a hint must not move a CI verdict. |
| `category_weights.logic` | `1.0` | Ordinary duplicated logic. |
| `category_weights.data` | `0.15` | Equals [RANK-CATEGORY]'s `data_clone_weight` default (gh #336). |

Weights are configured under `[metrics]` in `.deslop.toml` ([EXCLUSION-CONFIG]); each value must be finite and in `[0.0, 1.0]`, rejected otherwise with a `ConfigThreshold`-style error naming the config path (exit `2`). `0.0` is legal — it excludes the class from the weighted numerator only. Weights are per-bucket declared constants, **not** the fused confidence: fused is still being hardened (gh #343 lineage) and a percentage must be recomputable from the report alone by anyone holding the weight table.

**Wire.** `RepoMetrics` gains `weighted: WeightedMetrics { duplicated_loc: f64, duplication_percent: f64, threshold: ThresholdSummary, bucket_weights, category_weights }` — the resolved weight table is echoed on the wire so every consumer can recompute the number from the report alone. `FileMetric` gains `weighted_duplicated_loc` / `weighted_duplication_percent`; folder rollups sum the weighted numerators exactly as the unweighted ones. Modelled in [live-ipc.td](../models/live-ipc.td), regenerated, never hand-written.

**Invariants** (each is a test assertion): with all weights `≤ 1.0`, `weighted_duplication_percent ≤ duplication_percent`; with all weights `= 1.0` the two are equal to full `f64` precision; the mechanical fields are byte-identical with and without a `[metrics]` section — **no knob may ever change `duplication_percent`**.

Deliberate non-metrics:

- No weight-sum percentage. The ranking `weight` is a log-scaled quantity, not a fraction, and it never enters any percentage — the evidence weights above are declared constants echoed on the wire, a different thing entirely.
- No fused-scaled percentage. A continuous confidence multiplier makes the number a function of the fusion internals and irreproducible from the report; revisit only if the bucket constants prove insufficient.
- No single blended number. Replacing the mechanical percentage would break comparability with every other line-density tool and every existing ratcheted threshold; the two metrics ship side by side.
- No byte-level percentage. Developers reason in lines; a 3-line and a 30-line occurrence are not interchangeable even if their byte counts are similar.
- No "clone density per KLOC". Derivable from `duplicated_loc / analysed_loc * 1000`; we don't ship two spellings of the same ratio.

The text renderer prints a one-line header: `repo: 12.4% duplicated (1 843 / 14 876 LOC, 27 clusters across 11 files)`. Once [METRICS-REPO-WEIGHTED] ships, the header carries the companion figure in the same line — `repo: 12.4% duplicated, 8.1% evidence-weighted (…)` — and never one without the other. HTML surfaces the same line in the report header and colours it by the fail-over threshold (green < threshold, red ≥ threshold, neutral when no threshold is set); when both gates are set, the breached one names itself. JSON is canonical; both renderers read from `metrics`.

### [EXIT-CODES] CLI exit codes and fail-over threshold

Deslop's default exit code is `0` on a successful analysis regardless of how much duplication exists — the tool is diagnostic, not opinionated. Opt-in CI gating is expressed through a single flag and a single config key.

Exit codes:

- `0` — analysis succeeded; no enabled gate breached.
- `1` — unexpected runtime error (parse failure, I/O error, cache corruption that couldn't be recovered). Pre-existing behaviour; unchanged by this spec.
- `2` — invalid CLI invocation (bad flag, incompatible combination, missing required argument). Pre-existing behaviour; unchanged.
- `3` — **duplication threshold breached.** `metrics.duplication_percent > threshold` — or, once [METRICS-REPO-WEIGHTED] ships, `weighted_duplication_percent > weighted threshold` ([EXIT-CODES-WEIGHTED]) — after a successful analysis. The report is still written to disk in full so CI can surface the offenders.

Threshold sources, highest precedence first:

1. `--fail-over <percent>` CLI flag. Accepts a finite float in `[0.0, 100.0]`. `--fail-over 0` means "fail on any duplication". Invalid values → exit `2` with a named error.
2. `[threshold] max_duplication_percent` in `.deslop.toml` (or the file passed via `--config`). Same validation rules.
3. Absent — no threshold is enforced; exit `3` is unreachable and the text/HTML headers render the metric without a pass/fail verdict.

A `--no-fail-over` flag (mutually exclusive with `--fail-over`) overrides a config-file threshold and restores the "report only" behaviour, so a developer can run the CLI locally against a repo whose CI gate they don't want to trip.

#### [EXIT-CODES-WEIGHTED] Evidence-weighted gate

> Lands with [METRICS-REPO-WEIGHTED] (gh #344); unreachable until then.

A second, independent gate over `weighted_duplication_percent`, mirroring the mechanical gate exactly: `--fail-over-weighted <percent>` (highest precedence), then `[threshold] max_weighted_duplication_percent`, then absent → not enforced. Same validation, same full-precision strictly-greater comparison, equality passes. The two gates compose: either breach → exit `3`; the single `--no-fail-over` flag disables **both** — "report only" means report only. The mechanical gate remains the documented default in CI recipes; the weighted gate is opt-in for teams that want boilerplate-shaped findings priced below proven copy-paste, and each gate's verdict is carried separately on the wire (`metrics.threshold` / `metrics.weighted.threshold`) so a breach always names the ceiling it crossed.

The renderer always states the active threshold in the report header (`threshold: 10.00% (breached)` / `threshold: 10.00% (ok)` / `threshold: none`) so the report is self-explanatory when read out of context. The threshold value and breach flag are carried on `Report.metrics.threshold { percent: f64, breached: bool, source: "cli" | "config" | "none" }` so downstream tools do not re-derive the verdict.
