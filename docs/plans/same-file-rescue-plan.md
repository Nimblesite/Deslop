# Same-file shared-subtree rescue

Tracking issues: gh #492 (two drifted methods never cluster) and gh #496 (two methods differing only in literals are refused below the promote floor). One band, one fix.

Two methods that drifted apart inside one file are the same duplication as two that drifted apart across files. The file boundary records where the copy was pasted, not whether it is a copy. Today the shared-subtree rescue of [FUSED-SHARED-SUBTREE](../specs/fused.md) is cross-file only, so a same-file near-miss publishes the statement fragments its two methods share and never the methods.

## What the gap costs

The band has two halves and they meet in the middle. Below the 0.85 same-file promote floor, [FUSED-CONTENT-GATE] refuses a pair outright; above it the pair is admitted with no rescue needed. Between a literal-only copy and a shape family there is currently nothing but that number.

`csharp-merge-drift` holds `ApplyStandard` and `ApplyPremium` in `DriftLimits.cs`. They share a five-call skeleton; the premium copy grew an escalation guard and its own literals. The pair measures shared-subtree overlap 0.82. Nothing publishes it: 0.32.0 reported four fragment clusters covering two-line windows and single statements, and the current release reports the exact tail alone. A reader is told about pieces of a duplication and never about the duplication.

Pinned by `type3_enclosing_method::csharp_same_file_type3_reports_both_methods_in_one_cluster`, which asserts one cluster over lines 3-13 and 15-29 with every fragment absorbed. It is red, with its assertions intact, until the route below exists.

The other half needs no rescue at all, only the floor. `dart-forwarding-business-pair` holds `standardTotal` and `premiumTotal`: structurally identical, differing in one string literal and one integer. The pair measures agreement 0.727 and rename consistency 0.0, so the 0.85 floor refuses it. `dart-forwarding-duplicate-route`, `dart-forwarding-transform-before-delegation` and `csharp-merge-manyholes` fall the same way. 0.32.0 published all four, and `dart_forwarding_fail_open.rs` describes its pairs as liftable duplication that must stay on the report, while its assertions now require them absent. That contradiction is gh #496 and it has to be settled before the rescue question is worth asking: if the floor is what refuses a two-literal copy, no rescue route reaches the pair either.

Settle first whether `rename_consistency` is right to report 0.0 for a pair whose two varied positions substitute consistently. If it counts identifier renames only, then a literal-only copy is judged on agreement alone and the lever named at the gate is not the lever doing the work.

## Why admitting every same-file pair is wrong

Admitting each otherwise-valid same-file candidate to rescue measurement was tried and reverted. It publishes families that are not duplication:

- `dart-issue-197-settings-getters` — one class of REST accessors that share a skeleton and agree on 36% of their positions. Eight convicted components where the release convicts one.
- `python-issue-103-helper-call-sites` — test functions that each call one already-extracted helper. A two-member cluster publishes where the release demotes it.
- `issue_190` data tables outrank the logic clone they sit beside, because a table repeated in one file carries more mass than a real clone.

Requiring both endpoints to be whole authored functions removes the table and window cases but not these: a class of sibling accessors is a set of whole authored functions. Requiring a Merkle-equal fragment inside both endpoints does not remove them either, because the call sites share byte-identical argument runs.

## What the route needs

A discriminator that separates a copied method from a shape family, computed from evidence the pipeline already produces, not a threshold fitted between two fixtures. Candidates worth measuring:

1. **Exact statement-run mass.** The drift methods share five whole byte-identical statements; the settings getters share none. Measure the largest Merkle-equal run of whole authored statements inside both endpoints and require it to carry a configured share of the smaller endpoint's mass.
2. **Interior agreement rather than positional agreement.** [FUSED-CONTENT-GATE-INTERIOR] already judges a rename on the whole method. A same-file family whose interiors name different endpoints should fail it where a drifted copy passes.
3. **Family size.** A pair drawn from a component of many same-shape siblings in one file is a family, and [CLONE-NOISE-VERBATIM-SUBGROUP-FAMILY] already convicts families. A rescue that consults the pre-gate family before admitting a same-file pair would refuse the accessor class on evidence the pipeline computes anyway.

## Acceptance — how gh #492 and its skip end

- `csharp_same_file_type3_reports_both_methods_in_one_cluster` passes with its assertions unchanged.
- `dart_forwarding_fail_open`'s business, duplicate-route and transform-before-delegation controls assert what the module documentation states, and `csharp-merge-manyholes` gains an occurrence and range pin either way the question is settled.
- `dart_issue_197_single_file_structural_only`, `python_issue_103_helper_call_sites`, the three `issue_190` modes, `cli::bucket_groups` and both `refactor_merge_refusals` same-file pins stay green.
- The paired 0.32.0 fixture scan loses no finding.
