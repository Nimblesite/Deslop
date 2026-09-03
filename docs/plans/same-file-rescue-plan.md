# Same-file shared-subtree rescue

Tracking issues: gh #492 (two drifted methods never cluster) and gh #496 (two methods differing only in literals are refused below the promote floor). One band, one fix.

Two methods that drifted apart inside one file are the same duplication as two that drifted apart across files. The file boundary records where the copy was pasted, not whether it is a copy. This plan closed that gap; what remains open is recorded at the bottom.

## What landed

Three routes, each stated in [fused.md](../specs/fused.md) and each pinned:

**The bucket star, inside one file** ([FUSED-CANDIDATE-BUCKET-STAR]). A structural-hash bucket paired every member with the member that sorted first. Inside one file that member decides everything, and when it is the one that *differs* the byte-identical pair behind it was never a candidate at all — one unrelated sibling deleted an exact duplicate from the report. Members of a bucket that share a file are now paired with each other, all of them.

**The rescue, inside one file** ([FUSED-SHARED-SUBTREE-SAME-FILE]). A same-file pair reaches shared-subtree measurement when both endpoints are whole authored declarations, they do not overlap, they still enclose a Merkle-equal clone of at least `admission.shared_subtree_min_node_count` nodes, and the shared mass *beyond* that clone clears the same floor. `csharp-merge-drift` now publishes `ApplyStandard` and `ApplyPremium` as one cluster with the statement fragments absorbed.

**The literal alphabet** ([FUSED-CONTENT-GATE-PARAMETER]). Where the identifier bijection claims no rename, a literal substitution seen once is Baker's unconstrained wildcard rather than a contradiction, so `csharp-merge-manyholes` — every identifier and every call preserved, twelve literal positions substituted — is judged as the parameterised copy it is instead of on `agreement` alone.

## Why admitting every same-file pair was wrong

Admitting each otherwise-valid same-file candidate to rescue measurement was tried and reverted. It publishes families that are not duplication:

- `dart-issue-197-settings-getters` — one class of REST accessors that share a skeleton and agree on 36% of their positions. Eight convicted components where the release convicts one.
- `python-issue-103-helper-call-sites` — test functions that each call one already-extracted helper.
- `issue_190` data tables outrank the logic clone they sit beside, because a table repeated in one file carries more mass than a real clone.

Requiring whole authored declarations removes the table and window cases but not the accessor family: its overlap (0.81–0.88) brackets the drifted pair's 0.84 and its raw-content agreement reaches 0.56 against the drifted pair's 0.55. What the copy has and the family has not is **authored code the edit never touched** — a Merkle-equal clone inside both declarations, which the pipeline already computes as its own candidate pair. That is condition 2. Condition 3 is the existing echo rule turned inward: when the pair shares nothing *beyond* that clone, the clone is the finding and the wider view would only displace it (`csharp-merge-readafter`).

## The forwarding band — gh #496 and gh #497

`dart_forwarding_fail_open`'s five fixtures are one family, and its module documentation states a positive contract for every one of them: each is liftable duplication that the forwarding proof must not hide. Four of the five asserted the opposite. They now assert the contract:

| fixture | published pair | support | rename |
| --- | --- | --- | --- |
| `dart-forwarding-fail-open` | `Calc.scaledDomestic` / `scaledExport` | 0.583 | 1.00 |
| `dart-forwarding-duplicate-route` | `Api.resetDelta` / `resetEpsilon` | 0.857 | — |
| `dart-forwarding-business-pair` | `Pricing.standardTotal` / `premiumTotal` | 0.727 | 1.00 |
| `dart-forwarding-transform-after-delegation` | `Ledger.standardTotal` / `premiumTotal` | 0.750 | 1.00 |
| `dart-forwarding-transform-before-delegation` | `Billing.quarterlyFee` / `annualCharge` | 0.750 | 1.00 |

What separates them from the scaffolding beside them is not a floor. `Api`'s three distinct-route wrappers measure agreement 0.714 and the `#197` settings family 0.77–0.82 — *above* two of the pairs that must publish — so no value of a content floor can sort them. What sorts them is anchor mass on the rename axis: 5 affirming positions for a one-line REST wrapper, 8 and 9 for a two-statement business method ([REPAIR-RENAME-ANCHOR-MASS]). `Pricing`'s renamed arrow pair stays out at 0.375, exactly as its own fixture comment says it must.

Two consequences worth naming:

- `dart_issue_197_single_file_structural_only` now convicts **two** components rather than one: the `resetX` wrappers close into a family they previously never reached at all. Nothing it asserts about the report changes — no cluster published, no line counted, no percentage moved — and the count stays exact.
- `deslop-core`'s `content_gate_rejects` pin used `dart-forwarding-business-pair` as its example of a content-rejected pair. It now uses `csharp-issue-134-structural-only`, which still is one, with both assertions unchanged.

## Acceptance

- [x] `csharp_same_file_type3_reports_both_methods_in_one_cluster` passes with its assertions unchanged.
- [x] `csharp-merge-manyholes` gains its occurrence and range pin.
- [x] `dart_forwarding_fail_open`'s duplicate-route control asserts what the module documentation states.
- [x] The before/after-delegation and business controls agree with each other and with the module documentation (gh #496, gh #497).
- [x] `dart_issue_197_single_file_structural_only`, `python_issue_103_helper_call_sites`, the three `issue_190` modes, `cli::bucket_groups`, both `refactor_merge_refusals` same-file pins and `cross_cluster_collapse` stay green.
