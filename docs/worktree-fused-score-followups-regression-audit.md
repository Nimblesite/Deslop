# Regression audit: `worktree-fused-score-followups`

Date: 2026-08-17  
Scope: current filesystem snapshot in `.claude/worktrees/fused-score-followups`  
Comparison: adjacent main checkout at `/Users/christianfindlay/Documents/Code/Deslop`

This document replaces the previous audit. It describes the repaired worktree as it exists now; superseded failures and stale repair instructions have been removed.

## Verdict

**Merge-ready.** Every blocking regression is fixed and pinned by a test that was watched failing for the defect's own reason. The three remaining items are P2/P3 verification gaps that this branch does not introduce and cannot regress; each is now a tracked issue.

| Finding | Disposition |
|---|---|
| RA-01 read-after refusal | **Fixed.** The Type-1 merge branch runs the same dataflow rules the extract tier does, so the read-after reason reaches the plan instead of being replaced by routing advice. |
| RA-02 `type2_recall` | **Fixed.** The population heuristic is renamed `type2_gate_liveness` and no longer claims recall. `type2_recall` is now a curated check over hand-verified `must_find_type2` pairs, and the manifests carry real entries in two languages. |
| RA-03 stale UI across restart | **Fixed.** A client session epoch resets the store on restart and makes in-flight refreshes from the dead session inert. |
| RA-04 installer test routing | **Fixed.** The `site` job runs the installer contract, and a routing contract proves an installer-page-only change schedules it. |
| RA-05 live-bubble deadline | **Fixed.** Budget expiry is recorded on the dispatch and rejects late completions, proven with an injected clock against a server that ignores cancellation. |
| RA-09 500-line rule | **Fixed.** The three VSIX suites are split, and the corpus-confidence suite that grew past the cap during this work is split with it. |
| RA-06 Win32 path semantics | Open — [#393](https://github.com/Nimblesite/Deslop/issues/393). |
| RA-07 Dependabot event matrix | Open — [#394](https://github.com/Nimblesite/Deslop/issues/394). |
| RA-08 plan ledger contradiction | Open — [#395](https://github.com/Nimblesite/Deslop/issues/395). |

Disposition of the original eleven findings:

- Nine have their reported runtime or contract defect fixed.
- REG-05 is fixed: reconnect now resets the store and invalidates outstanding refreshes.
- REG-09's implementation and Linux gate are fixed; native Win32 behavior is still not exercised (#393).

The previous format, Clippy, #339 false-red, installer fail-open, VSCE compatibility, Dependabot actor-skip, bounded-max admission, cross-language documentation, signal-strip, ledger-cap, and MCP contract findings no longer reproduce.

## Original finding disposition

| Finding | Current disposition |
|---|---|
| REG-01 | **Fixed.** `engines.vscode` and `@types/vscode` are aligned on 1.101; `npx --no-install vsce ls` passes. |
| REG-02 | **Fixed behavior.** Dependabot security PRs are no longer actor-skipped by CI, dependency review, CodeQL, or Action self-test. Anti-regression event-matrix coverage is still missing; see RA-07. |
| REG-03 | **Fixed.** Valid synthetic sibling windows resolve through the language-aware token path; the corrected unit and exact-range F# E2E tests pass. A separate Rust integration regression in the repaired worktree is RA-01. |
| REG-04 | **Fixed.** The heuristic is renamed `type2_gate_liveness` and requires gate-vouched evidence; `type2_recall` now reads curated pairs. See RA-02. |
| REG-05 | **Fixed.** Real async tests cover stale success, stale failure, ABA, and disposal; restart now resets the store and voids in-flight refreshes. See RA-03. |
| REG-06 | **Fixed.** Retraction history is capped at 256, pruned oldest-first, and covered by a 2,000-delta test. |
| REG-07 | **Fixed.** A direct pair-layer test distinguishes bounded max from sum using `0.44 / 0.42 / 0.0`, with positive controls at `0.85` and `0.86`. |
| REG-08 | **Fixed.** The signal strip, exact-proof glyph rule, implementation, README, and VSIX spec agree. |
| REG-09 | **Partial verification.** Platform injection, POSIX-only socket cap, and `make lint` wiring are fixed; native Win32 path behavior is not run in Windows CI. Tracked as #393. See RA-06. |
| REG-10 | **Fixed.** The 0.10 exception is scoped to cross-language candidates without a structural anchor in code, authoritative documentation, public EN/ZH pages, and tests. |
| REG-11 | **Fixed.** MCP ordering is specified as final report `weight` descending, including the data-category exception, and the plan points at the shipped contract. |

## Blocking regressions

### RA-01 — RESOLVED — the repaired worktree regresses a merge-safety integration test

Locations:

- `crates/deslop-core/src/pipeline/signatures.rs`
- `crates/deslop-core/tests/refactor_merge_refusals.rs:81-86`
- `crates/deslop-core/tests/common/merge.rs:141-160`
- `crates/deslop/tests/fixtures/csharp-merge-readafter/Prefix.cs`

The focused test fails deterministically in this worktree:

```text
cargo test --release -p deslop-core --features live,test-support \
  --test refactor_merge_refusals declared_inside_read_after_refuses -- --nocapture

FAILED
Error: csharp-merge-readafter: some refusal names `read after`
```

The same command passes in the adjacent comparison checkout. The failure also reproduces in the debug profile, so it is not release optimisation or timing.

The report shape changed in the repaired worktree:

| Snapshot | Reported candidate ranges |
|---|---|
| Comparison | one `nearly_identical`: `85..275` / `374..563` |
| Repaired worktree | one `identical`: `117..275` / `405..563`; one `nearly_identical`: `85..212` / `374..500` |

Every resulting plan still refuses, but none reaches or reports the intended read-after safety reason. The integration guard therefore no longer proves that a local declared inside the selected span and read afterwards is rejected by the correct safety gate. This also makes the claim that the Rust targets are green false.

Action:

1. Make the failing helper print every actual refusal reason so the earlier gate is visible.
2. Trace which pipeline change fragments the fixture's safety-relevant candidate; start at the changed language-aware signature path.
3. Restore a candidate that exercises the read-after gate, or correct the planner if a valid candidate bypasses it. Do not weaken the assertion to accept an unrelated refusal.
4. Re-run the focused test in both profiles and the exact `make test`/CI Rust command.

### RA-02 — RESOLVED — `type2_recall` still produces both false greens and false reds

Locations:

- `crates/deslop-test-support/src/corpus_confidence.rs:125-172`
- `crates/deslop-test-support/src/corpus_confidence/tests.rs:280-307`
- `crates/deslop/tests/corpus_repos.rs:186-208`
- `crates/deslop-core/src/report_render.rs:277-300,419-442`
- `crates/deslop-core/src/buckets.rs:270-288,313-319`
- `corpus/tokio.json:15-16`
- `corpus/nest.json:13-14`
- `docs/specs/corpus.md:19-23`

`check_type2_recall` passes whenever any visible cluster is `nearly_identical`. Its own test explicitly establishes that one such cluster clears twenty demotions. The check does not identify an expected Type-2 pair and does not read curated recall expectations.

`nearly_identical` is not proof that the content gate vouched for a rename. The C# LSH Type-3 path can produce that bucket with `structural = 0`, `embedding = 0`, and token Jaccard `0.90..0.949`; below the saturating-token floor of 0.95, `content_gated_signals` returns without gating it.

Consequences:

- all genuine Type-2 renames may be demoted while one unrelated C# near miss keeps the gate green;
- a corpus with many legitimate demotions and no expected Type-2 pair can fail;
- Tokio and Nest still have empty `must_find` arrays whose manifests explicitly say accuracy is unasserted, yet this heuristic runs on them and is named as recall.

Action:

1. Add a manifest schema for hand-verified Type-2 expectations: stable paths plus markers/ranges and the required visible/actionable bucket.
2. Check those exact pairs for presence, visibility, and verdict.
3. Rename the existing population heuristic as telemetry; do not use it as an accuracy gate or call it recall.
4. Add counterexamples for unrelated-near rescue, legitimate demotions without ground truth, and one curated actionable Type-2 pair.

### RA-03 — RESOLVED — generation rollback protection preserves stale UI across an LSP restart

Locations:

- `clients/vscode/src/reportStore.ts:165-195`
- `clients/vscode/src/extension.ts:247-256,364-379`
- `clients/vscode/src/notifications.ts:75-106`
- `crates/deslop-core/src/live/session.rs:303-329`
- `docs/specs/vsix.md:251-260`

`ReportStore.setSnapshot` now rejects every snapshot below the current generation. That correctly stops an out-of-order completion within one server session, but generations are not global: every new `AnalysisSession` starts at 1.

The pinned `vscode-languageclient` automatically restarts a crashed server by default. The extension installs no restart/reset handler and only seeds the store during initial activation. A store at generation 100 therefore rejects the restarted server's generation-1 snapshot and retains the old report:

```text
old store: generation 100, cluster old-server
setSnapshot(new-server report, 1)
=> accepted=false, generation=100, cluster=old-server
```

The stale report and every derived surface can remain visible until the new process reaches generation 100. This directly contradicts the VSIX contract that reconnect resets the store and rerenders every surface from empty.

Action:

1. Introduce a client connection/session epoch separate from the server generation.
2. On restart, atomically clear the report, generation, retractions, pending refreshes, and lifecycle state; bump the client revision so outstanding probes become inert.
3. Retain the lower-generation rejection only within one connection epoch.
4. Add a deterministic test: old session at generation 100, restart/reset, new session generation 1 accepted, and all old surfaces empty before the new report arrives.

## CI and verification gaps

### RA-04 — RESOLVED — the installer security test is skipped on installer-only documentation changes

Locations:

- `Makefile:140-150`
- `.github/workflows/ci.yml:58-84,398-430`
- `scripts/installer-snippet.test.mjs`
- `site/src/docs/index.md:63-96`
- `site/src/zh/docs/index.md:64-97`

The installer itself is now fail-closed, and its six contract tests pass. The tests run only through `make lint` in the `ci` job. A change limited to either published installer page is classified `site=true, code=false`; that skips `ci`, while the website job runs taxonomy and Eleventy but not the installer contract.

The exact security-sensitive snippet can therefore regress in a documentation-only PR without running the test written to protect it.

Action: run `node --test scripts/installer-snippet.test.mjs` in the `site` job and add a routing contract proving that an installer-page-only change schedules it.

### RA-05 — RESOLVED — the 250 ms live-bubble deadline still relies on cooperative cancellation

Locations:

- `clients/vscode/src/bubble/live.ts:190-249`
- `docs/specs/vsix.md:46-49`

The branch correctly replaced the unused `AbortController` with a real VS Code `CancellationTokenSource`. The budget timer, however, only cancels the token. `isSuperseded` checks the probe epoch, store revision, URI, and document version, but not budget expiry.

JSON-RPC cancellation sends `$/cancelRequest`; it does not force the response promise to reject. If the server finishes after 250 ms, the late success still renders, contrary to the requirement to skip that edit cycle.

Action: record an expired state (or invalidate the probe epoch) when the deadline fires, reject both late success and failure UI mutations, and add an injected-clock fake-client test whose server ignores cancellation and resolves after expiry. Do not use sleeps.

### RA-06 — OPEN, tracked as #393 — REG-09 does not exercise Win32 path semantics on Windows

Locations:

- `clients/vscode/scripts/vscode-test-user-data-dir.mjs:18-84`
- `clients/vscode/scripts/vscode-test-user-data-dir.test.mjs:23-69`
- `.github/workflows/ci.yml:202-250`

The POSIX cap and Windows no-cap rules are now correct, and 23 script tests run through the Ubuntu `make lint` job. The injected `win32` case still calls the host `node:path`; on POSIX this does not exercise Windows separators or native `%TEMP%` behavior. The existing Windows job sets up Node but runs only Rust checks and TCP E2E.

Action: run `node --test clients/vscode/scripts/*.test.mjs` in the Windows job. For host-independent unit semantics, use `path.win32` when the injected platform is `win32`.

### RA-07 — OPEN, tracked as #394 — the Dependabot security-gate repair has no event-matrix regression test

Locations:

- `.github/workflows/action-selftest.yml:15-43`
- `.github/workflows/ci.yml:35-84,432-445`
- `.github/workflows/codeql.yml`
- `.github/workflows/dependabot-automerge.yml`
- `scripts/test-release-workflow-contract.mjs:12-29,302-335`

The actor skips and stale comments are fixed. Existing workflow tests cover the staging sweep, but none asserts that a Dependabot security PR to `main` retains CI, dependency review, CodeQL, and Action self-test while routine version updates remain on `dependabot-upgrades`.

Action: add a YAML-parsed event-matrix test for both PR classes and all four gates. Keep the sweep staging-only and actor-gated.

## Repository-contract regressions

### RA-08 — OPEN, tracked as #395 — the current-branch plan simultaneously marks completed work open and fixed

Location: `docs/plans/fused-score-followups.md`

The status ledger at lines 7-21 says #345 is partial, all six VSIX tests are open, and #339 is pinned red and unfixed. Lines 69-71 say the six tests are restored and green; lines 406-413 mark #345 complete; the current #339 production and regression tests are green. The later completion sections were added without reconciling the document's declared current ledger.

Action: rewrite the plan into one authoritative current-status ledger. Preserve earlier defect descriptions only under an explicitly historical heading, and distinguish “implemented in this worktree” from “issue closure awaiting CI/merge.”

### RA-09 — RESOLVED — three remediation test files now violate the hard 500-line rule

Repository rule: `CLAUDE.md:59` — files must remain below 500 lines.

| File | Comparison | Current |
|---|---:|---:|
| `clients/vscode/src/test/unit/live-bubble.unit.test.ts` | 458 | 528 |
| `clients/vscode/src/test/unit/report-store.unit.test.ts` | 420 | 506 |
| `clients/vscode/src/test/unit/extension-internals.unit.test.ts` | 493 | 555 |

Action: move race, notification-refresh, and retraction-ledger cases into focused suites without deleting or weakening assertions.

## Verification performed

| Check | Current result |
|---|---|
| `cargo fmt --all --check` | Passed |
| `cargo clippy --release --all-targets --workspace -- -D warnings` | Passed |
| `cargo test -p deslop-core --lib` | Passed — 13 |
| Corrected #339 signature unit | Passed |
| F# sibling-window E2E | Passed — 2 |
| Pair-admission bounded-max suite | Passed — 3 |
| Cross-language threshold suite | Passed — 3 |
| Corpus-confidence unit suite | Passed — 14; RA-02 is a predicate-design failure, not a red test |
| Focused `declared_inside_read_after_refuses` in repaired worktree | **Failed deterministically** |
| Same focused test in comparison checkout | Passed |
| VSIX typecheck and ESLint | Passed |
| VSIX script tests | Passed — 23 |
| `npx --no-install vsce ls` | Passed |
| VSIX extension-host assertions | Passed — 466 workspace + 1 no-folder |
| Top-level VSIX `npm test` process | Exit 1: both cached VS Code 1.133 launches ended with `SIGABRT` after their extension hosts reported clean assertion completion |
| Installer contract | Passed — 6 |
| `actionlint` on the affected CI, CodeQL, Action self-test, and Dependabot workflows | Passed |

Broad Rust execution also encountered sandbox-denied loopback binds in mock-server tests. Those environment failures were not classified as regressions. The real-repository corpus suite was not claimed green; the current Type-2 predicate must be corrected before such a green result can be treated as recall evidence.

No product code, tests, workflow configuration, or product documentation was changed during this audit. This rewritten audit document is the only intentional edit.
