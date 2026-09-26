# Accuracy and performance progress

Status: **paused on 2026-09-24**. The working branch is `taxonomy2` at `f042657b90ff8bc3b50b39b9775884669247e174`; the changes described below are in the uncommitted worktree. There is no pull request yet.

## What was done

- Ran the 13-repository, 9-language corpus against the judged clone registers. The scorer now treats the old `bucket` and current `kind` taxonomies consistently. An old `structural_only` finding is informational, not a clone; absent and unknown taxonomy labels cannot satisfy a judged pair.
- The interim optimized build found all **295/295** judged pairs, compared with **278/295** for both `f92300e5e1004ef6c53a94174a0d7e842232ec80` and the published `v0.32.0` binary. Its pair-level score was 100%, with 0 false negatives and 0 false positives; each baseline had 13 false negatives and 4 false positives.
- Restored the Polly copied setup tail. The focused C# suite passes **7/7** and asserts a separate exact Type I copy and broader edited Type III finding, with no clone occurrence crossing authored method boundaries.
- Fixed the large-tree overlap fallback's double credit for nested empty byte spans. A test reproduced 9 credited nodes where exact alignment found 4; the corrected test and existing core controls pass.
- Added an exact-overlap shortcut for small trees when an ordered upper bound equals the conservative lower bound. The core suite passed **293/293**, and formatting plus strict core Clippy passed after that change.
- Improved rank-time reuse of rescue alignments and bounded memo rotation. The focused tests passed **3/3** and **12/12**, with strict core Clippy passing.
- The wider branch also contains the Deslop screen taxonomy and comparison UI work, corpus scoring changes, and related specifications and tests.

## Measurements and limits

These figures come from the mechanically generated, local scorecards: [comparison to `f92300e5`](../../.corpus/measurements/f923-compare/SCORE.md) and [comparison to `v0.32.0`](../../.corpus/measurements/v032-compare/SCORE.md). Both use the same pinned corpus and judged registers. The measured optimized binary predates the latest source changes, so these are interim results.

| Measure | `v0.32.0` | Interim optimized build | Result |
|---|---:|---:|---:|
| Judged pairs correct | 278/295 | 295/295 | Improved |
| Wall time | 227.22 s | 167.56 s | 59.66 s faster |
| CPU time | 220.04 s | 341.25 s | 121.21 s slower |
| Peak memory | 2,420 MB | 2,178 MB | 242 MB lower |
| Matched clone-range coverage | 98.6% | 93.3% | Regressed |

The pair score alone overstated the result. In Axios, a byte-identical 31-line pair reported fully at `f92300e5` is split into short fragments by the interim build. A new black-box assertion reproduces this extent loss and currently fails because the full exact clone is missing. The code changes since the measured build have not been rescanned, so neither the accuracy nor CPU result describes the final worktree.

## What remains

1. Fix the Axios extent false negative while preserving the Polly method-boundary guard and the separate exact and edited findings. The failing assertion is in [adjacent_calls.rs](../../crates/deslop/tests/clone_extent_recovery/adjacent_calls.rs).
2. Rerun the complete pinned corpus against both `f92300e5e1004ef6c53a94174a0d7e842232ec80` and the official `v0.32.0` binary. Verify pair verdicts **and** matched range coverage, then reduce CPU time below the `v0.32.0` measurement.
3. Run `make ci`, review the complete branch diff, create the PR, monitor all required checks to green, and merge it.
4. Follow up on open issues [#557](https://github.com/Nimblesite/Deslop/issues/557), [#558](https://github.com/Nimblesite/Deslop/issues/558), [#559](https://github.com/Nimblesite/Deslop/issues/559), and [#560](https://github.com/Nimblesite/Deslop/issues/560). They cover the C# overbroad finding, stale MCP/editor wire mismatch, Axios extent loss, and empty-span overlap credit.

The goal remains unproven: the interim build was faster in wall time and used less memory, but it used more CPU, and its matched range coverage regressed. No PR was created and `make ci` has not been run on the current worktree.
