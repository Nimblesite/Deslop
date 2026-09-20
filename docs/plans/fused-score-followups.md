# Pair admission and mass-only clusters — wholesale cutover plan

This plan replaces the shipped cluster-evidence design in one cutover so code, tests, generated wire models, CLI, LSP, MCP, reports, and VSIX agree with [fused.md](../specs/fused.md). There is no compatibility stage, adapter, deprecated field, dual rendering path, fallback, or period in which the old model is preserved.

## Governing contract

- Structural similarity, token Jaccard, embedding similarity, content agreement, rename consistency, literal fraction, admission result, and pair classification belong to one exact pair.
- Candidate pairs are admitted under [FUSED-STRATEGY-BOUNDED-MAX](../specs/fused.md#fused-strategy-bounded-max). Clusters form from the transitive closure of admitted pairs.
- Preserve genuine clone subsets when separating noisy or informational relations ([CLONE-NOISE-VERBATIM-SUBGROUP], [CLONE-KIND-FOLD](../specs/taxonomy.md#clone-kind-fold-compare-the-actual-members)). Rejected or indirect comparisons must not mislabel near-copies as Similar code.
- A clone cluster owns identity, membership, kind, AST size, duplicated mass and rank. Raw pair measurements stay on explicit comparisons. Shape-only information is a non-clone, listed last with zero duplication weight ([CLONE-BUCKETS-STRUCTURAL-ONLY](../specs/taxonomy.md#clone-buckets-structural-only-shape-only-is-not-duplication)).
- Keep the existing AST-node formula and eligibility rules in [RANK-MASS-SUM](../specs/pipeline.md#rank-mass-sum-rank-by-duplicated-mass-only). Do not introduce another weight or formula.
- Pair evidence renders only after a caller explicitly identifies two distinct occurrences. The VSIX pair view uses a compact `PAIR EVIDENCE` surface; content evidence is a muted secondary line, never a cluster card.

## One destructive replacement

- [x] Cleanse the governing specifications so pair evidence, closure, convicted-noise handling, mass, wire ownership, presentation, metrics, and severity do not contradict one another.
- [ ] Replace the canonical wire model in `docs/models/live-ipc.td`: delete every cluster `signals`, `bucket`, `category`, `interpretation`, evidence-verdict, pair-source, and fused-gate field; add one explicit pair-comparison request/response keyed by two occurrence endpoints; regenerate Rust and TypeScript models once.
- [ ] Keep the engine-authored kind under [CLONE-KIND-FOLD]. Delete component evidence averages, maxima, confidence scores and weighting multipliers; never copy one pair's measurements onto a group.
- [ ] Make pair measurement the single owner of `S`, `J`, `E`, `A`, `R`, literal fraction, admission, and optional pair classification. Store or recompute the exact endpoint-keyed record without copying it into a component.
- [ ] Form candidates from admitted pairs, preserve established clone subsets, and separate informational relations before counting or ranking ([CLONE-KIND-FOLD]). No rejected pair becomes a clone through an indirect connection.
- [ ] Replace report weighting wholesale with [RANK-MASS-SUM](../specs/pipeline.md#rank-mass-sum-rank-by-duplicated-mass-only). Delete every multiplier, boost, confidence factor, spanned-byte factor, and evidence tie-break. Equal mass sorts by cluster id.
- [ ] Update every renderer and client to show shared category names, actual-clone mass and rank, and separate shape-only information last. Pair measurements appear only in explicit comparisons.
- [ ] Use Similar code as the public label for `loosely_similar`. Same shape, different content is informational, not a merge recommendation or a default diagnostic.
- [x] Restore one-click canonical comparison ([VSIX-PAIR-COMPARE]) while preserving the explicit `deslop.comparePair` endpoint command. Both routes show the exact source ranges in the native diff; engine pair measurements remain separate from cluster surfaces ([VSIX-PAIR-EVIDENCE]).
- [ ] Implement category grouping and configurable diagnostic severities from [FACET-GROUP-BY-KIND] and [SEVERITY-CONFIG](../specs/severity.md#severity-config-configuration). Diagnostic severity never comes from mass rank; shape-only defaults to no diagnostic.
- [ ] Separate literal-family findings from clone closure components so literal kind cannot masquerade as pair classification or cluster evidence. Kept literal findings use unmodified mass.
- [ ] Delete evidence-weighted repository metrics, weight tables, weighted gate flags, weighted wire fields, configuration, renderers, and tests. The one repository duplication percentage remains unweighted line density.

## Assertions that must fail before the replacement and pass after it

- [ ] Black-box reports assert kinds, exact occurrence paths/ranges/counts, clone mass, ordering, and clone-only metrics. Shape-only-only input reports zero duplication; mixed groups preserve the real clones ([CLONE-KIND-TESTING](../specs/taxonomy.md#clone-kind-testing-required-examples-and-assertions)).
- [ ] Pair-comparison tests select two concrete endpoints and assert exact `S`, `J`, `E`, `A`, `R`, literal fraction, admission result, and pair classification. Reversing endpoint order preserves symmetric evidence while replacing either endpoint asks a different relation; a cluster id alone cannot request evidence.
- [ ] Grouping tests preserve exact copies and near-copies beside unrelated or more extensively edited members; rejected comparisons cannot supply clone membership or a fallback clone label.
- [ ] Ranking tests retain the AST-node mass formula for actual clones and assert zero duplication weight for every non-clone. Informational visibility and diagnostic overrides never change clone rank or percentages.
- [ ] LSP/MCP and VSIX tests assert shared category names, configured diagnostic defaults and overrides, and no shape-only diagnostics by default. Raw pair evidence requires explicit endpoints ([SEVERITY-TESTING](../specs/severity.md#severity-testing-required-checks)).
- [x] VSIX unit and Playwright assertions pin [VSIX-PAIR-COMPARE]: the canonical row is disabled, one peer click posts that exact occurrence, and the native diff shows canonical bytes alongside the selected peer even for a third occurrence in the same file. Cluster pages continue to exclude pair measurements ([VSIX-PAIR-EVIDENCE]).
- [ ] Generated-model tests assert removed cluster fields do not exist in Rust or TypeScript and that hand-written mirror types cannot drift from the generated contract.
- [ ] Regression fixtures assert exact clusters, occurrences, paths, ranges, mass, and order across every affected language; no assertion is weakened to a cluster count.

## Whole-system proof

- [ ] Run formatting, lint, generated-model verification, Rust build, Rust tests with coverage, TypeScript typecheck, VSIX unit tests, Playwright webview smoke, packaging verification, and the full repository CI target with zero failures.
- [ ] Verify the installed VSIX: category names/order, explicit comparisons, kind-based diagnostic defaults/overrides, and clone-only totals. A watched edit refreshes every surface. Follow the deployment workflow without killing VS Code.
- [ ] Re-run repository-wide searches for every removed cluster field, old command, old label, compatibility shim, multiplier, and weighted-metric surface. Only historical issue prose outside executable/spec contracts may remain.
- [ ] Run [spec-check](../../.agents/skills/spec-check/SKILL.md) and [ci-prep](../../.agents/skills/ci-prep/SKILL.md). Submit through [submit-pr](../../.agents/skills/submit-pr/SKILL.md) only after all gates pass.

## Completion

The plan is complete only when the source specs, generated schema, Rust model, pair admission, closure and suppression behavior, mass ranking, every renderer, every client, every assertion, and the installed VSIX UI all enforce the same boundary with no old path left in the repository.

## The accessor-pair pins (gh #532)

The two accessor-pair tests in `crates/deslop/tests/content_gate_signal_honesty.rs` run without skips. They reject the unrelated accessor functions and the original short spans through explicit pair comparisons, preserve the measured evidence checks, and require the byte-identical control to remain the only published cluster. Exact control membership, rank, mass, node count, occurrence count, and report metrics are asserted alongside the absence of pair-only fields on cluster surfaces ([FUSED-CONTENT-GATE], [FUSED-SHARED-SUBTREE-CORE], [RANK-MASS-SUM]).

Their entries are removed from `CURATED_SKIPS` and `SKIPS_PER_ISSUE`; the skip-policy gate rejects their reintroduction. The pair remains CLEARLY OUT in `corpus/register/deslop.json`, so the corpus gate also rejects any change that publishes it. GitHub issues remain open.
