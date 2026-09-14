# Same-file shared-subtree rescue

Tracking issue: gh #492 (two drifted methods never cluster). gh #496 (two methods differing only in literals were refused below a same-file promote floor) is settled below and no longer needs the rescue.

Two methods that drifted apart inside one file are the same duplication as two that drifted apart across files. The file boundary records where the copy was pasted, not whether it is a copy. The shared-subtree rescue of [FUSED-SHARED-SUBTREE](../specs/fused.md) was cross-file only, so a same-file near-miss published the statement fragments its two methods share and never the methods. It reaches them now, and gh #492 is closed alongside gh #496.

## What the gap costs

The band has two halves. Below the 0.70 support floor, [FUSED-CONTENT-GATE] refuses a pair outright; above it the pair is admitted with no rescue needed and the sibling-family question goes to the forwarding proof.

`csharp-merge-drift` holds `ApplyStandard` and `ApplyPremium` in `DriftLimits.cs`. They share a five-call skeleton; the premium copy grew an escalation guard and its own literals. The pair measures shared-subtree overlap 0.82. Nothing publishes it: 0.32.0 reported four fragment clusters covering two-line windows and single statements, and the current release reports the exact tail alone. A reader is told about pieces of a duplication and never about the duplication.

Pinned by `type3_enclosing_method::csharp_same_file_type3_reports_both_methods_in_one_cluster`, which asserts one cluster over lines 3-13 and 15-29 with every fragment absorbed. Its `#[ignore]` is gone and its assertions are unchanged.

The other half needed no rescue at all, only the floor, and it is settled. `dart-forwarding-business-pair` holds `standardTotal` and `premiumTotal`: structurally identical, differing in one string literal and one integer, measuring agreement 0.727 and rename consistency 0.0. The 0.85 same-file admission floor that refused it was PR #485's relocation of a render-time bucket grade to admission; 0.32.0 published the pair as `structural_only` and left the sibling-family question to [RANK-STRUCTURAL-ONLY-FORWARDING], which reads where each call goes. That is the discriminator: the REST settings family and a two-literal copy of one method measure in the same 0.70–0.85 band, so no admission floor separates them, and the forwarding proof does. Every pair now pays `content_gate.support_floor` in every scope; only an unanchored LSH-only pair pays `promote_floor`. `dart_forwarding_fail_open.rs` asserts the positive contract its documentation always stated (gh #496, gh #497), `declaration_family_plurality` again publishes the nonbijective pair its fixture calls liftable, and `content_gate_admits.rs` pins the admission at the pipeline seam.

`rename_consistency` was right to report 0.0 for the pair: nothing in it is renamed, and the rename axis measures renames. A literal-only copy is judged on agreement, which is the lever that now does the work.

`csharp-merge-manyholes` (agreement 0.50–0.57) still falls below the support floor, and agreement is not the axis that should judge it. Every identifier and every call is preserved and only the twelve literals move, which [FUSED-CONTENT-GATE-PARAMETER] now reads as the parameterisation it is: where the bijection claims no rename, a literal substituted once is Baker's unconstrained wildcard rather than a contradiction. `Sprawl.cs:3-12` / `:14-23` publishes.

## Why admitting every same-file pair is wrong

Admitting each otherwise-valid same-file candidate to rescue measurement was tried and reverted. It publishes families that are not duplication:

- `dart-issue-197-settings-getters` — one class of REST accessors that share a skeleton and agree on 36% of their positions. Eight convicted components where the release convicts one.
- `python-issue-103-helper-call-sites` — test functions that each call one already-extracted helper. A two-member cluster publishes where the release demotes it.
- `issue_190` data tables outrank the logic clone they sit beside, because a table repeated in one file carries more mass than a real clone.

Requiring both endpoints to be whole authored functions removes the table and window cases but not these: a class of sibling accessors is a set of whole authored functions. Requiring a Merkle-equal fragment inside both endpoints does not remove them either, because the call sites share byte-identical argument runs.

## What landed

**The rescue, inside one file** ([FUSED-SHARED-SUBTREE-SAME-FILE]). A same-file pair reaches shared-subtree measurement when both endpoints are whole authored declarations, they do not overlap, they still enclose a Merkle-equal clone of at least `admission.shared_subtree_min_node_count` nodes, and the shared mass *beyond* that clone clears the same floor. The second condition is the discriminator this plan was looking for, and it is neither of the thresholds below: the settings accessors' overlap (0.81–0.88) brackets the drifted pair's 0.84 and their agreement (up to 0.56) brackets its 0.55, so neither shape nor content sorts them — but the drifted pair keeps four whole statements the edit never touched and the family keeps none. The third condition is the echo rule turned inward, and it is why `csharp-merge-readafter` still publishes the contiguous run its two methods share rather than the methods wrapping it.

**The bucket star, inside one file** ([FUSED-CANDIDATE-BUCKET-STAR]). A structural-hash bucket paired every member with the member that sorted first. Inside one file that member decides everything, and when it is the one that *differs*, the byte-identical pair behind it was never a candidate at all — one unrelated sibling deleted an exact duplicate from the report. Members that share a file are now paired with each other, all of them. The cost is quadratic in same-file bucket size and is tracked as gh #506.

## Discriminators considered and not taken

A discriminator that separates a copied method from a shape family, computed from evidence the pipeline already produces, not a threshold fitted between two fixtures. Candidates worth measuring:

1. **Exact statement-run mass.** The drift methods share five whole byte-identical statements; the settings getters share none. Measure the largest Merkle-equal run of whole authored statements inside both endpoints and require it to carry a configured share of the smaller endpoint's mass.
2. **Interior agreement rather than positional agreement.** [FUSED-CONTENT-GATE-INTERIOR] already judges a rename on the whole method. A same-file family whose interiors name different endpoints should fail it where a drifted copy passes.
3. **Family size.** A pair drawn from a component of many same-shape siblings in one file is a family, and [CLONE-NOISE-VERBATIM-SUBGROUP-FAMILY] already convicts families. A rescue that consults the pre-gate family before admitting a same-file pair would refuse the accessor class on evidence the pipeline computes anyway.

## Acceptance — how gh #492 and its skip end

- [x] `csharp_same_file_type3_reports_both_methods_in_one_cluster` passes with its assertions unchanged and its `#[ignore]` removed.
- [x] `dart_forwarding_fail_open`'s controls keep asserting what the module documentation states, and `csharp-merge-manyholes` gains an occurrence and range pin.
- [x] `dart_issue_197_single_file_structural_only`, `python_issue_103_helper_call_sites`, the three `issue_190` modes, `cli::bucket_groups` and both `refactor_merge_refusals` same-file pins stay green.
- [x] The paired 0.32.0 fixture scan loses no finding.
