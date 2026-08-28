# Release regression audit

Audit date: **2026-08-28**

Baseline: [`f92300e5e1004ef6c53a94174a0d7e842232ec80`](https://github.com/Nimblesite/Deslop/commit/f92300e5e1004ef6c53a94174a0d7e842232ec80) (`v0.32.0`)

Candidate: [`7d6d6996d58a5b49e34d86f9a5eb23a81e48c2cf`](https://github.com/Nimblesite/Deslop/commit/7d6d6996d58a5b49e34d86f9a5eb23a81e48c2cf)

## Verdict

**Not ready. The audit does not clear the candidate as free of serious regressions.**

The ordinary release gate passes, the candidate does not crash in the comparative scans, the host VSIX validates, and the scheduled Tokio/Nest corpus slice passes. That is necessary evidence, but it is not an accuracy sign-off: all **eight curated accuracy assertions fail** when explicitly run. One failure is on the post-baseline persisted-signature path, another loses expected LSP embedding evidence, and three prove that the corpus Type-2 recall judge can pass without validating the curated duplicate's extent.

The operator-drift false positive is serious but is **not a new regression from this baseline**: at `--min-nodes 30`, both binaries publish the same `ledger_credit.py` / `ledger_debit.py` pair, rank it above the identical control, and bill 38 duplicated lines. At `--min-nodes 4`, the baseline is worse: it publishes additional operator-only families as `nearly_identical`, while the candidate demotes the extra families to zero-weight `structural_only`. This distinction matters, but it does not excuse the post-baseline failures or the blind recall gate.

## What changed

GitHub reports the candidate is 15 commits ahead of the baseline. Clean source archives contain **1,030 changed paths**: 477 added, 41 deleted, and 512 modified. The changed surface includes 250 production Rust files and 30 non-test VS Code source files. This is a substantive engine, persistence, reporting, deployment, and UI change set—not a documentation-only delta.

## Validation results

| Check | Result | Evidence |
|---|---:|---|
| `make ci` | Pass | Formatting, clippy, build, contracts, self-scan, Rust tests, extension-host tests, browser tests, and coverage completed with exit 0. |
| Default Rust tests | Pass with skips | 1,235 passed; 20 curated ignores remain: 8 accuracy and 12 corpus infrastructure/curation. |
| Rust coverage | Pass | `deslop-core` 95.8%, `deslop` 97.2%, `deslop-lsp` 95.8%, `deslop-mcp` 97.0%; workspace 94.6% (23,324/24,646 lines). |
| VS Code extension host | Pass | 471 tests passed. |
| Webview / standalone HTML | Pass | 7 webview browser tests and 3 standalone-report browser tests passed. |
| Host VSIX | Pass | `darwin-arm64`, 12.9 MB, 16 entries; all three bundled binaries, manifest, schema, allow-list, and no-stub checks passed. |
| Scheduled corpus slice | Pass | Tokio and Nest passed their curated gates; two Nest runs were identical. |
| Explicit accuracy skips | **Fail** | 0/8 passed when the ignored assertions were run directly. |

`make ci` deliberately removed `deslop`, `deslop-lsp`, and `deslop-mcp` from `PATH` before validating the bundled-binary workflow. The host PATH is therefore left clear of those binaries.

## Baseline-versus-candidate measurements

Both release binaries scanned the same inputs with embeddings off, no incremental cache, and the same node floor. Counts are measurements, not automatic quality judgments: an increase can be recovered recall or manufactured false positives, and a decrease can be precision work or a false negative.

### Checked-in fixture matrix

The two binaries each scanned 200 fixture directories at `--min-nodes 4` and `30`: **400 paired cases / 800 executions**, all with exit 0.

- 300 paired cases produced the same cluster, duplicated-LOC, and hidden-group counts.
- 100 changed: candidate duplicated LOC was lower in 30, higher in 47, and unchanged with a reshaped report in 23.
- The changed cases include asserted precision improvements and asserted recall improvements; the green default suite covers those intended outcomes.
- The matrix also reproduces the pre-existing operator-drift defect described in the verdict.

### Same candidate source tree, empty config

| Engine | Files | Clusters | Duplicated LOC | Duplication | Hidden groups |
|---|---:|---:|---:|---:|---:|
| Baseline `f92300e5` | 1,339 | 1,053 | 14,687 | 8.44% | 599 |
| Candidate `7d6d6996` | 1,339 | 1,429 | 24,487 | 14.07% | 1,043 |

The 5.63-point increase is large enough that the accuracy assertions must adjudicate it. Because those assertions are not all green, the increase cannot be used as release evidence on its own.

### Pinned real repositories

| Corpus | Baseline | Candidate | Candidate gate |
|---|---:|---:|---:|
| Tokio `tokio-1.49.0` | 1,779 clusters, 19.99% | 2,186 clusters, 30.98% | Pass; 758 files, 168,480 LOC, 672 MB peak RSS |
| Nest `v11.1.28` | 1,329 clusters, 32.68% | 1,005 clusters, 30.44% | Pass; 1,726 files, 115,848 LOC, 597 MB peak RSS |
| Nest determinism | Not rerun for baseline | 1,005 clusters and 30.4416% in both runs | Pass |

These results stay inside the curated cluster-count bands and find the curated Type-2 pairs. However, the Type-2 judge's three red extent tests mean that “found” is not yet a trustworthy proof that the intended full duplicate was found rather than a smaller fragment over the same files.

## Release-blocking accuracy evidence

| Issue | Red assertions | Measured failure | Regression assessment |
|---|---:|---|---|
| [#433](https://github.com/Nimblesite/Deslop/issues/433) | 1 | A mixed persisted-signature pass changes `agreement` from 0.358974 to 0.333333 and `rename_consistency` from 0.583333 to 0.560784 versus a cold pass over equivalent source. | **In scope.** Persisted parse/signature work changed after the baseline; identical corpus state must not produce different evidence. |
| [#369](https://github.com/Nimblesite/Deslop/issues/369) | 2 | The routing pin gets `nearly_identical` instead of `same_behavior`; the LSP refresh expects a second embedding-supported cluster but receives none, while the surviving cluster has `embedding_cos = 0.0`. | **Not cleared against the baseline.** The changed fusion/embedding paths cannot be signed off while their own assertions are red. |
| [#439](https://github.com/Nimblesite/Deslop/issues/439) | 3 | The curated Type-2 judge accepts an empty extent, a far-too-small fragment, and a boilerplate family spanning the same paths. | **Blocks proof.** The corpus gate can return a false green, so its pass cannot close recall risk. |
| [#432](https://github.com/Nimblesite/Deslop/issues/432) | 2 | Operator-only drift reaches `nearly_identical`, ranks above a byte-identical control, and is billed as duplication. | Serious pre-existing defect, directly reproduced in both baseline and candidate; not counted as a new regression. |

The remaining 12 ignores are 11 oversized corpus executions tracked by [#422](https://github.com/Nimblesite/Deslop/issues/422) and one incomplete corpus-scope curation assertion tracked by [#426](https://github.com/Nimblesite/Deslop/issues/426).

## Validation limits

- The full eleven-repository corpus suite was not run. Only the scheduled Tokio/Nest slice and Nest determinism were run.
- Linux, Windows, and `darwin-x64` VSIX artifacts were not built on this host.
- A tagged release asset, Marketplace/Open VSX publication, Homebrew package, Scoop package, and the post-tag Action download/install path were not exercised.
- The comparative repository scan used an empty config to hold both engines to the same input policy; the normal candidate self-scan separately passed with the repository's `.deslop.toml`.

## Release decision

Do not describe this candidate as having no serious regressions. Release requires, at minimum, green resolutions for the post-baseline persistence and fusion/embedding accuracy pins, plus a Type-2 corpus recall judge that proves the curated extent instead of only matching file paths. Re-run this audit from `f92300e5e1004ef6c53a94174a0d7e842232ec80` after those assertions return to the default green gate.
