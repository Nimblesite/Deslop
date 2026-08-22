# GitHub Actions failure report

## Scope

- Repository: `Nimblesite/Deslop`
- Branch: `worktree-fused-score-followups`
- Pull request: [#424](https://github.com/Nimblesite/Deslop/pull/424)
- Window: 2026-08-21 04:46:39 UTC through 2026-08-22 04:46:39 UTC (the preceding 24 hours)
- Evidence: GitHub Actions run, job, step, timing, and failed-step logs only
- No local tests, builds, lint commands, or CI commands were run for this report.

## Executive summary

There were **54 workflow runs** in the window: **18 CI failures**, **18 successful CodeQL runs**, and **18 successful action self-tests**. Every revision failed the main CI workflow. The failures are therefore not a general GitHub Actions outage: they are concentrated in the branch's main CI path.

The branch shows a clear whack-a-mole sequence of correctness regressions followed by a CI redesign that made feedback dramatically slower:

1. Early revisions failed quickly on lint, website validation, Windows TCP behavior, cache integrity, AST goldens, and corpus-manifest contracts.
2. Later revisions repeatedly failed Rust accuracy/determinism tests after several minutes.
3. The newest revisions moved compilation and coverage onto longer critical paths. Failures now arrive after 15–46 minutes, and the newest run timed out all four Rust shards together.

The highest-priority problem is not CodeQL or the action. It is uncontrolled iteration on accuracy-sensitive detector code while CI infrastructure is simultaneously being reworked. Stop changing both surfaces at once.

## Failures in the last 24 hours

| UTC | SHA | Run | Failed/cancelled work | Time to workflow completion |
|---|---|---:|---|---:|
| Aug 21 12:32 | `a1b7c203` | [32482356802](https://github.com/Nimblesite/Deslop/actions/runs/32482356802) | `CI / Lint`; website typed-report assertion; Windows `mcp_tools_work_over_tcp_transport` | 5m14s |
| Aug 21 12:58 | `1c4ed80c` | [32484475612](https://github.com/Nimblesite/Deslop/actions/runs/32484475612) | Rust cache-integrity tests; website broken OG-image check; same Windows TCP test | 4m42s |
| Aug 21 13:15 | `a3ccbb35` | [32485929690](https://github.com/Nimblesite/Deslop/actions/runs/32485929690) | Ten AST golden tests; same Windows TCP test | 4m39s |
| Aug 21 13:29 | `73327195` | [32487116646](https://github.com/Nimblesite/Deslop/actions/runs/32487116646) | Corpus manifest contract; same Windows TCP test | 4m47s |
| Aug 21 14:31 | `40da4b93` | [32492590443](https://github.com/Nimblesite/Deslop/actions/runs/32492590443) | `CI / Lint` | 1m39s |
| Aug 21 14:36 | `85c0c98d` | [32493040556](https://github.com/Nimblesite/Deslop/actions/runs/32493040556) | Rust `Test` — corpus manifest contract | 4m30s |
| Aug 21 15:06 | `97cb15bb` | [32495763757](https://github.com/Nimblesite/Deslop/actions/runs/32495763757) | Rust `Test` | 4m42s |
| Aug 21 15:45 | `c699763d` | [32499360961](https://github.com/Nimblesite/Deslop/actions/runs/32499360961) | Rust `Test` | 4m25s |
| Aug 21 16:20 | `33756824` | [32502447847](https://github.com/Nimblesite/Deslop/actions/runs/32502447847) | Rust `Test` | 6m08s |
| Aug 21 17:01 | `4def78fe` | [32506012679](https://github.com/Nimblesite/Deslop/actions/runs/32506012679) | Rust `Test` | 6m30s |
| Aug 21 18:44 | `27d7d095` | [32514899505](https://github.com/Nimblesite/Deslop/actions/runs/32514899505) | Rust `Test` | 6m41s |
| Aug 21 20:06 | `11c483bb` | [32521899507](https://github.com/Nimblesite/Deslop/actions/runs/32521899507) | Rust `Test` | 8m41s |
| Aug 21 21:47 | `9595c39e` | [32530130800](https://github.com/Nimblesite/Deslop/actions/runs/32530130800) | Rust `Test` | 8m33s |
| Aug 21 22:20 | `22541c27` | [32532572853](https://github.com/Nimblesite/Deslop/actions/runs/32532572853) | Rust `Test` | 8m31s |
| Aug 22 01:01 | `74a24b4c` | [32542178321](https://github.com/Nimblesite/Deslop/actions/runs/32542178321) | Incremental multilang cold/warm determinism goldens | 28m54s |
| Aug 22 02:11 | `021cb37d` | [32545570127](https://github.com/Nimblesite/Deslop/actions/runs/32545570127) | Coverage test `typescript_near_miss_produces_cross_file_structural_cluster`; build cancelled at its old 25-minute cap | 25m20s |
| Aug 22 03:14 | `bb5e483e` | [32548446084](https://github.com/Nimblesite/Deslop/actions/runs/32548446084) | Coverage failed; build cancelled at 25m16s during duplication gate | 25m33s |
| Aug 22 03:42 | `e22493d2` | [32549706011](https://github.com/Nimblesite/Deslop/actions/runs/32549706011) | Coverage failed; all four Rust shards cancelled at the 20-minute job cap; final `CI` gate failed because the shard matrix did not succeed | 46m04s |

### Confirmed failure signatures

- **Windows TCP accuracy defect, four consecutive revisions:** `mcp_tools_work_over_tcp_transport` expected live LSP clusters but `find-similar` returned `clusters: []` and `total_occurrences: 0`. Runs 32482356802, 32484475612, 32485929690, and 32487116646.
- **Cache/report integrity defect:** `a_tampered_signature_payload_is_a_miss_that_self_heals`, `corrupt_blob_shapes_always_miss_and_never_crash`, and `a_blob_swapped_to_another_files_address_serves_neither` disagreed with the authored clone's user-visible id, size, canonical-node count, or category. Run 32484475612.
- **Broad AST-output drift:** ten committed AST goldens changed together across C#, Dart, Go, JavaScript, JSX, PHP, Python, Rust, TypeScript, and TSX. Run 32485929690. This points to a shared AST dump/normalization change, not ten unrelated language defects.
- **Corpus validation defect:** `every_manifest_curates_a_non_vacuous_scan_scope` reported that the Django manifest lacked a positive `expect_files_min`, allowing a zero-file scan to pass cluster assertions. Runs 32487116646 and 32493040556.
- **Incremental determinism defect:** `fully_warm_multilang_run_reproduces_the_committed_golden` and `cold_multilang_report_matches_committed_golden_byte_for_byte` disagreed in top-level `clusters`. Run 32542178321.
- **TypeScript structural-cluster defect:** `typescript_near_miss_produces_cross_file_structural_cluster` failed inside coverage. Run 32545570127.
- **Website defects:** the first website run failed an assertion (`0` was not at least `4`); the next found `https://deslop.live/assets/img/blog/towards-100-percent-accuracy-og.png` broken. Runs 32482356802 and 32484475612.
- **Lint failures:** runs 32482356802 and 32492590443 failed before tests could provide useful feedback.
- **CI orchestration/timeouts:** runs 32545570127 and 32548446084 cancelled the build near 25 minutes. After the workflow redesign, run 32549706011 lasted 46 minutes and cancelled every Rust shard at about 20 minutes. The final `CI` job is only a downstream symptom.

## Longest jobs and steps

| Run | Job/step | Duration | Outcome |
|---:|---|---:|---|
| [32549706011](https://github.com/Nimblesite/Deslop/actions/runs/32549706011) | Whole CI workflow | 46m04s | Failure |
| [32548446084](https://github.com/Nimblesite/Deslop/actions/runs/32548446084) | `Build release` job | 25m16s | Cancelled |
| [32545570127](https://github.com/Nimblesite/Deslop/actions/runs/32545570127) | `Build release` job | 25m08s | Cancelled |
| [32542178321](https://github.com/Nimblesite/Deslop/actions/runs/32542178321) | Rust `Test` step | 22m59s | Failure |
| [32545570127](https://github.com/Nimblesite/Deslop/actions/runs/32545570127) | `Coverage` step | 20m50s | Failure |
| [32549706011](https://github.com/Nimblesite/Deslop/actions/runs/32549706011) | Rust shard `Test` steps | 19m47s–19m53s each | Cancelled |
| [32548446084](https://github.com/Nimblesite/Deslop/actions/runs/32548446084) | `Coverage` step | 18m13s | Failure |
| [32549706011](https://github.com/Nimblesite/Deslop/actions/runs/32549706011) | `Coverage` step | 15m11s | Failure |

The latest run's four shard cancellations are one cascading CI failure, not four independent product failures. With matrix `fail-fast: true`, the first shard reaching the 20-minute limit causes the rest to be cancelled at essentially the same time.

## Pattern and likely causes

### 1. The branch is fixing symptoms one commit at a time without stabilizing the accuracy contract

The failure signature moves from cache integrity to cross-language AST drift, corpus validation, incremental determinism, and TypeScript structural matching. These all touch the detector's accuracy surface. The changing failures show that each push is getting past one gate only to expose or introduce another.

### 2. Shared pipeline logic is causing broad blast radii

Ten language goldens failed together, the Windows TCP result returned an empty but nominally successful response, and cold/warm reports diverged in `clusters`. Those are shared normalization, caching, transport synchronization, or ranking problems. Treating each fixture independently will hide the common defect.

### 3. CI was redesigned while the code was already red

The early fail-fast pipeline produced actionable failures in roughly 2–8 minutes. The newest architecture adds a release build, cached test compilation, four shards, separate coverage, and a synthetic final gate. It now takes up to 46 minutes to report failure. The redesign has not demonstrated that downstream shards restore usable test artifacts, and the current 20-minute shard budget is below observed runtime.

### 4. Coverage is duplicating expensive test execution

The coverage job executes tests independently while the Rust shard matrix also executes the suite. This creates two expensive correctness paths and makes a single test defect consume 15–21 minutes in coverage before the main suite has resolved.

### 5. Auxiliary workflows are healthy

All 36 CodeQL/action-self-test runs passed. Do not spend time changing those workflows or blaming runner-wide instability. The evidence points to the branch's Rust/site correctness and main CI orchestration.

## Actionable recovery plan

### P0 — Stop the churn

1. **Freeze feature changes and CI refactors on this branch.** Only accept changes that close a currently observed failure signature.
2. **Use the newest failing SHA as the sole baseline.** Do not chase historical failures that are no longer present unless the current code still violates the same contract.
3. **Do not re-bless AST or report goldens as a shortcut.** Ten-language drift and cold/warm cluster divergence require a root-cause explanation and reviewed semantic diff first.

### P1 — Restore correctness in dependency order

1. **Fix the TypeScript near-miss/coverage failure first.** It is a confirmed current accuracy regression and blocks coverage before a report is emitted.
2. **Then resolve the shard timeout with evidence from the restore step.** Expose `actions/cache/restore`'s `cache-hit` output, print the exact key, and fail immediately if the commit-specific key misses. A shard must not spend 20 minutes silently rebuilding.
3. **Measure the shard manifest before execution.** Print the number of test binaries/tests assigned to each shard and the estimated/previous runtime. Rebalance by measured runtime, not simple test count.
4. **Keep the cold/warm incremental golden failure as a hard blocker.** The only permitted report difference is `cache_stats`; cluster divergence is an accuracy defect.
5. **Verify the Windows TCP fix is causal.** The repeated empty-cluster response indicates the server was reachable but analysis state was not ready or not transferred. The contract should wait on an explicit readiness/state acknowledgement, never timing or sleeps.

### P1 — Make CI fail fast again

1. **Put cheap deterministic gates first:** grammar pins, format, lint, manifest validation, and targeted contract checks must finish before release compilation, full coverage, VSIX, or JetBrains work.
2. **Do not solve this by increasing every timeout.** The latest shards need cache-hit proof and runtime balancing. Raise a timeout only after proving the job is doing intended work rather than recompiling.
3. **Stop the final `CI` gate from obscuring the cause.** Its `Every Rust shard passed` error is a roll-up, not a diagnosis. Preserve the failed shard name and conclusion in the summary/output.
4. **Avoid simultaneous CI architecture and detector changes.** First recover green on a simple, known pipeline. Optimize only after collecting several green-run timings.

### P2 — Reduce wasted runner time

1. **Remove redundant compilation between build, coverage, and shards.** Coverage requires instrumented artifacts, but ordinary shards should consume an exact, verified artifact/cache produced by the build job.
2. **Consider artifact transfer for compiled test binaries if the cache is unreliable.** Cache restoration is an optimization and may fall back; an artifact is an explicit handoff. Whichever mechanism is used must fail closed on a missing exact revision.
3. **Run expensive VSIX/JetBrains gates only after the cheap correctness gate succeeds**, unless parallel cost is deliberately accepted for latency.
4. **Emit a machine-readable timing summary** for every job and named step so regressions are obvious without manually mining logs.

## Exit criteria before normal development resumes

- The current failing test is identified by name and fixed at root cause.
- AST/report goldens are unchanged unless an intended semantic change is documented and reviewed.
- Cold and fully warm reports have identical clusters, ids, ranks, spans, and metrics.
- Windows TCP returns live clusters through an explicit readiness contract.
- Every Rust shard proves an exact cache/artifact restore and completes comfortably below its timeout.
- At least three consecutive CI runs on unchanged workflow architecture complete successfully, with no timeout increases used to mask recompilation or imbalance.

