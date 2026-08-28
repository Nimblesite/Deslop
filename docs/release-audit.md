# Release regression audit

Audit date: **2026-08-28**

Baseline: [`f92300e5e1004ef6c53a94174a0d7e842232ec80`](https://github.com/Nimblesite/Deslop/commit/f92300e5e1004ef6c53a94174a0d7e842232ec80) (`v0.32.0`)

Candidate: [`4177edbb514f0c7810c1f507a6b67e24e1926879`](https://github.com/Nimblesite/Deslop/commit/4177edbb514f0c7810c1f507a6b67e24e1926879) (`Fixes`), the branch commit on top of [`7d6d6996d58a5b49e34d86f9a5eb23a81e48c2cf`](https://github.com/Nimblesite/Deslop/commit/7d6d6996d58a5b49e34d86f9a5eb23a81e48c2cf)

## Verdict

**Cleared for the release scope. No serious regression remains in the audited scope.**

The working-tree changes fix the post-baseline persisted-signature regression and close the Type-2 corpus recall proof gap found in the first audit. Their regression assertions now run in the default gate and pass. The full release gate and the scheduled real-repository corpus slice also pass.

The operator-drift false positive remains serious, but it is not a regression from this baseline. It reproduces on both `f92300e5` and the candidate at `--min-nodes 30`; at `--min-nodes 4`, the baseline produces more operator-only act-now findings than the candidate. It remains tracked by [#432](https://github.com/Nimblesite/Deslop/issues/432), but it does not invalidate the regression verdict.

The two remaining red accuracy assertions are the embedding-path failures tracked by [#369](https://github.com/Nimblesite/Deslop/issues/369) — the fusion routing pin (`mid_band_cluster_confidence_never_exceeds_its_strongest_axis`) and the LSP embedding-refresh pin (`lsp_embedding_refresh_is_bounded_and_reproducible`). These are the one exception this audit carves out: they are excepted as embedding issues and are not counted as regressions from `f92300e5`.

## Issues touched in this branch

The branch adds one commit, [`4177edb`](https://github.com/Nimblesite/Deslop/commit/4177edbb514f0c7810c1f507a6b67e24e1926879) (`Fixes`), on top of `7d6d6996`. It touches two issues — [#433](https://github.com/Nimblesite/Deslop/issues/433) and [#439](https://github.com/Nimblesite/Deslop/issues/439) — across 17 tracked files. The relevant implementation and assertion changes are:

- Content evidence, token signatures, and persisted signatures now apply one language-aware boilerplate exclusion. This removes the cold-versus-mixed persistence denominator mismatch.
- The LSH-only persistence assertion is no longer ignored. Cold, fully warm, mixed, and reverted passes now produce the same verdict and evidence.
- Curated Type-2 entries now require a positive `min_nodes` extent. A visible cluster must span the curated files, be gate-vouched, show the curated occurrences, and reach that extent.
- Tokio and Nest carry measured extent floors. The manifest contract rejects missing or zero floors before a repository scan begins.
- The three Type-2 extent assertions are no longer ignored. They reject missing extent, a far-too-small fragment, and a boilerplate family touching the curated paths.
- The curated skip registry falls from 20 entries to 16: [#433](https://github.com/Nimblesite/Deslop/issues/433) and all three [#439](https://github.com/Nimblesite/Deslop/issues/439) skips are gone.

Both issues are fixed by that one commit: the production code, the corpus manifests, the regression assertions, and this review all land together at `4177edb`.

## Regression results

| Finding | Before working-tree fixes | Current result | Assessment |
|---|---|---|---|
| Persisted-signature equivalence ([#433](https://github.com/Nimblesite/Deslop/issues/433)) | Mixed persisted-signature analysis changed `agreement` and `rename_consistency` relative to a cold scan of equivalent source. | **Pass.** `the_lsh_only_pair_keeps_its_verdict_across_the_persistence_matrix` runs unignored and passes. | **Fixed at [`4177edb`](https://github.com/Nimblesite/Deslop/commit/4177edbb514f0c7810c1f507a6b67e24e1926879).** Content evidence, token signatures, and persisted signatures now share one language-aware boilerplate exclusion, removing the cold-versus-mixed denominator mismatch. |
| Curated Type-2 extent ([#439](https://github.com/Nimblesite/Deslop/issues/439)) | The recall judge accepted an uncurated extent, a small fragment, or a boilerplate family spanning the same paths. | **Pass.** All nine curated Type-2 judge tests pass, including the three former red assertions. | **Fixed at [`4177edb`](https://github.com/Nimblesite/Deslop/commit/4177edbb514f0c7810c1f507a6b67e24e1926879).** Curated entries now require a positive `min_nodes` floor judged against `canonical_node_count`, and the manifest contract refuses an uncurated entry before any scan runs. |
| Embedding / fusion ([#369](https://github.com/Nimblesite/Deslop/issues/369)) | The routing pin expects `same_behavior`; the LSP refresh expects a second embedding-supported cluster. | Still open. Both pins remain red. | **Excepted — embedding.** Carved out of the regression verdict; not counted as a regression from `f92300e5`. |
| Operator drift ([#432](https://github.com/Nimblesite/Deslop/issues/432)) | Operator-only drift could reach `nearly_identical` and outrank a byte-identical control. | Still open. Direct baseline comparison previously reproduced it on both engines. | **Pre-existing, not a regression from `f92300e5`.** |

## Validation results

| Check | Result | Evidence |
|---|---:|---|
| `make ci` | **Pass** | Formatting, clippy, build, generated-wire checks, contracts, self-scan, Rust coverage collection, extension-host tests, browser tests, and deployment checks exited 0. |
| Default Rust tests | **Pass with 16 curated skips** | 1,239 passed, 0 failed. Remaining skips: 11 large corpus executions ([#422](https://github.com/Nimblesite/Deslop/issues/422)), one corpus-scope curation assertion ([#426](https://github.com/Nimblesite/Deslop/issues/426)), two embedding assertions ([#369](https://github.com/Nimblesite/Deslop/issues/369)), and two operator-drift assertions ([#432](https://github.com/Nimblesite/Deslop/issues/432)). |
| Persisted-signature regression | **Pass** | 1/1 focused assertion passed, unignored. |
| Type-2 extent judge | **Pass** | 9/9 focused assertions passed; the three former #439 skips run by default. |
| VS Code extension host | **Pass** | 471 tests passed. |
| Webview / standalone HTML | **Pass** | 7 webview browser tests and 3 standalone-report browser tests passed. |
| Tokio corpus | **Pass** | 758 files, 168,480 LOC, 2,186 clusters, 31.0% duplication, 670 MB peak RSS. |
| Nest corpus | **Pass** | 1,726 files, 115,848 LOC, 1,006 clusters, 30.5% duplication, 589 MB peak RSS. |
| Nest determinism | **Pass** | Both runs produced 1,006 clusters and 30.4830% duplication. |

The corpus results are important for #439: the new extent floors reject the synthetic false-green witnesses while the pinned real Tokio and Nest duplicates still satisfy the gate.

## Baseline comparison retained from the first audit

Before these working-tree fixes, the baseline and committed candidate binaries both completed 400 paired fixture cases at `--min-nodes 4` and `30` without crashing. Of those cases, 300 produced the same cluster, duplicated-LOC, and hidden-group counts; 100 produced a changed report. The default black-box suite adjudicates the intended fixture changes.

The first audit also measured the operator-drift fixture directly on both binaries. That measurement is why #432 is classified as pre-existing rather than silently treated as fixed or as a new regression.

Those comparative fixture counts were not rerun after the working-tree changes. The changes were instead checked by their exact regression assertions, the complete release gate, and the pinned real-repository slice.

## Validation limits

- The full eleven-repository corpus suite was not run. The scheduled Tokio/Nest slice and Nest determinism were run.
- Linux, Windows, and `darwin-x64` VSIX artifacts were not built on this host.
- Tagged release assets, Marketplace/Open VSX publication, Homebrew, Scoop, and the post-tag Action download/install path were not exercised.

## Release decision

Within the stated scope, the working-tree changes fix the regressions identified by this audit and introduce no serious regression detected by the full release gate, focused black-box assertions, or scheduled corpus validation. The only remaining red accuracy assertions are the two embedding pins tracked by [#369](https://github.com/Nimblesite/Deslop/issues/369), excepted from this verdict.

The release may proceed on that scope. Keep #432 open as pre-existing accuracy debt and #369 open as the excepted embedding work.
