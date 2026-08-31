# Embedding candidate accuracy — implementation plan

This plan owns embedding candidate recall, false-positive control, provider fidelity, and ANN determinism under the pair-scoped contract in [fused.md](../specs/fused.md). Embeddings expand the candidate set and supply pair evidence `E`; they do not create a cluster-level fused score, change existing pair measurements, scale ranking weight, or replace content routing.

## Governing contract

- Structural and token discovery always run; embeddings are optional and local by default ([FUSED-SIGNALS-THREE-LAYER](../specs/fused.md#fused-signals-three-layer)).
- Pair admission uses `f_admit = max(H,J,E)` before rescue. `E` is exact cosine from `embedding::cosine_similarity`, accumulated in `f64`, clamped to `[0,1]`, and zero for a zero-norm or absent vector ([FUSED-ALGEBRA](../specs/fused.md#fused-algebra)).
- HNSW distance discovers candidates only. Rendered `embedding_cos` is remeasured by the canonical cosine function and belongs to the elected admitted pair ([FUSED-EMBED-PROVIDER](../specs/fused.md#fused-embed-provider), [FUSED-CLUSTER-SIGNALS](../specs/fused.md#fused-cluster-signals)).
- Candidate admission still obeys endpoint-size coherence, the applicable LSH-only guard, pair-specific thresholds, and the separate shared-subtree rescue. An ANN edge cannot waive a guard merely because it exists.
- Provider input limits are provider-owned. Oversized inputs count as failures and are never silently truncated.
- Final clustering and ranking operate on the union of structural, token, and embedding candidates. Adding embeddings may add real pairs; it must not alter the evidence or membership of an already admitted structural/token component through discovery topology.

## Execution rule — one destructive replacement

Replace the embedding candidate subsystem in one indivisible cutover. Delete the defective candidate generation, stale guards, obsolete mock behavior, removed exact-pair assumptions, ANN bridge behavior, incompatible tests, and every dependent expectation up front. Do not preserve old entry points, dual algorithms, fallback selection, compatibility flags, temporary adapters, or a compiling intermediate state. Core, CLI, LSP, provider harnesses, generated models, fixtures, and documentation all move to the final contract together. The build and test suite may remain broken throughout the work; only the finished replacement is required to compile and pass, with no ignored acceptance tests.

## Current measured state

- The deterministic mock now uses a 128-lane signed five-byte-shingle signature. It separates the real `ts-mixed-band` near-copy from whole-function/chunk and parameter/chunk false positives while keeping identical text at cosine `1.0`.
- Duplicate snippets collapse to one ANN point and fan the vector back to every owning fingerprint (#357). `attempted_subtrees = succeeded_subtrees + failed_subtrees` is occurrence-based; `indexed_subtrees ≤ succeeded_subtrees` is unique-vector based.
- The small-corpus exact-pair guard was removed even though [FUSED-TUNING-LEVERS](../specs/fused.md#fused-tuning-levers) still specifies `candidates.embedding_exact_pair_limit = 256`. Small-corpus recall therefore lacks its specified protection against top-k truncation.
- Real Ollama measures the Type-4 `totalRecursive`/`totalIterative` pair below the current candidate floor without shared text and above it with a shared docstring (#407). This is a production recall defect, not a reason to lower a threshold without a corpus sweep.
- Embeddings-on route-invariance fixtures still expose ANN bridge effects (#356): an embedding pass can change or erase clusters that structural/token discovery already found.

## Wholesale replacement scope

**Replace the acceptance surface with the final contract.**

- The final cutover removes every #369 `#[ignore]` and makes the original full-strength assertions pass: pair-size coherence, bounded pair admission, and LSP embedding determinism. No intermediate suite state is maintained.
- Assert exact cluster IDs, occurrence counts, file paths, buckets, elected source pair, `S/J/E`, content support, and ranking order. A cluster-count-only test is insufficient.
- Separate deterministic-mock tests from real-Ollama tests. The lexical mock validates plumbing and repeatability; only a declared semantic fixture may validate Type-4 meaning.

**Replace Type-3 recall without redefining fused.**

- Pin the parenthesised TypeScript near-copy from #367 with embeddings off. The enclosing pair begins with `H = 0` and weak `J`; graded `S` must be measured through [FUSED-SHARED-SUBTREE](../specs/fused.md#fused-shared-subtree), the compound rescue must admit it, and its pre-rescue fused value must remain unchanged.
- Assert the enclosing function ranges, not surviving nested fragments, in every supported language fixture covered by `type3_enclosing_method.rs`.
- Remove language-specific LSH promotion such as `is_csharp_lsh_type3_near_miss`; [CLONE-BUCKETS-ROUTING](../specs/taxonomy.md#clone-buckets-routing) row 4b is evidence-based and language-agnostic.
- Treat multiset or weighted token signatures as a separate representation change. It requires a named spec section, cache-epoch change, positive and negative corpus measurements, and proof that repetitive scaffolding does not gain false-positive recall. Do not lower `fused_threshold` or the LSH-only floor.

**Replace candidate guards and restore small-corpus recall.**

- Restore an exact cosine pass for candidate sets at or below `candidates.embedding_exact_pair_limit`; use the configured limit, the canonical cosine function, and the existing precomputed norms. Pin a near-tied small corpus where HNSW top-k alone drops the only declaration-level pair.
- Keep HNSW for larger sets with fixed seed and construction parameters. Exact and ANN paths must produce the same pair measurements where their candidate sets overlap.
- Pin [PAIR-SIZE-COHERENCE] with the 19-node parameter-list versus 274-node arithmetic-chain false positive; no unanchored pair outside `max_endpoint_node_ratio` may survive regardless of cosine.
- Resolve #365 by replacing the algebra, code, and tests simultaneously: if a sub-threshold positive cosine currently lets a token-carried pair evade the LSH-only guard, the final `lsh_ok` contract must state that only embedding evidence independently clearing `t(p)` waives the guard. No undocumented condition or transitional behavior survives.

**Replace the test provider with an honest bounded instrument.**

- Keep identical input exactly `1.0`, renamed near-copies high, unrelated shape/chunk pairs low, and vector width fixed. Assert these relationships directly instead of preserving historical numeric noise.
- Benchmark the exact path at 128 lanes and the configured exact-pair limit; the large-subtree context-budget fixture must complete without falling into `O(N²)` work above the guard.
- Keep the mock behind `test-support`, absent from `ProviderRegistry::production`, and excluded from the shipped VSIX ([REMOVE-STUB](../specs/fused.md#remove-stub)).
- Preserve provider-owned input budgets, `failed_subtrees`, model identity/version cache keys, and no-truncation behavior while changing candidate generation.

**Replace the broken Type-4 candidate path — gh #407.**

- Keep `dart_same_role_function_pair_still_surfaces` red and unchanged until a real provider forms the semantic pair.
- Measure candidate recall and false positives across the real-Ollama corpus for model, representation, exact/ANN search, top-k, and candidate-floor variants. Record the full precision/recall table before changing a default in [FUSED-TUNING-LEVERS](../specs/fused.md#fused-tuning-levers).
- Do not use the lexical mock to prove semantic equivalence. Either run these fixtures under the real-Ollama gate or add a deterministic semantic mode with an explicit fixture-to-vector contract that cannot be triggered by incidental lexical overlap.
- Reclassify #358 as candidate/harness recall unless a formed candidate is demonstrably suppressed by the role gate. A missing pair cannot prove a routing defect.

**Replace ANN bridge behavior with route-invariant deterministic discovery.**

- Fix #356 so adding embedding candidates cannot weld independent structural components, replace an admitted pair’s evidence, or change a bucket solely because the discovery path changed. Compare embeddings-off and embeddings-on occurrence sets before comparing buckets.
- For repeated runs with identical provider/model/version, assert identical candidate pairs, admitted pairs, elected source pairs, clusters, order, and provenance. Fixed ANN parameters are necessary but not sufficient.
- A zero-success refresh must emit terminal failure and preserve the previous good report; partial success must report attempted, succeeded, failed, and indexed counts in their documented units.
- Keep hidden-cluster diagnostics structured and content-free; tests must inspect rendered reports rather than depend on logs.

## Completion

This plan is complete when the #369 and #407 pins run without ignores, Type-3 rescue works with embeddings off, the specified exact-pair guard protects small corpora, unanchored false positives fail the documented gates, embeddings-on preserves existing structural/token results, real semantic recall is measured rather than mocked lexically, repeated runs are deterministic, and every reported cosine is the measured pair’s canonical `f64` measurement.

Related ownership: cluster-level fused removal and elected content evidence live in [fused-score-followups.md](fused-score-followups.md); token-representation redesign beyond the specified rescue requires `rename-recall-plan.md`; corpus adjudication lives in [corpus-assertion.md](corpus-assertion.md).
