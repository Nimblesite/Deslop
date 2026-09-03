# Pipeline stages (v1, hybrid by default)

### [PIPELINE-LANG-TRAIT] Language plugin trait
The single extension point. Implementations live in `deslop-core::lang::<name>`. Each implementation provides: (a) tree-sitter grammar factory, (b) file-extension filter, (c) per-language node-kind normalization rules that collapse identifier / literal / trivia nodes into their structural kind. The trait output type (`NormalizedNode`) is identical across languages so downstream stages are language-agnostic. v1 ships with `csharp` (`tree-sitter-c-sharp`), `rust` (`tree-sitter-rust`), `python` (`tree-sitter-python`), `dart` (`tree-sitter-dart`), `javascript` (`tree-sitter-javascript`), `typescript`, `tsx` (`tree-sitter-typescript`), `php` (`tree-sitter-php`), `fsharp` (`tree-sitter-fsharp`, source grammar `LANGUAGE_FSHARP` for `.fs`/`.fsx`), and `go` (`tree-sitter-go`). Adding a language = one `LanguageParser` impl + pinning the grammar version in `Cargo.toml`. Shared walking / interning plumbing lives in `lang::shared`; JavaScript, TypeScript, and TSX additionally share `lang::ecmascript` so their normalisation surface stays aligned.

### [PIPELINE-DISCOVER-FILES] File discovery
Walk the target path with the `ignore` crate, respecting `.gitignore` and Git's standard ignore rules. Filter by the set of file extensions contributed by registered `LanguageParser`s. Additionally drop paths matching built-in or configured `[EXCLUSION-CONFIG]` `exclude` patterns — those files are never parsed. Every surviving path is registered with [STATE-FILE-REGISTRY] and downstream code traffics in `FileId`, never `Path`.

### [PIPELINE-NORMALIZE-AST] AST normalization
For each file, parse with the selected language's tree-sitter grammar and walk the resulting tree bottom-up, producing `NormalizedNode { kind: &'static str, children: Vec<Self>, byte_range, file_id }`. Identifier / literal / comment / whitespace nodes are collapsed to their structural kind so Type-2 clones (renamed identifiers) hash identically. Byte ranges are preserved and are the source of truth for any later rendering — line numbers are derived.

The synthetic `__file__` root spans the nodes normalization kept, not the tree-sitter parse root. The parse root covers leading and trailing trivia — a licence header, a comment block — that normalization has already dropped, so inheriting it reports bytes contributing zero nodes to the match: a whole-file occurrence opens on comments instead of the code it duplicates, and its start offset stops tracking edits that move that code. Real nodes keep their own span, because a declaration's braces belong to the duplication even when a comment sits between them. A file that normalizes to nothing keeps the parse root's span. Pinned by `crates/deslop-mcp/tests/issue_153_rescan_freshness.rs` ([LIVE-RESCAN-FRESHNESS]).

### [PIPELINE-NORMALIZE-AST-OPERATOR] Operators survive normalization

Every grammar here spells the operator of a binary, unary or compound-assignment expression as an **anonymous** token, and the walk above reads named children only. `alpha + beta` and `alpha - beta` therefore normalized to the same subtree with the same identifier frontier and the same literals: no stage downstream held any evidence that they differ. The pair measured `structural = 1.00`, `token_jaccard = 1.00`, `agreement = 1.00` — the engine's strongest evidence, made about code that computes a different answer. Sign errors and inverted comparisons are exactly the class of defect that survives review.

Behaviour-bearing anonymous tokens are therefore kept as leaves **carrying their own token**: the normalized kind is `__op__+`, never a shared `__op__`. Operators must not collapse the way identifiers collapse to `__ident__` and literals to `__literal__`, because collapsing them breaks the premise the digest rests on. Identifiers and literals collapse because a rename and a constant edit preserve behaviour, so equal hashes mean *the same code up to renames* and unequal hashes mean the code itself differs — the premise [PIPELINE-CLUSTER-CLOSURE] applies to pair admission. An operator swap is neither a rename nor a literal edit, so a shared placeholder makes `alpha + beta` and `alpha - beta` hash identically and the fingerprint asserts a sameness that does not exist. Every stage reading the digest inherits it: `structural` saturates at 1.00, `token_jaccard` echoes it, the LSH bands collide, and [FUSED-CONTENT-GATE] is left pricing four disagreeing frontier positions out of twenty as a ten-percent discount, still routing the pair as near-identical. Discrimination belongs in the fingerprint, not in a downstream gate correcting for it.

Type-2 recall is untouched. A consistently renamed clone changes identifiers and literals, which still collapse; it does not change its operators, so the operator leaves stay equal and the subtree still hashes identically.

The prefix keeps the operator namespace disjoint from every grammar kind, so an operator can never be confused with a named production that happens to spell itself `in`, `is` or `not`. The leaf is also a **position on the content frontier** whose raw bytes are the operator, which is what lets the content stage report *which* positions disagreed rather than only that the shapes did.

The list is an allowlist over token text, one list for every grammar because tree-sitter names an anonymous node by its own token: arithmetic, comparison, boolean (symbolic and worded), membership and identity, bitwise and shifts, compound assignment, null handling, and ranges. Framing stays out — brackets, commas, semicolons, colons, dots, arrows and the plain `=` of an assignment are already implied by the parent production, and keeping them would inflate every subtree with positions no two members can disagree on.

Operators are a **third** content population, belonging to neither the identifier nor the literal one. There is no substitution that turns `+` into `-`, so counting an operator as an identifier would report a broken rename, and counting it as a literal would report a data table; `literal_fraction` is measured over the identifier/literal vocabulary alone so a data table stays exactly as literal-dominated as it was. An operator that changed is `substance_varies` on its own evidence.

The same predicate is what `cluster_filters::body_shape` compares bodies by, so the signature-only and polymorphic suppressions no longer answer "these bodies are the same" about implementations that compute different answers. Pinned by `crates/deslop/tests/operator_drift_is_not_duplication.rs`, which asserts in the same run that a byte-identical control still buckets `identical` and still saturates — a normalization change wide enough to separate the operator families must not have separated everything.

### [PIPELINE-BOILERPLATE-FILTER] Boilerplate-only clone filtering
Language front-ends classify syntax-only scaffolding before fingerprinting. Import declarations, C# `using` directives, namespace/package headers, Python decorators such as FastAPI route declarations, and equivalent module prologues are treated as **boilerplate carriers**, not business logic. A subtree or sibling window made only from these carriers is excluded from structural fingerprints, sibling-extension windows, token LSH input, and embedding input by default.

Rationale: clone detection literature and mature tools normalize or filter irrelevant syntactic features before comparison. Repeated import blocks produce high-copy false positives that drown out actionable duplication. They are still useful style signals, so the renderer may emit a low-noise action hint rather than a clone warning.

C# special case: if the same non-static `using` directives appear across many files in the same project, the human-facing hint is `Consider moving repeated usings to a global using file` and links to the affected namespaces. This is not a clone cluster and contributes neither mass nor `duplicated_loc`. The JSON/AI report may carry the suppressed byte ranges as `boilerplate_hints` so an agent can propose a safe `GlobalUsings.cs` or project-file `<Using Include="..." />` change.

Configuration:

- Default: boilerplate-only clones are suppressed and no hint is emitted (`boilerplate.imports = "suppress"`).
- Opt-in diagnostic mode: `.deslop.toml` can set `boilerplate.imports = "report"` under `[defaults]` or `[language.<id>]` to include them as low-severity hints for teams that explicitly want import hygiene audits. This mode does not restore import-only clone warnings; it emits structured `boilerplate_hints` instead.
- No mode may rank import/using-only ranges above executable or declarative code clones.

### [PIPELINE-FINGERPRINT-MERKLE] Structural fingerprint (Merkle)
Bottom-up Merkle hash over `NormalizedNode`. Each node's hash combines its own `kind` string with the ordered hashes of its children using `blake3`. Each node stores `(hash, subtree_node_count, byte_range, file_id)`. Nodes whose subtree size is below `--min-nodes` are excluded from clustering per [DECISION-MIN-NODES].

### [PIPELINE-SIGNATURE-MEMO] MinHash construction is memoised by token-stream digest

`MinHash` over the k-grams is a pure function of the token stream alone, so the memo — keyed by a length-prefixed blake3 digest of the stream — is exact by construction: a repeated stream gets the byte-identical signature of its first construction, and one corpus build pays for each distinct stream once. The memo spans a whole batch corpus build (cross-file, where the repetition lives) and one file on an incremental change pass. Fallback signatures never enter it: they are deliberately scoped to the fingerprint's byte range (#86) so unrelated empty streams cannot cluster through shared emptiness. Hit/miss counts are surfaced on the `fingerprint corpus built` record ([PIPELINE-OBSERVABILITY-STAGES]); the miss count is the distinct-stream population; retention is capped at `SIGNATURE_MEMO_MAX_ENTRIES`, so the memo's residency stays a bounded share of the memory budget on any corpus, post-cap streams being constructed fresh with identical output. Pinned by `a_repeated_token_stream_costs_one_minhash_construction` and `too_short_streams_never_touch_the_memo`.

### [PIPELINE-CLUSTER-EXACT] Exact subtree clustering
Group `NormalizedNode` fingerprints by `hash` to propose exact candidate pairs. This covers Type-1 and normalized Type-2 deterministically in O(n). A hash bucket is not a cluster: each concrete pair must still pass [FUSED-STRATEGY-BOUNDED-MAX]. Candidate pairs are language-scoped by default per [CONFIG-CROSS-LANGUAGE]; the exact same hash may still be compared across languages when `.deslop.toml` opts into cross-language comparison.

### [PIPELINE-CLUSTER-EXACT-SCOPE] Inside one declaration the widest view is the finding

One duplication is fingerprinted at many depths, so several candidate views can cover the same region. Before pair admission, a same-file overlap run selects its physical view by authored scope and width only. When one view encloses another inside the same authored declaration, the enclosing view is proposed; otherwise the wider view is proposed, with stable byte-range ordering as the tie-breaker — except where two views straddle each other and one of them is a declaration ([PIPELINE-CLUSTER-EXACT-SCOPE-STRADDLE]). Pair scores cannot choose a view because no pair has been admitted yet.

Declarations are read from the normalised tree and keyed by language. A view that is the declaration is treated as the enclosing authored scope. Pinned by the TypeScript, JavaScript, and F# scope fixtures, which assert proposed ranges and eventual admitted pairs without attaching a grade to a cluster.

#### [PIPELINE-CLUSTER-EXACT-SCOPE-STRADDLE] A view that is the declaration beats a view that cuts through it

Two views can overlap without either containing the other: one starts at a `namespace` line and ends in the middle of a method, the other is that method, modifier through closing brace. The first welds a cut-off body to whatever sits beside it — a namespace line, a class shell, a sibling member — and no author wrote that region. When two views straddle each other like this, the view whose range **is** a function-like declaration is the finding, whatever its width. Views where one contains the other are untouched by this rule: a whole file that holds a method whole is still the wider authored scope and still wins. Pinned by `issue_389_subsumption_modifier_straddle` (C#: the namespace-to-mid-method window loses to the authored `ReconcileEntries` method) and the `incremental-multilang` expectation table.

#### [PIPELINE-CLUSTER-EXACT-SCOPE-SCRAPS] A view that is the function beats a window that merely wraps it

A same-file window can enclose an authored function whole while adding only scraps around it — the two field declarations above a Dart accessor, a constructor line. Such a window is a view Deslop cut over a run of siblings, not something the author wrote, and it shares almost nothing beyond the function it wraps. When a window that is not itself a node of the tree encloses an authored function-like declaration and holds fewer than `admission.shared_subtree_min_node_count` nodes beyond it, the function is the finding, whatever the window's width. A node the author wrote — a class body, a module, a whole file holding the function — keeps the width rule of [PIPELINE-CLUSTER-EXACT-SCOPE]. Without this, one method of a seven-member family published as "fields plus method" while its six siblings published as methods. Pinned by `rename_needs_an_anchor` (Dart: every accessor publishes at its own extent).

### [PIPELINE-CLUSTER-CLOSURE] Clusters are exactly the transitive closure of admitted pairs

Candidate generation may propose a pair through exact fingerprints, token LSH, or embedding neighbours, but proposal is not admission. [FUSED-STRATEGY-BOUNDED-MAX] evaluates structural similarity, token Jaccard, embedding similarity, and content similarity for that exact pair. Only a pair that passes the complete admission contract becomes an edge.

Clusters are formed as the connected components of the admitted-pair graph. No component-level average, edge selection, family score, label, or post-admission similarity judgement may add, remove, or redirect an admission edge. A bridge that passes admission legitimately connects its endpoints; a bridge that should not connect them must fail the pair admission rule. Tests pin the admitted edges and their closure.

The post-closure noise stage is separate. Under [CLONE-NOISE-VERBATIM-SUBGROUP], a component that a noise filter convicts is replaced by its qualifying byte-identical families and members outside those families are dropped; a component no filter convicts is handed on untouched. This exhaustive suppression exception does not revise admission, manufacture pair evidence, or classify the component. Each rendered survivor receives mass from its own canonical extent and visible membership under [RANK-MASS-SUM].

Pair evidence remains attached to the admitted edge. The resulting component owns identity, canonical extent, occurrence membership, mass, and mass-derived rank only.

### [PIPELINE-CLUSTER-CANDIDATE-CONTAINER] Container candidates obey the same pair admission rule

A class, sliding window, or file root is not removed from a component by a component-level container heuristic. It is an ordinary candidate endpoint and survives only through admitted pairs. If a container is inaccurate, the pair assertion and admission implementation are wrong. The only post-closure partition is the convicted-noise exception in [CLONE-NOISE-VERBATIM-SUBGROUP].

### [PIPELINE-CLUSTER-SUBSUME] Cross-cluster subsumption
One physical duplication can be fingerprinted at several AST depths and therefore appear as several closure components covering the same bytes. Subsumption removes duplicate views; it never changes the membership of a component and never recomputes admission.

Two components describe the same physical duplication only when coverage is bidirectional by per-occurrence containment: every occurrence of each component contains, or is contained by, an occurrence of the other in the same file. Bare intersection and one-way coverage are insufficient. A view naming a file the other does not name is never dropped.

When two components describe the same duplication, the survivor is selected by file coverage, physical enclosure, occurrence coverage, duplicated mass, and stable cluster-id tie-breaking in that order. Structural, Jaccard, embedding, content, rename, literal, pair classification, and any presentation label are forbidden inputs. A view is published when no published view describes the same duplication and outranks it; every other view is absorbed by one that does. That is a property of the published set, not of the order views were met in, so whatever a view absorbed is judged again against the views that remain when it leaves the report ([PIPELINE-CLUSTER-SUBSUME-KERNEL]).

Pinned by `crates/deslop-core/tests/cluster_subsumption/region.rs` and end-to-end overlap fixtures. Assertions must prove the survivor's files, occurrences, ranges, and mass; pair scores cannot be asserted on a cluster.

#### [PIPELINE-CLUSTER-SUBSUME-STRADDLE] Two views that straddle a nested view are padded readings of it

Two admitted windows can overlap without either containing the other: a byte-identical block with one differing statement kept on its left in one view and one differing statement kept on its right in the other. Each window clears the content floor on the strength of the block it shares, and their union does not, so neither can absorb the other and both would reach the report — the same block published twice, under two extents that each count a line the other refuses.

When two components name the same files, every occurrence of each overlaps an occurrence of the other in its file, and a third component lies strictly inside both in every file, the two straddling views are dropped and the nested view is the finding. Straddles are looked for among the published views; the two are removed for good and their file set is resolved again without them, so whatever either had absorbed — through any verdict — is judged again and the nested view collects its own nested rivals. Two overlapping regions with no admitted view nested in both stay two findings, exactly as before: a shared byte, a half overlap, or an overhang is never enough on its own.

Implemented in `cluster/subsume/kernel.rs`; pinned by `two_windows_straddling_one_nested_view_publish_that_view`, `a_view_that_yielded_to_a_straddler_is_released_when_it_dies` and `a_view_nested_in_only_one_straddler_leaves_both_published` in `crates/deslop-core/tests/cluster_subsumption/straddle.rs`, and by `cross_cluster_collapse::padded_windows_straddling_a_verbatim_block_publish_the_block` end to end.

#### [PIPELINE-CLUSTER-SUBSUME-KERNEL] Survivors are a property of the published set, not of scan order

The views over one file set, with the survivor order between every pair that describes the same duplication, form a directed graph: an edge runs from the preferred view to the view it re-describes. The published set is that graph's kernel — no published view is outranked by another published view, and every unpublished view is outranked by a published one. It is found by publishing, in rank order, whichever undecided view no undecided or published view outranks, and absorbing every undecided view of its region as it goes. A view whose absorber is later removed, whether outranked or dropped as a straddler, is therefore judged again against the views that remain, and nothing a removed view absorbed is forgotten.

Implemented in `cluster/subsume/kernel.rs`; pinned by `a_view_released_by_its_absorber_is_judged_against_the_views_that_remain` in `crates/deslop-core/tests/cluster_subsumption/release.rs`, and by the report contract every subsumption test holds its result to: survivors in rank order with unique ids and the mass [RANK-MASS-SUM] gives them, no two survivors describing one duplication, and every unpublished view re-described by a survivor over its own files or straddling a nested view a survivor reports.

#### [PIPELINE-CLUSTER-SUBSUME-CYCLE] A cycle in the survivor order is decided by coverage, mass and id

Three views can each outrank the next: enclosure decides one pair, and the coverage-mass-id order decides the other two crossed pairs the opposite way. Then no view of the cycle is free of an undecided rival and no kernel exists. The view that leads on occurrence coverage, duplicated mass and stable id is published, and every other view of its region is absorbed by it — the tie-break the survivor order already ends in, applied once more. The region is still reported exactly once.

Pinned by `three_views_that_outrank_each_other_in_a_cycle_publish_the_leader` in `crates/deslop-core/tests/cluster_subsumption/release.rs`.

#### [PIPELINE-CLUSTER-SUBSUME-FILESET] Views are judged only against views over exactly their own files

Every verdict above needs both views to name the same set of files: same-region coverage pairs each occurrence with a same-file partner in both directions, and a straddle demands file coverage both ways before it looks for a core, whose own occurrences must lie inside both straddlers in every file. Two views over different file sets therefore always keep each other, and a view is only ever absorbed, released or chosen as a core within its own file set.

Each file set is resolved on its own, in rank order. On the Flutter corpus that is the difference between 217,045 views squared — a stage that held one core for half an hour without a record — and a sum of small squares. No verdict changes.

The stage reports its counts — views, file sets, pairs evaluated, same-region pairs, absorptions, cycles, straddle rounds and straddlers — in a completion record and a fixed-interval progress record ([PIPELINE-OBSERVABILITY-STAGES]).

Pinned by `each_file_set_is_judged_on_its_own` and `disjoint_file_sets_are_never_compared` in `crates/deslop-core/tests/cluster_subsumption/release.rs`.

### [PIPELINE-DETERMINISM] Cross-run determinism
Two runs of the pipeline over an unchanged corpus produce bit-identical deterministic output: identical MinHash signatures (blake3 XOF, fixed k-gram ordering), identical pair evidence (`token_jaccard` compared bit-for-bit), identical admitted pairs, closure components, cluster ids, mass, and ranking. Determinism is what makes persisted processing ([PIPELINE-INCREMENTAL]) sound and cluster ids stable across sessions. The embedding/ANN layer is the only approximate stage and is bounded separately ([FUSED-EMBED-PROVIDER]); a missed ANN neighbour only loses recall, never changes an already admitted pair.

Determinism holds over corpus *state*, not edit history: identical paths and bytes produce an identical report whatever sequence of edits got there. Every pipeline ordering is therefore keyed by workspace-relative path (with the registration id only as a tie-breaker), never by `FileId` alone — ids are append-only, so removing and restoring a byte-identical file would otherwise reorder the corpus, move the LSH star centre, and change rendered ranges and metrics for identical source. Rendered occurrence order follows the same path-ordered corpus. Pinned by the LSP `history_determinism` suite, which cycles a config exclusion over live files and asserts the restored report is field-for-field identical.

### [PIPELINE-OBSERVABILITY-STAGES] Long stages emit bounded aggregate records, never per-item events

Every corpus-scale stage reports what it did as aggregate gate counters in a completion record at `info`, plus fixed-interval progress records for stages long enough to be mistaken for a hang — so a default-level run of a large corpus is distinguishable from a stuck one, and the record volume is bounded by how long a stage runs, never by how much work it does. Per-item events in corpus-scale hot paths are `trace`: one `debug!` per measured pair produced 793,076 records on the corpus that motivated this, burying the stage events and measurably slowing the stage being diagnosed. Records carry counts, elapsed milliseconds, and substage attribution (`read_ms`/`parse_ms`/`fingerprint_ms`/`signature_ms` on `fingerprint corpus built`; gate counters on the shared-subtree rescue tally; `stage`/`clusters`/`elapsed_ms` on `cluster stage complete`) — never file contents or user paths. A completion record is emitted even when a stage found nothing: an absent event and an empty population are otherwise indistinguishable in a log.

### [PIPELINE-INCREMENTAL] Persisted processing — the parse store
Deslop persists processing to disk and re-derives only what changed. Each stored artefact is a computation result addressed by exactly the content that determines it, and [PIPELINE-DETERMINISM] makes the stored result bit-identical to recomputing it — a content-addressed store with correctness invariants, not a discardable accelerator hint. "Cache" survives in the surface names (`.deslop/cache/`, `cache_stats`, `FingerprintCache`) and in hit/miss vocabulary; the semantics are the ones this section states.

The parse store holds one blob per `(language_id, tool_version, min_nodes, source_byte_hash)`. The hash is `blake3` over the file's **raw bytes** — never over a decoded string. A lossy decode collapses every maximal invalid UTF-8 subsequence to one U+FFFD, making the key non-injective: byte-distinct files share one entry and the second is served the first's tree and fingerprints (gh #382, pinned by `crates/deslop/tests/cache_key_lossy_utf8_collision.rs`). A hit rehydrates the structural fingerprints, the normalised AST, and the per-fingerprint MinHash signatures ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]) from a compact little-endian binary blob, so unchanged files skip tree-sitter *and* signature construction entirely; a miss parses the file, builds fingerprints and signatures, and persists the bundle. Before a hit is served, the fingerprints are re-derived from the rehydrated tree and compared with the stored records: any disagreement voids the blob — the stored signatures are positionally bound to the stored fingerprint list, so they are unattributable the moment that list cannot be reproduced — and the file takes the miss path, whose store self-heals the blob. Any mismatch on the key — tool upgrade, grammar pin, `--min-nodes` change, source edit — degrades gracefully to a miss; stale blobs never leak into a run.

**Activation.** On by default on every surface. Incremental analysis is a first-class path, not a bolt-on: the LSP runs on it permanently, and a batch CLI run is just "incremental starting from an empty store". `deslop --no-incremental` opts out per invocation for callers that must not write to the tree at all; `[analysis] incremental = false` in `.deslop.toml` ([CONFIG-INCREMENTAL-OPTOUT]) opts the whole workspace out on every surface without a flag. Stats land on every report as `cache_stats { hits, misses }` at top level. Text renderer surfaces them as `cache: N hit / M miss`.

**[PIPELINE-INCREMENTAL-INVALIDATION] Invalidation is addressing, not bookkeeping.** The store cannot serve a stale parse, because a stale parse is *unaddressable*: the blob's filename is `blake3(file contents)`, under path segments for language, tool version, and `min_nodes`. Edit a file with nothing watching — an agent writing, a `git checkout`, an editor with the LSP stopped — and its content hash changes, so the lookup lands on a path that does not exist and the file is re-parsed from disk. There is no mtime heuristic, no watcher-maintained index, and no invalidation step that could be skipped or get out of sync.

Two further properties make persistence-on-by-default safe for the CLI:

- **Corpus membership never comes from the store.** Every run performs a fresh discovery walk, so files added or deleted while nothing was watching are picked up regardless of store state. A deleted file's blob is orphaned — never consulted, kept as revert reuse until [PIPELINE-INCREMENTAL-RETENTION] prunes it.
- **A warm run and a cold run agree.** Wiping `.deslop/cache/` changes the `cache_stats` counters and nothing else about the report. The store never defines results — the source tree is the only source of truth; persistence only decides how much of it must be re-derived.

The one artefact that *can* go stale is the live state file `live-report.json` ([LIVE-STATE-FILE]) — a whole-report snapshot, not a content-addressed entry. The CLI never reads it. The LSP seeds from it for instant warm-start and immediately runs a cold pass that replaces it, reporting `Running` until that pass installs ([LIVE-CACHE-SEED]).

**Layout.** `<root>/.deslop/cache/fingerprints/<language_id>/<tool_version>/<min_nodes>/<source_byte_hash>.bin`. Shares `.deslop/cache/` with the embedding cache from [FUSED-EMBED-PROVIDER]; the two layers invalidate independently.

**Format.** `u32` magic, then a 32-byte **binding digest** ([PIPELINE-INCREMENTAL-INTEGRITY]), then the payload: a recursive `NormalizedNode` tree (`u32 kind_len`, kind UTF-8 bytes, `u64 start`, `u64 end`, `u32 child_count`, children...), then `u64 fingerprint_count` followed by one `{ [u8;32] hash, u64 start, u64 end, u64 node_count }` record per fingerprint, then `u64 signature_count` followed by one 128×`u64` MinHash signature per fingerprint, positionally 1:1 with the fingerprint records. Decode rejects any blob whose signature count disagrees with its fingerprint count, and any blob whose payload does not consume the file exactly. No serde, no schema drift: the magic + tool-version path segment bracket every format change — the pre-signature and pre-digest layouts' magics decode as a plain miss and their blobs are rewritten in the current format.

### [PIPELINE-INCREMENTAL-INTEGRITY] A blob is bound to its address

The filename alone proves nothing about the bytes inside it: a corrupted payload keeps its filename, and a blob moved, swapped, or copied to another valid address decodes cleanly there. Both were reproduced serving wrong reports — corrupted MinHash payloads flipped `token_jaccard`, and a two-blob swap exchanged two files' rendered spans and buckets — so blob trust is part of the accuracy surface, not an optimisation detail.

Every blob therefore carries a BLAKE3 **binding digest** over its payload and the full address that wrote it: language id, `min_nodes`, source-byte hash, the layout revision (the magic), the signature width, and a **semantic epoch** — a constant bumped when parsing, normalisation, fingerprinting, or signature construction changes meaning without changing layout, deliberately independent of the reused `0.0.0-dev` package version in the directory path. A lookup recomputes the digest from its *own* address before decoding anything. Corruption anywhere in the file, a blob under the wrong source hash, a blob copied across a language or `min_nodes` partition, trailing bytes, and a stale epoch all fail identically: a plain miss that re-parses from source and self-heals the blob, with the next pass hitting cleanly.

Corrupt bytes may never crash the run either, and the digest is verified **before** a single payload byte is decoded — so ordinary corruption never reaches an allocation path at all. The bounds behind it hold even for a payload whose digest checks out:

- **The read is bounded on the read, not on a prior measurement.** One handle does both jobs: the length comes off the opened file and the read is taken one byte *past* `MAX_BLOB_BYTES` (256 MiB), so a file that another binary grows mid-read is observable and refused rather than silently truncated into a valid-looking prefix. The buffer is reserved fallibly — an allocation the machine cannot satisfy is a miss, not an abort.
- **Every decode-side length field is proven against the bytes actually remaining** before it sizes an allocation, so a corrupt count degrades to `InvalidData` rather than a capacity-overflow abort.
- **A global node budget bounds the whole tree** (`MAX_DECODED_NODES`, 4 M). The byte bound alone is not enough: an encoded node costs 24 bytes on disk but a resident one costs several times that, so counts that pass the remaining-bytes test could still multiply into an allocation many times the file size. The budget is claimed per node *including the child slots that follow it*, so an absurd child count is refused before its `Vec` is reserved. `MAX_AST_DEPTH` bounds one path; this bounds a wide-but-shallow tree, which the depth guard cannot see.

After the digest verifies, the served hit is still cross-checked by re-deriving fingerprints from the rehydrated tree ([PIPELINE-INCREMENTAL]), defence in depth against an encoder bug the digest would faithfully sign.

**The semantic epoch is the one lever no equivalence test can pull.** Every other invalidation axis is addressed: change the source, the language, `min_nodes` or the blob layout and the lookup lands somewhere else. Change what a parse *means* — normalisation rules, a grammar pin, fingerprinting, signature construction — without changing the layout, and in a development build (where `tool_version` is the permanently-reused `0.0.0-dev`) every stored blob stays addressable. A warm run then serves the pre-change analysis, and no warm-versus-cold comparison can detect it because both sides are stale together. Release builds are stamped with their own version and partitioned on their own, so this can only ever mislead a development store. What catches a forgotten bump is the goldens that pin the pre-change analysis: the per-language `Sample.expected.ast` dumps ([PIPELINE-NORMALIZE-AST]) for parsing and normalisation, and the two committed report goldens for fingerprinting and signature construction. Each names the constant in its failure message, so the change that must bump it is the change that is told to, and `fpcache::tests::the_blob_format_revisions_are_pinned` makes the bump itself deliberate from the other direction.

Pinned end-to-end by `crates/deslop/tests/cache_blob_integrity.rs` (tampered signature payload, same-partition blob swap, cross-language blob copy, truncation / trailing garbage / zeroed interior — each asserting exact miss accounting, truth-report equality, and clean healing) and at the unit level by `crates/deslop-core/src/fpcache/tests.rs` (wrong-address bindings, superseded magics, count and length bombs with valid digests, oversized files).

**Failure modes.**

- Corrupt, truncated, misaddressed, or oversized blob → treated as a miss, logged at `warn!`, overwritten by the next successful parse.
- Cache directory unavailable (permissions, read-only fs) → `FingerprintCache::open` fails, the pipeline falls back to the full parse path for the affected language, logs `warn!`, keeps running.
- Blob write fails (e.g. disk full) → `warn!`, return the in-memory result, pipeline continues.

Zero-zero stats indicate the pass ran without the store (`--no-incremental` passed, the `[analysis] incremental = false` config opt-out ([CONFIG-INCREMENTAL-OPTOUT]) applied, or discovery yielded nothing). Any non-zero counter proves the store was consulted.

**Scope.** [PIPELINE-INCREMENTAL] governs persistence for the parse stage and for the per-fingerprint MinHash signatures stored beside it ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]) — the dominant cost of the LSH block. Everything further downstream — band collision enumeration, candidate pairing, clustering, ranking, metrics, rendering — recomputes in full on every pass regardless of how many files changed. Making that remaining cost track the size of the change is the rest of [PIPELINE-INCREMENTAL-ANALYSIS].

### [PIPELINE-INCREMENTAL-RETENTION] The store prunes itself after every full pass

A full pass is the one moment the live blob set is exactly known — every admissible file was read, so every blob the corpus can address is enumerable. Retention runs there and only there: never on a single-file change pass, and never when the store is disabled (the opt-out leaves the store untouched, [CONFIG-INCREMENTAL-OPTOUT]).

- **Nothing is deleted while the store is under budget.** Two classes of blob look useless and are not. An **orphan** — a blob in the current partition whose source bytes left the corpus — is exactly the content-addressed reuse set for a revert or a branch switch, and [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] asserts a revert full-hits the store, so eager removal is a recall regression against that contract. A blob under **another tool version** is unaddressable by *this* binary but may belong to a different one still running against the same workspace — an LSP from the installed VSIX beside a freshly-built CLI. Deleting it destroys that binary's store for no space gain, so retention classifies it and leaves it.
- **Over budget, eviction is by class, then age.** The classes in precedence order: **other-version** blobs first (this binary can never address them), then **orphans** (the current corpus does not reference them), then **live**. Within a class, oldest modification time first, path as the deterministic tie-break, stopping the moment the store fits. Class outranks age in both directions: the newest other-version blob is evicted before the oldest live one. Blobs under other `min_nodes` partitions of the current version count as live — a different invocation may still address them — and are age-ranked only. Evicting any blob is correctness-free: the next pass that addresses it misses, rebuilds from source, and self-heals ([PIPELINE-INCREMENTAL-INVALIDATION]).
- **The budget is 2 GiB** over the whole fingerprint store (~11× the pinned tokio benchmark's 185.8 MiB store), so ordinary repositories never see an eviction at all.
- Only `.bin` blobs are retention's to manage; foreign files are never touched. Every step is best-effort — an unremovable entry is skipped, never an error. The sweep logs counts only: `fingerprint store swept { other_version_blobs, orphan_blobs, evicted_blobs, store_bytes }`.

Pinned end-to-end by `crates/deslop/tests/cache_retention.rs` (an edit cycle whose kept orphan lets the revert full-hit, live blobs byte-unchanged across sweeps, disabled-store passes leaving the store untouched) and at the unit level by `crates/deslop-core/src/fpcache/retention/tests.rs` (another tool version's partition classified but never swept under budget, class-before-age eviction across all three classes, orphan-before-live order, oldest-first fallback, budget stop condition, foreign-file safety, other-`min_nodes` liveness).

### [PIPELINE-INCREMENTAL-ANALYSIS] Incremental analysis
⏳ **Partially implemented.** Signature reuse ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]) is implemented and pinned by `crates/deslop/tests/signature_reuse.rs`. The equivalence contract is enforced end-to-end on both reuse paths: across separate processes by `crates/deslop/tests/incremental_equivalence.rs` (the on-disk parse store), inside one long-lived session by `crates/deslop/tests/live_session_equivalence.rs` (the in-memory splice), and across six languages sharing one store by `crates/deslop/tests/incremental_multilang_golden.rs` (committed cold golden, warm reproduction) and `crates/deslop/tests/incremental_multilang_matrix.rs` (per-language touch, delete, revert, edit-chain and parser-partition scenarios). The remaining downstream stages are tracked by gh #383.

An **incremental pass** is one that is given the set of files whose content changed since a previous pass over the same corpus, and is permitted to reuse work from that pass. A **cold pass** reuses nothing.

**[PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] An incremental pass owes the cold report.** For any corpus state reachable by any sequence of edits, the report produced by an incremental pass must equal the report a cold pass produces for that same state — field for field: admitted pair records, cluster ids, occurrence paths and byte ranges, mass, ranking order, `metrics`, and `clusters_hidden`. `cache_stats` is the sole permitted difference. This follows from [PIPELINE-DETERMINISM] holding over corpus *state* rather than edit history: if two paths to the same state can produce different reports, the incremental path is wrong, not the cold one. A performance gain that costs equivalence is not a gain.

**Any sequence of edits, not any sequence of processes.** There are two reuse paths and the contract binds both. A fresh batch invocation rebuilds its in-memory corpus from discovery and reuses only the on-disk parse store. A live session (`PipelineSession`) keeps the flat fingerprint / signature / tree store, the per-file sources, languages, line counts, boilerplate ranges and path map alive in memory and *splices* one file's records per change — a far larger reuse surface, and one no parse-store integrity check can vouch for, because the store was never consulted for the state a splice carried forward. `live_session_equivalence.rs` drives that path through the binary (`--rerun-add` / `--rerun-remove` mutate the tree between `initialise` and `update_files`) and compares the spliced report against a cold pass over a fresh tree in the same state: add, edit, remove, all three in one pass, edit-then-revert, and an add whose path sorts *ahead* of every existing file. The last one is not a variation for its own sake — the store holds one span per file in ascending workspace-relative-path order and a render borrows those slices as they are, so a splice that appends rather than inserting at the file's sort position renders that occurrence, and the `summary` line built from it, out of order while every other reading stays identical.

**[PIPELINE-INCREMENTAL-ANALYSIS-REUSE] What may be reused.** Any value that is a pure function of content that did not change. Concretely: a MinHash signature is determined by one subtree's normalised token k-grams; a pair's structural and token-Jaccard scores are determined by its two subtrees. Neither depends on the rest of the corpus, so neither needs recomputing when the rest of the corpus is untouched. Values that depend on corpus-wide state — cluster mass, repo metrics, and the duplication percentage — are derived from the assembled cluster set and are recomputed every pass.

*Implemented for per-language MinHash signatures.* Each file's signatures are built once at parse/load time and persisted in its parse-store blob, positionally 1:1 with its fingerprints ([PIPELINE-INCREMENTAL] Format); a warm pass attaches them instead of rebuilding, on both the batch and the live splice path. The reuse is observable as the `signatures_built` / `signatures_reused` structured fields on the `fingerprint corpus built` tracing event — a fully-warm pass reports `signatures_built=0` with `signatures_reused` equal to the fingerprint count. Cross-language signatures ([CONFIG-CROSS-LANGUAGE]) are opt-in audit state and stay render-time.

**[PIPELINE-INCREMENTAL-ANALYSIS-ADDRESSING] Reuse is addressed, not bookkept.** Reused artefacts follow [PIPELINE-INCREMENTAL-INVALIDATION]: each is stored under a key derived from the content that determines it, so a stale artefact is *unaddressable* rather than merely unused. No mtime heuristics, no watcher-maintained validity index, no invalidation step that could be skipped or drift. A key derived from anything other than that content — a version string, a path, a lossy transform of the bytes — does not satisfy this and is a defect, not an optimisation. The key's language component is load-bearing in the same way: two byte-identical files routed to different parsers must occupy two entries, or the second is served a tree built under a grammar it was never parsed with. `crates/deslop/tests/incremental_multilang_matrix.rs` pins that partition against a mixed six-language corpus.

**Corpus membership never comes from reuse.** As with the parse cache, every pass performs a fresh discovery walk. Files added or removed while nothing was watching are picked up regardless of what state was carried forward.

**What is left, and what it was measured against (gh #383).** Everything downstream of signatures — band collision enumeration, candidate pairing, clustering, ranking, metrics, rendering — still recomputes corpus-wide on every pass. The attribution that ordered this work, taken on the pinned tokio corpus (release, `--embeddings off`), is what any future phase should re-measure against rather than re-derive:

| stage, warm pass | share |
|---|---|
| parse-store load (decode, digest verify, fingerprint re-derivation) | ~23% |
| LSH band enumeration | ~12–14% (was ~44% before `band_key` identity concatenation) |
| pair classification + metrics + JSON write | ~25% |
| candidate scoring, closure, rank, content | ~5% |

Signature construction — ~69% of the LSH block before this work — is gone from the warm path entirely. Two design decisions are settled and should not be relitigated without new numbers. **Full signatures, not band hashes, are what the blob persists**: `estimate_jaccard` consumes full signatures for every candidate pair that reaches scoring, so persisting only the 32 band hashes per fingerprint (256 B against 1 KB) would force full-signature reconstruction for exactly the pairs that matter. **The recorded alternative for the banding phase** is to persist the band index (band → bucket → fingerprint hash) instead: an incremental pass evicts the changed files' fingerprints from their buckets, inserts the new ones, and reads off only the collisions involving them — O(k·N) rather than O(N²), which is what makes cost track change size. If that lands, the per-fingerprint signatures stop earning their bytes (~85% of the 185.8 MiB tokio store) and the blob can drop back to roughly its pre-signature shape.

### [PIPELINE-DIFF-INGEST] Unified-diff ingestion and verification

> **Status: shipped.** Pinned by `crates/deslop-core/src/diff_scope/` unit tests, `crates/deslop/tests/diff_scoped_reporting.rs`, `crates/deslop/tests/diff_scoped_ingest.rs` (the stale-diff refusal) and `crates/deslop/tests/diff_ingest_refusals.rs`.

`--diff` ([cli.md §CLI-ARG-DIFF](cli.md)) is consumed by a strict line-oriented parser — exact structural prefixes and integer parsing, never pattern matching; an unrecognised construct rejects the whole diff (exit `2`) rather than guessing at spans. Recognised grammar: `diff --git` headers, `---`/`+++` file targets with `a/`/`b/` prefixes and C-quoted paths, rename/copy/similarity and `Binary files` lines, `@@ -l[,n] +l[,n] @@` hunks, and ` `/`+`/`-`/`\` body lines. Outside a copy section, only new-side **added** lines produce spans — context and deletions scope nothing, so a pure rename or a deletion-only hunk tags nothing. Spans are merged and sorted per file. Paths resolve against the working directory, then re-relativise to the scan root — the same form `ReportOccurrence.path` carries.

**A hunk requires a target.** Any `diff ` line opens a file section, so junk followed by a valid-looking hunk would otherwise assemble a *pathless* section — one the verifier has no file to check and therefore skips, silently dropping its added lines from the scope and from `added_loc`, which lets `--fail-over 0` pass a run it should gate. A `@@` header in a section that has not seen a `+++` line is refused (exit `2`) naming that line. `+++ /dev/null` counts as seen: a deletion *has* a target line, it just names no new-side file. A section with no hunks at all needs no target.

**A corpus miss is triaged, never blanket-ignored.** A repo-root diff legitimately touches files the scan never sees, so three misses stay ignorable and are counted on the `diff ingested` tracing event: a path outside the scan root, a path whose extension no registered language parser claims, and a path present on disk that discovery deliberately excluded ([EXCLUSION-CONFIG] or gitignore). A section claiming no new-side line at all — every hunk removes — is likewise nothing to verify. But a **supported, in-root** target the tree does not hold is a *stale diff*, not an out-of-scope file: it is refused (exit `2`) naming path and line, because ignoring it would silently zero the very scope a merge gate reads.

**A git copy adds its whole target.** `copy from` / `copy to` are payload, not inert metadata. The target of a copy did not exist before the change, so every one of its lines is content this change introduced, and git states that in both of its copy shapes: a metadata-only copy (`similarity index 100%`, no hunks) asserts the target byte-equals the source, and a copy *with* hunks describes the target as the source plus a delta. Either way both halves are resolved, the byte-equality claim is verified against the tree, and the target's full `1..=line_count` range is added; hunks, when present, are verified like any other hunk but project nothing of their own, so the full range is never counted twice. The source is untouched by the copy and stays out of the scope. A dangling or duplicated copy half, a source or target the tree does not hold, and a `100%` claim the bytes contradict all refuse with exit `2`.

`\ No newline at end of file` annotates the terminator of the line above it rather than being a body line of its own. It is recognised wherever it appears inside a file section — `git` emits it after the last line of a hunk, by which point the hunk's declared counts are already satisfied — and it consumes no count on either side, because counting it would shift every new-side line number after it and mis-tag the occurrences those numbers address. With no file section above it, it is junk like any other unrecognised line and rejects the diff.

**The diff must describe the scanned tree.** Every context and added line of every hunk must byte-match the scanned file at the claimed new-side line number (line terminator excluded). The first mismatch aborts with exit `2` naming the file and line: a stale diff would tag the wrong occurrences, and under `--only-changed` a mis-tag is a silent false negative in a merge gate.

### [PIPELINE-RANK-WORST-FIRST] Ranking: worst offenders first
Before ranking, each cluster's occurrences are reduced to one member per **transitively overlapping run** per file. Fingerprinting emits one subtree per AST node, so a duplicated region yields a nest of overlapping windows over the same bytes; publishing more than one inflates the occurrence count, the cluster size, and the duplication percentage. Overlap is transitive, so the run's frontier is tracked separately from its representative: for `[0,100]`, `[90,110]`, `[105,200]` the bridging window is the narrowest and loses the width contest, and a sweep that tests the next window against the representative alone reports one region as two. The widest window of each run is the reported location; a cluster left with one location is not a duplicate and is dropped. Pinned by `crates/deslop-core/tests/cluster_overlap_collapse.rs`.

Ranking uses the duplicated-mass formula owned by [RANK-MASS-SUM] below. Clusters sort by mass descending and cluster id ascending. Visible member count excludes [EXCLUSION-CONFIG] `report_hide` occurrences. No similarity evidence, pair classification, finding kind, confidence, policy multiplier, or spanned-LOC term changes mass.

The sort ends by stamping the ranking onto the report: `rank` (one-based, worst first) and `rank_band` ([severity.md §SEVERITY-BAND](severity.md#severity-band)) ride on every rendered cluster, so every consumer displays the repository's ranking rather than numbering rows from its own array position.

### [RANK-MASS-SUM] Rank by duplicated mass only

Duplicated mass is canonical nodes × additional visible occurrences. This formula is a Deslop product definition, not a result borrowed from the literature. Juergens et al. (ICSE 2009) establishes the fault risk of inconsistent clone changes, and Islam, Mondal, and Roy (SANER 2019) establishes that even micro-clones can be bug-prone; neither paper proposes this ranking equation. SonarQube's duplicated-line density is likewise a separate repository metric, not evidence for AST-node mass. Pair evidence already did its job at admission: an admitted edge either contributes to the closure or it does not. Once the cluster exists, only its extent and repeated membership determine mass. At equal mass, cluster id makes the order total and reproducible.

**For AI.** `mass = canonical_node_count × max(visible_members − 1, 0)`. `weight` has no independent definition: where the legacy word appears in an external explanation it means this exact mass. Sort by mass descending, then cluster id ascending. No other term is legal.

### [RANK-CATEGORY] Category never changes mass

A detection-time finding kind may drive an explicit exclusion before ranking. It is not carried as clone-cluster similarity metadata and never multiplies, discounts, boosts, or tie-breaks cluster mass. Every surviving cluster is ranked by [RANK-MASS-SUM].

### [RANK-STRUCTURAL-ONLY] Pair evidence never changes mass

`StructuralOnly` is a pair classification for a candidate whose normalized AST shape is strong but required content support is absent. It explains why that pair is rejected; it is not a cluster score and cannot be an edge in a closure. The retired `structural_only_weight`, `data_clone_weight`, and `demote` ranking modes are forbidden because they make weight mean something other than mass.

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

One further guard bounds the hide: **no two proven wrappers may share a body**. Two sibling wrappers forwarding to the same route are a copy-paste bug — one of those calls is dead or misaimed — and one shared body disqualifies the suppression for the whole family. The comparison is on bodies, not on pair evidence or reported cluster scores. Pair content evidence has already been consumed by admission and is unavailable to this post-closure filter.

Languages without a wired grammar table return false for shape 2, so their
single-declaration windows are never hidden.

### [RANK-LITERAL-FAMILY] Literal families use the same mass formula

Literal-family findings ([CLONE-CATEGORY-REGISTRY] kinds `magic_literal`, `shadowed_constant`, `constant_duplicate`, `constant_drift`, and `constant_alias`) use the same mass definition without becoming clone closure components. A literal is one canonical node, so its mass is `visible_occurrences - 1`. A literal finding with fewer than two visible occurrences is dropped and counted in `literal_findings_hidden`. Length, file spread, kind, and policy never multiply, discount, boost, or tie-break its mass.

Detection-time noise rules and explicit pre-ranking exclusion may decide whether a literal finding is visible. The retired `demote`, `magic_literal_weight`, and `constant_findings_weight` settings are forbidden because a surviving finding keeps its mass without modification.

### [RANK-UNUSED-PUBLIC] Unused-public markers never change mass

The [LITERAL-UNUSED-MARKER] may label or filter a constant-family finding, but it never changes mass. The retired `unused_public = "boost"` mode and `unused_public_weight` setting are forbidden.

### [STATE-FILE-REGISTRY] File registry (the only global state)
`deslop-core::state::FileRegistry` maps `FileId ↔ PathBuf`. This is the *only* place mutable state associated with a pipeline run may live. Instances are per-run (not process-global) so a future long-running daemon can keep multiple analyses side-by-side.

### [OUTPUT-SCHEMA-JSON] Canonical JSON schema
JSON is the canonical report format ([PRINCIPLES-AUDIENCE-AGENT]). Text and HTML are derived from it — nothing lives in two places. Text is terse and AI-readable (ASCII, line-oriented, no colour). HTML is single-file, inline-CSS, and human-readable; its schema-reference section renders only when the report carries a non-empty `schema_doc`, which the CLI's rendered reports do not (see below).

Top level:

- `tool_version: String` — producer binary version.
- `min_nodes: u32` — subtree size floor used for the run.
- `files_analysed: usize` — count of files actually parsed.
- `clusters_hidden: usize` — clusters that existed but were suppressed from `clusters` because every occurrence matched a [EXCLUSION-CONFIG] `report_hide` pattern. Surfaces the volume of ignored duplication without leaking the content.
- `cache_stats: { hits: usize, misses: usize }` — incremental fingerprint-cache telemetry per [PIPELINE-INCREMENTAL]. Both zero when `--no-incremental` was passed; otherwise `hits + misses == files_analysed` for files whose language has a registered parser.
- `metrics: RepoMetrics` — repo-wide duplication totals per [METRICS-REPO]. Always populated; zero when no duplication exists.
- `schema_doc: String` — markdown explaining every field, signal, threshold, ranking formula, byte-range convention, and clone taxonomy, sourced via `include_str!` from `REPORTING-CONTEXT.md` so it cannot drift from the schema. The CLI ships the field **present but empty** in every rendered report (#110/#111) — inlining ~13 KB into each report drowns the actual content — and the document is served on demand instead (`schema-doc` tool, `deslop://schema` resource per [MCP-*]). Pinned by the committed report golden.
- `embedding_provenance: Option<EmbeddingProvenance>` — provider/model identity plus `attempted_subtrees`, `succeeded_subtrees`, `indexed_subtrees`, and `failed_subtrees` so embedding coverage is visible. The first, second and fourth count **occurrences** and satisfy `attempted = succeeded + failed`; `indexed_subtrees` counts **distinct index points** and satisfies `indexed <= succeeded`. Mixing the two units is how a collapsed pass comes to look like a lossy one — `indexed/attempted` is not a coverage ratio. Duplicate successful snippets collapse before ANN indexing, and provider-rejected subtrees are omitted from the embedding ANN input; they are never represented as zero vectors.
- `clusters: Vec<ReportCluster>` — ranked worst-offenders-first per [PIPELINE-RANK-WORST-FIRST].
- `literal_findings: Vec<LiteralFinding>` — dedicated literal and constant findings per [LITERAL-WIRE], separate from clone clusters.
- `literal_findings_total: usize` and `literal_findings_hidden: usize` — visible and omitted literal-finding counts; neither contributes to `clusters_total`.
- `literal_findings_capped: bool` and `literal_max_findings: usize` — whether [LITERAL-NOISE]'s per-kind cap omitted findings and the configured cap that produced the report; `literal_max_findings == 0` means unlimited.

`ReportCluster`:

- `id`, `mass`, `canonical_node_count`, `rank`, `rank_band` — cluster identity, canonical extent, duplicated mass, and mass-derived order metadata.
- `occurrences: Vec<ReportOccurrence>` — each with `path`, `start_byte`, `end_byte`, and `hidden: bool` (true when the occurrence matched a `report_hide` pattern per [EXCLUSION-CONFIG]).

`ReportCluster` carries identity, canonical extent, occurrence membership, mass, and rank. Pair evidence is returned only by an explicit comparison of two occurrences.

Default output paths, the format suppressors, and `--from-report` re-rendering are invocation behaviour, owned by [cli.md §OUTPUT-FORMAT-DERIVED](cli.md).

#### [OUTPUT-SCHEMA-DIFF-TAGS] Diff-scope tags

> **Status: shipped.** Field presence and absence are both pinned by `crates/deslop/tests/diff_scoped_reporting.rs`.

Under `--diff` ([cli.md §CLI-ARG-DIFF](cli.md)) the report carries the diff verdicts; without it every field below is **absent**, never defaulted `false` — a run given no diff asserts nothing about one. Intersection is closed-interval over the 1-indexed `start_line`/`end_line` the occurrence already carries; one added line inside an occurrence tags it, because touching a clone counts as touching the clone. `intersects_diff` ignores `hidden` occurrences, matching the [METRICS-REPO] projection; `is_newly_introduced` does **not** — a hidden pre-existing copy vetoes the flag, because content that already existed anywhere in the tree did not arrive with this change, and claiming otherwise in a merge gate would be a false accusation.

- `ReportOccurrence.in_diff: Option<bool>` — the occurrence's lines intersect an added span for its path.
- `ReportCluster.intersects_diff: Option<bool>` — ≥ 1 non-hidden occurrence in diff.
- `ReportCluster.is_newly_introduced: Option<bool>` — `intersects_diff` holds **and** every occurrence, hidden included, is in diff (#364's "all occurrences" definition).
- `clusters_outside_diff: Option<usize>` (top level) — clusters `--only-changed` omitted from `clusters`.
- `metrics.diff: Option<DiffMetrics>` — [METRICS-DIFF-SCOPE].

Modelled in [live-ipc.td](../models/live-ipc.td), regenerated, never hand-written; live/LSP/MCP sessions carry `None` throughout. Text and HTML derive from the tags: occurrence badges (`[in diff]` / `[existing]`) through the one shared occurrence renderer, a CSS-only "only diff-affected" toggle in HTML, and the `--only-changed` stderr delta summary ([cli.md §CLI-ARG-ONLY-CHANGED](cli.md)).

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
      embeddings/                  # [FUSED-EMBED-PROVIDER]
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

`[report] split_by_language` in `.deslop.toml` (default `false`, with a `--split-by-language` CLI mirror) divides the report body into one `<section>` per language instead of the single `Duplicate groups` section. Cluster cards remain in engine mass order and are never grouped by pair classification. With the flag off, output is byte-identical to the single-section form. With it on, `write_clusters` groups clusters by the stable first occurrence's `language_for_path(...)`, emits one heading per language with its group count, preserves engine rank within each section, and orders sections by the lowest engine rank they contain.

### [METRICS-REPO] Repo-wide duplication metrics

One repository duplication percentage is computed deterministically from the visible cluster set and carried at `Report.metrics`. It is pure line coverage and drives the single fail-over gate in [EXIT-CODES]. Pair evidence never changes it.

`RepoMetrics` fields:

- `analysed_loc: u64` — physical lines across every file in `files_analysed`. Counted once per file, regardless of clustering. Lines are `\n`-terminated plus the trailing partial line if any; empty files contribute zero.
- `duplicated_loc: u64` — lines covered by **≥ 2 clone occurrences across the whole corpus**, deduplicated per file so overlapping sibling-extension ranges do not double-count. Computed by projecting every `ReportOccurrence` from every non-hidden clone cluster onto a per-file `BTreeSet<line>`, unioning, and summing set sizes. Hidden occurrences (`[EXCLUSION-CONFIG]` `report_hide`) are excluded. Dedicated literal findings ([RANK-LITERAL-FAMILY]) are not clone clusters and never enter `duplicated_loc`, `duplication_percent`, or `clusters_total`. Every visible fragment-clone line counts equally. Pair classification and pair evidence are unavailable to this calculation.
- `duplication_percent: f64` — `100.0 × duplicated_loc / analysed_loc`, clamped into `[0.0, 100.0]`. Zero when `analysed_loc == 0`. Rounded to two decimals in text + HTML; carried at full `f64` precision in JSON.
- `clusters_total: usize` — count of non-hidden clone clusters carried in `clusters`; always equals `clusters.len()` — including after `--only-changed` filtering, where the repo-wide count is recovered as `clusters_total + clusters_outside_diff` ([METRICS-DIFF-SCOPE]) — but is carried explicitly so downstream consumers do not re-derive it. Dedicated literal findings have their own count under [RANK-LITERAL-FAMILY].
- `duplicated_files: usize` — count of files containing at least one non-hidden clone occurrence. Upper-bounded by `files_analysed`.
- `per_file: Vec<FileMetric>` — per-file breakdown, one `FileMetric { path, analysed_loc, duplicated_loc, duplication_percent }` per analysed file (clean files included with `duplicated_loc == 0` so percentage denominators stay exact). Same per-file line-set computation as the repo aggregate, scoped to one file; `duplication_percent` uses that file's own `analysed_loc` as the denominator. Sorted by `duplication_percent` desc, path tiebreaker. **`path` is rendered relative to the scan root**, the same form `ReportOccurrence.path` carries, so a consumer that opens a `FileMetric` must resolve it against the workspace exactly as it resolves an occurrence — treating it as absolute names a file that does not exist. Powers the per-file rows in [VSIX-METRICS-PANEL].
- `folders: Vec<FileMetric>` — engine-computed per-folder rollup, one row per folder prefix containing at least one duplicated line. Each row sums the `analysed_loc` and `duplicated_loc` of every `per_file` row under the prefix — clean files included, keeping the denominator exact — and derives `duplication_percent` through the **same single `percent` function** as the repo and per-file figures. Consumers render these rows verbatim; recomputing a folder percentage (or re-summing folder LOC) outside the engine is prohibited — every duplication percentage on every surface traces back to this one function. Paths are scan-root-relative folder prefixes joined with `/` on every platform; sorted `duplication_percent` desc, path tiebreaker, exactly as `per_file`. Zero-duplication folders are omitted. `#[serde(default)]` on the wire so reports written before the field parse as empty. Powers the folder rows in [VSIX-METRICS-PANEL].

#### [METRICS-REPO-WEIGHTED] Evidence weighting is prohibited

There is no evidence-weighted duplication percentage. Structural, Jaccard, embedding, and content evidence belong to pairs and decide admission; projecting any of them onto a closure component invents a cluster score. `bucket_weights`, `category_weights`, `WeightedMetrics`, `weighted_duplicated_loc`, `weighted_duplication_percent`, and a `[metrics]` weight table are forbidden wire and configuration fields.

The text renderer prints one line: `repo: 12.4% duplicated (1 843 / 14 876 LOC, 27 clusters across 11 files)`. HTML renders the same engine-computed metric and colours it by the one fail-over threshold. JSON carries the same canonical value. No surface computes a companion figure.

#### [METRICS-DIFF-SCOPE] Diff-scoped duplication percentage

> **Status: shipped.** Pinned by `crates/deslop/tests/diff_scoped_reporting.rs`.

Under `--diff`, `RepoMetrics` gains `diff: DiffMetrics { added_loc: u64, duplicated_added_loc: u64, duplication_percent: f64, threshold: ThresholdSummary }` — absent without the flag. Numerator: added lines covered by the same non-hidden, non-literal-family occurrence projection as `duplicated_loc`. Denominator: added lines in analysed files. The same clamp, rounding, and zero-denominator rules apply. The repository-wide fields are byte-identical with and without `--diff`; no knob may change `duplication_percent`.

Under `--only-changed` the [EXIT-CODES] mechanical gate reads `diff.duplication_percent` against the same threshold sources, and the report header names the scope (`threshold: 10.00% of added lines (ok)`). Without `--only-changed` the gate is untouched even when `--diff` is present — tagging alone must not move a CI verdict.

`--only-changed` filtering never touches the repo-wide **line** metrics (`analysed_loc`, `duplicated_loc`, `duplication_percent`, `duplicated_files`, `per_file`, `threshold`), but `clusters_total` follows the filtered body so the [METRICS-REPO] invariant — the banner counts the list it sits above — survives filtering. The repo-wide cluster count stays recoverable as `clusters_total + clusters_outside_diff`, and that sum is what the repo-scoped line in text and HTML renders. Every surface derives its verdicts from the **governing** gate: the HTML banner's colour class and named threshold verdict come from `diff.threshold` whenever the CLI resolved one (its `source` is non-`none` only under `--only-changed`), so the page can never render green while the run exited `3`. The `--only-changed` delta summary (text, stderr, HTML banner tail) carries four reconciling figures — intersecting = newly introduced + cross-file-with-untouched-code, plus the omitted count — and a filtered-empty run says "no diff-affected duplication" with the omitted count, never that the codebase is clean.

### [EXIT-CODES] CLI exit codes and fail-over threshold

Deslop's default exit code is `0` on a successful analysis regardless of how much duplication exists — the tool is diagnostic, not opinionated. Opt-in CI gating is expressed through a single flag and a single config key.

Exit codes:

- `0` — analysis succeeded; no enabled gate breached.
- `1` — unexpected runtime error (parse failure, I/O error, cache corruption that couldn't be recovered). Pre-existing behaviour; unchanged by this spec.
- `2` — invalid CLI invocation (bad flag, incompatible combination, missing required argument). Pre-existing behaviour; unchanged.
- `3` — **duplication threshold breached.** `metrics.duplication_percent > threshold` after a successful analysis. Under `--only-changed` the gate reads the diff-scoped percentage instead ([METRICS-DIFF-SCOPE]). The report is still written to disk in full so CI can surface the offenders.

Threshold sources, highest precedence first:

1. `--fail-over <percent>` CLI flag. Accepts a finite float in `[0.0, 100.0]`. `--fail-over 0` means "fail on any duplication". Invalid values → exit `2` with a named error.
2. `[threshold] max_duplication_percent` in `.deslop.toml` (or the file passed via `--config`). Same validation rules.
3. Absent — no threshold is enforced; exit `3` is unreachable and the text/HTML headers render the metric without a pass/fail verdict.

A `--no-fail-over` flag (mutually exclusive with `--fail-over`) overrides a config-file threshold and restores the "report only" behaviour, so a developer can run the CLI locally against a repo whose CI gate they don't want to trip.

#### [EXIT-CODES-WEIGHTED] Evidence-weighted gates are prohibited

There is no `--fail-over-weighted`, `max_weighted_duplication_percent`, or weighted threshold on the wire. [EXIT-CODES] owns the one duplication-percentage gate. Pair evidence cannot alter a cluster metric or a repository gate.

The renderer always states the active threshold in the report header (`threshold: 10.00% (breached)` / `threshold: 10.00% (ok)` / `threshold: none`) so the report is self-explanatory when read out of context. The threshold value and breach flag are carried on `Report.metrics.threshold { percent: f64, breached: bool, source: "cli" | "config" | "none" }` so downstream tools do not re-derive the verdict.
