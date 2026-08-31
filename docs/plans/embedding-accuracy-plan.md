# Embedding pair accuracy — wholesale replacement plan

This plan owns embedding candidate recall, pair measurement, false-positive control, provider fidelity, and ANN determinism under [fused.md](../specs/fused.md). Embeddings propose and support exact pairs. They never create cluster evidence, classification, severity, weight, or a repository metric.

## Contract

- Structural and token discovery always run. Embeddings are optional and local by default ([FUSED-SIGNALS-THREE-LAYER](../specs/fused.md#fused-signals-three-layer)).
- `E(p)` is the canonical cosine for one pair, accumulated in `f64`, clamped to `[0,1]`, and zero for absent or zero-norm vectors. ANN distance proposes neighbours only; it is never rendered.
- Pair admission uses `max(S,J,E)` with the pair-content and guard rules in [FUSED-ALGEBRA](../specs/fused.md#fused-algebra). Shared-subtree rescue remains a separate compound pair route.
- A pair response identifies both endpoints and may render `E(p)` beside that pair's other evidence. No embedding value appears in a cluster record or cluster surface.
- Cluster mass remains [RANK-MASS-SUM] regardless of whether embeddings ran.

## One destructive replacement

- [ ] Replace embedding candidate construction with one exact endpoint-keyed pipeline: embed eligible occurrences, exclude provider failures, build exact or ANN neighbours, remeasure every proposed pair with the canonical cosine, and hand the pair to the common admission function.
- [ ] Restore the specified exact-pair path for corpora at or below `candidates.embedding_exact_pair_limit`; delete any top-k-only shortcut that can lose a qualifying small-corpus pair.
- [ ] Delete every path that copies an embedding value into a cluster, uses it to select a component edge, changes cluster severity or order, or synthesizes an AI-match cluster label.
- [ ] Enforce role-mismatch and endpoint-size guards before an embedding-carried edge is admitted. Do not lower thresholds to rescue a fixture.
- [ ] Keep the deterministic embedding stub behind `test-support`, absent from `ProviderRegistry::production`, and excluded from the VSIX. Production registries expose only real providers.
- [ ] Keep cache identity bound to provider, model, model version, dimensions, normalization contract, source bytes, language, and semantic epoch. Delete stale or ambiguous key paths rather than adding fallbacks.
- [ ] Make provider failure accounting exact: `attempted = succeeded + failed`, `indexed ≤ succeeded`, and failed occurrences never become zero vectors.
- [ ] Expose embedding provenance at report/session level only. Cluster payloads remain membership plus mass; explicit pair responses carry `E(p)` only when those endpoints were measured.

## Accuracy assertions

- [ ] Exact-pair tests assert both endpoint identities, exact cosine, all other pair evidence, admission result, and optional pair classification. No test reads embedding evidence from a cluster.
- [ ] Small-corpus tests prove a qualifying pair outside an ANN top-k neighbourhood is still found by exact enumeration.
- [ ] ANN tests compare candidate recall with exact search on a fixed corpus and assert deterministic endpoint pairs after canonical remeasurement.
- [ ] Provider-failure tests assert failed occurrences are excluded, accounting identities hold, no zero-vector pair appears, and deterministic structural/token results remain unchanged.
- [ ] Role-mismatch and size-coherence fixtures assert the false pair is rejected while a nearby genuine semantic pair remains admitted.
- [ ] Embeddings-off/on comparison asserts existing deterministic admitted pairs, cluster membership, mass, and order do not change except where newly admitted embedding edges legitimately alter closure membership.
- [ ] Cache tests assert provider/model changes miss, byte-identical inputs hit, corrupted entries self-heal, and cold/warm pair records are identical.
- [ ] VSIX tests assert no cluster surface displays embedding similarity. Explicit Compare renders the exact pair's embedding value compactly and only while the pair view is open.

## Corpus calibration

- [ ] Measure candidate recall and false positives for model, representation, exact/ANN search, top-k, and candidate-floor variants on the real-Ollama corpus.
- [ ] Record endpoint pairs, admission outcomes, false positives, false negatives, provider failures, and runtime. Cluster counts alone are not evidence.
- [ ] Change a default only with an updated [FUSED-TUNING-LEVERS](../specs/fused.md#fused-tuning-levers) provenance row and black-box positive and negative fixtures.

## Whole-system proof

- [ ] Run Rust format, lint, build, tests with coverage, generated-model verification, TypeScript typecheck, VSIX unit tests, Playwright webview smoke, packaging verification, and full CI.
- [ ] Build and install the current VSIX without killing VS Code. Verify clusters show membership and mass only; explicitly compare two occurrences and verify the pair-only embedding value; switch the embedding model through the real UI and verify provenance plus reactive refresh.
- [ ] Search the repository for cluster embedding fields, AI-match cluster labels, component edge selection by embedding, compatibility shims, and test-only providers in production registries; all must be absent.

## Completion

This plan is complete when exact and ANN candidate paths agree within the specified recall contract, every embedding value belongs to two named endpoints, no cluster or rank consumes embedding evidence, provider and cache behavior are fully pinned, and the installed VSIX proves the pair-only UI.
