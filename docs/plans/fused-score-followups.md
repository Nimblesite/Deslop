# Fused confidence — follow-ups for the next release

This plan tracks remaining `[FUSION-CONTENT-GATE]` work after `fusedhardening` and the v0.31.0 triage. Requirements live in [`root-cause-fusion.md`](../root-cause-fusion.md); the shipped mechanism is specified in [`fusion.md`](../specs/fusion.md#fusion-content-gate) and pinned by `fused_golden_bands.rs` and `fused_golden_invariants.rs`.

## Status ledger — 12 Aug 2026, current branch

What is fixed on this branch versus still outstanding. "Fixed" means the accuracy quarantine landed, the pinning test exists and is green, and the relevant suites passed locally — not that the issue is closed (closure waits on a green corpus CI run per the #331/#336 rule below).

| Issue | State | Evidence on this branch |
|---|---|---|
| **#347** corpus gate never boots | ✅ Fixed | `corpus.yml` now installs `typediagram@0.11.0` (pin matches `ci.yml`). Needs a `workflow_dispatch` after merge to produce the gate's first real measurement. |
| **#301** corpus determinism | ✅ Fixed | `snapshot_corpus` iteration-order defect quarantined with a mandated panic; ordered replacement landed. `corpus_determinism_nest_typescript` (1293 clusters / 30.0687% both runs) and `corpus_determinism_jellyfin_csharp` (1933 / 19.8354% both runs) green. `known-failures.json` ratcheted: `nest`/`jellyfin` `determinism` entries deleted; only `flutter`/`fsharp` `memory` (#166) remain. |
| **#343** sum-then-clamp saturation | ✅ Fixed | `PairScore::fused()` quarantined (mandated panic, `pair.rs`); `bounded_fused()` — max of the three axes, bounded to `[0,1]` — replaces it at every call site (admission in `survival_decision`, rendering in `ReportSignals`). Pinned by `issue_343_sum_clamp_saturation.rs` (mid-band `ts-mixed-band` fixture: st 0.00 / tj 0.30 / emb 0.94 rendered fused 1.000 before the fix; the test watched that failure). `ts-mixed-band` added to the `fused_golden_invariants.rs` sweep (now 21 corpora). 33 fused/bucket suites (220 tests) green; corpus cluster counts and percentages unchanged from the post-#301 baselines. |
| **#342** ancestor excludes → zero files | ✅ Fixed | `built_in_excluded` quarantined (mandated panic, `config.rs`); `corpus_built_in_excluded` replacement excludes only components below the scan root. Pinned by `issue_342_scan_root_under_excluded_ancestor.rs`, which asserts the `dist/`-rooted and plain-rooted reports agree — green. |
| **#344** carry confidence to every consumer | 🔴 Open | The admission row of its surface table is now fixed as a side-effect of #343 (`bounded_fused` at admission); every other surface — metrics gate, VSIX severity, text report, LSP, autofix preconditions, the wire fields, the 17 softened fixtures — is untouched. |
| **#345** doc drift | 🟡 Partial | `fusion.md` and `SPEC.md` reconciled on this branch: `[FUSION-STRATEGY-MAX-SUM]` now records the quarantine and specifies the bounded max; `[FUSION-EMBED-PROVIDER]` says "ensemble by max, never sum or average"; `[FUSION-CONTENT-GATE]` states it is the definition of rendered confidence. Outstanding: `REPORTING-CONTEXT.md` threshold naming, `mcp.md` top-offenders claim, `--embeddings` default discrepancy, `fused_spread`/`type2_recall` corpus check ids. |
| **Six skipped VSIX tests** (A, B1, B2, C, D, E) | 🔴 Open | All six still `test.skip`-ed; all six defects still in the shipped VSIX. A (act-now near miss withheld from the bubble) is the release-introduced regression and remains first in line. |
| **#331 / #336** | 🟡 Mechanism fixed, awaiting corpus proof | Synthetic suites green since #341; real-repository confirmation now unblocked because #347's fix lets the gate boot. Close only on a green corpus run. |
| **#339** LSH fallback signatures render `token_jaccard = 0.0` | 🔴 Open | Confirmed empirically during the #343 work: the `ts-mixed-band` mid-band cluster renders tj ≈ 0.30 where the true k-gram Jaccard is far higher, because sibling-window fallback signatures under-measure. Already tracked; not touched on this branch. |
| **#71 #79 #103 #283 #284 #285** (blocked on #343) | 🔴 Open | Unblocked by this branch; each needs its own verification against the bounded fusion before closing. |
| **#351** measured cosines discarded (found 12 Aug while hardening the #343 suite) | 🛑 Quarantined, red test in tree | `add_embedding_pair` (`pair/candidates.rs`) silently discarded the ANN pass's measured cosine for pairs already discovered structurally (byte-identical pairs render `embedding_cos = 0.0` under `--embeddings required` — a false figure) or by LSH (the pair reclassifies `lsh_only`, faces the stricter `token_jaccard ≥ 0.90` survival gate, and its cluster hides — a false negative decided by discovery order; a measured 0.8478 cosine was watched being discarded on the `ts-mixed-band` pair). Both arms now carry the mandated panic; `issue_343_sum_clamp_saturation.rs::byte_identical_pair_still_earns_full_confidence_under_the_bound` is the red pinning test and stays red until the accurate replacement lands (record the max cosine for every pair regardless of discovery route; keep recall accounting off the evidence path). The two embedding-enabled tests in that suite are red for this reason; the embeddings-off tests and all other suites are unaffected. |

## Shipped this release (context, not work)

**Measurement.** `ContentEvidence` (`crates/deslop-core/src/content.rs`)
replaces the single pooled `content_agreement` mean with two populations
scored separately: positional byte `agreement`, and Type-2
`rename_consistency` — the min of literal preservation and bijective
identifier-mapping coverage, gated behind ≥ 4 positional literal anchors
(`RENAME_EVIDENCE_MIN_LITERALS`). The anchor floor is the whole discriminator:
a sibling-scaffolding family also has a bijective identifier mapping, so
mapping consistency alone cannot separate it from a genuine rename —
preserved literals can. Modal-partner selection counts through `BTreeMap`s
with a strict-greater replacement rule, so the new measurement is
order-independent and adds no #301 surface.

**Routing and fusion.** Both populations feed the gate:
`fused = max(embedding_cos, max(structural, token_jaccard) × max(agreement,
0.9 × rename_consistency))`, and shape-identical clusters route on
`support = max(agreement, rename_consistency)` against
`CONTENT_SUPPORT_FLOOR` (0.7, demote) and `CONTENT_PROMOTE_FLOOR` (0.85,
promote).

**Recall.** A maximal Type-2 rename whose anchors prove its mapping now reaches the act-now `nearly_identical` bucket instead of being demoted to "same shape, different content". Seven fixture families moved bucket across five suites: `csharp-small`, `javascript-small`, `js-regex`, `js-destructuring`, `js-type2-pipeline`, `ts-type3-reorder`, and `jsx-js-components`. Anchor-poor renames (`js-classes`, `js-async`, `js-template-literals`, `js-optional-chaining`, and both `*-type2-loop` pairs) still route `structural_only`; the shape-only families from #341 still rank last.

**Ranking.** The gated confidence scales the final report weight
continuously (`confidence_factor` in `report.rs`), so a proven clone
outranks a shape-only family of equal geometry. `DataTable` is exempt,
preserving the documented `data_clone_weight = 1.0` restore contract —
the same carve-out `structural_only_multiplier` already had. MCP
`top-offenders` reads report order and inherits the new ordering.

**Rendering.** A `nearly_identical` cluster with `structural ≥ 0.99` renders
`token_jaccard = 1.0`: the Merkle match already proves the token multiset,
so the placeholder-dominated LSH fallback is corrected rather than reported
(#232, and it narrows #339's blast radius).

**Tests.** `fused_golden_bands.rs` (three bands × six languages, asserting
both score and rank order) and `fused_golden_invariants.rs` (20 corpora,
8 languages) pin the contract; the seven moved families were each
adjudicated against fixture contents before their expectations changed, and
the weak `token_jaccard < 0.05` fallback assertions were replaced with exact
`== 1.0` — strengthened, not relaxed. Fixture `js-structural-only` was
renamed `js-type2-pipeline`, since the old name asserted the verdict this
release overturned.

## 🛑 Skipped VSIX tests to restore

Six valid assertions are currently `test.skip`-ed under an owner-approved release exception. Restoring them outranks every other item in `docs/plans/` — do it before any feature work. Restore them without weakening or deleting them; each carries a `🛑 SKIPPED — DEFECT <x>` comment pointing here. All six defects remain in the shipped VSIX.

| # | Test | File | Defect |
|---|---|---|---|
| A | `an act-now near miss below the fused cutoff still reaches the bubble` | `live-bubble-fused.unit.test.ts` | `bestBubbleCluster` gates on a UI-local `fused >= FUSED_THRESHOLD` (0.85) instead of the engine's bucket, so act-now clusters below the cutoff are silently withheld from the live surface. |
| B1 | `classifyCluster must not call a content-gated rename byte-identical` | `report-schema.unit.test.ts` | `classifyCluster` reads a proven rename's corrected signals as `identical` and tells the user "Safe to extract — every copy is the same" about code whose identifiers all differ. |
| B2 | `classifyCluster must not promote a shape-only family the content gate demoted` | `report-schema.unit.test.ts` | A shape-only family with a non-trivial token signal falls through to the `structural >= 0.99` arm and is promoted to an act-now bucket — the false positive #341 exists to stop. |
| C | `the signal strip distinguishes a proven rename from a verbatim copy` | `live-bubble-fused.unit.test.ts` | `signalStrip` never draws the fused confidence, so a verbatim copy and a proven rename both render `██▁`. |
| D | `a demoted shape-only family is not painted with act-now severity` | `severity.unit.test.ts` | Severity is pure rank, so a large demoted family that still sorts first gets the loudest decoration in the editor. |
| E | `a stale probe cannot resurrect a cluster the visible report dropped` | `live-bubble.unit.test.ts` | `bestBubbleCluster`'s `byId.get(id) ?? cluster` fallback re-paints a cluster the delta just cleared. **Least clear-cut of the six** — the same fallback legitimately serves clusters found live before a rescan. Settle the intended contract first, then fix the code or restate the test. |

**Defect A is a regression this release introduced.** The content gate
made `fused` systematically lower for act-now clusters (it is now shape ×
content), so near misses that previously rendered at a clamped 1.0 and
always bubbled now land near 0.80 and vanish. It is the one to fix first.

B1 and B2 share a root cause with issue **#344**: the UI re-derives buckets
from a signal triple that cannot see `ContentEvidence` because those fields
are not on the wire. Putting `agreement` / `rename_consistency` /
`literal_fraction` on `ReportSignals` and having `resolveBucket` trust the
engine's label unconditionally retires both. C and D are the rendering and
severity rows of the same issue.

## Triage — 12 Aug 2026

Every open issue now carries a GitHub type (Bug / Task / Feature), and the
blocking relationships below are recorded as GitHub issue dependencies — so
the tracker *enforces* the order this document describes instead of merely
restating it.

**New this round:**

| Issue | Type | Why it exists |
|---|---|---|
| **#347** | Bug | `corpus.yml` never installs `typediagram`, so `build.rs` cannot generate the wire models and the crate fails to compile. The `[CORPUS-CI]` accuracy gate has **never produced a measurement**: both runs since `fc779f7bc` — the commit that added it — died at ~70s, before a single repository was scanned. |
| **#348** | Bug | One transient Marketplace timeout aborts the remaining platforms under `set -e`. v0.31.0 was live on darwin-arm64 only until the job was re-run by hand, while Open VSX served all five. `publish-openvsx` has the identical loop shape and survived by ordering luck. Release infrastructure — tracked outside the ordered work below. |
| **#342** | Bug | `built_in_excluded` matches path components *above* the scan root, so a repo living under any folder named `dist`/`build`/`target`/`vendor`/`node_modules` analyses as zero files: clean report, exit code 0. **Fixed on this branch** — see the section below; the pinning test now exists and is green. |

**The dependency graph now recorded on GitHub:**

```
#347 corpus gate ──┬──► #301 determinism ──► #343 sum-then-clamp ──┬──► #344
  (never booted)   │                                              ├──► #345
                   └──────────────────────────────────────────────┘
                                    #343 also blocks
                        #71   #79   #103   #283   #284   #285

#342 ancestor excludes ──► #298 add `out/` to the defaults
```

`root-cause-fusion.md` names one root cause behind eight bugs at
`pair.rs:72`. Two of them (#331, #336) were fixed by #341; the remaining six
are now formally blocked on #343 rather than sitting unattributed. #298 is
blocked by #342 because adding `out` to the built-in defaults strictly
widens #342's blast radius — #342 was found while investigating #298.

**#331 and #336 are fixed but still open.**
`issue_331_336_shape_only_saturation.rs` (3 tests) and
`fsharp_issue_336_data_table_category.rs` (4 tests) are green, and #341
deleted both `flutter/boilerplate_rank` and `fsharp/data_table_rank` from
`corpus/known-failures.json` — which flips the gate from *tolerating* them
to *demanding* they pass. But those are synthetic fixtures: they prove the
mechanism demotes shape-only families, not the original claims (rank #1 on
`flutter/flutter` at 453 occurrences; rank #1 on `dotnet/fsharp` at 3,544).
The real-repository confirmation lives in `check_boilerplate_not_ranked_first`
and `check_data_tables_not_ranked_as_logic`, inside the gate #347 says has
never run. **Close them when a green corpus run exists, not before** —
closing on evidence CI has never seen is precisely what #347 exists to
prevent.

## ✅ #342 — total false negative, fixed on this branch

When this plan was written, the pinning test the issue described
(`issue_<this>_scan_root_under_excluded_ancestor.rs`) did not exist: the
most severe defect on the board — a repo under any folder named
`dist`/`build`/`target`/`vendor`/`node_modules` analysing as zero files
with a clean report and exit code 0 — had no assertion, while the tracker
claimed it did.

Fixed test-first per `CLAUDE.md`:

- `crates/deslop/tests/issue_342_scan_root_under_excluded_ancestor.rs`
  seeds a clone-bearing repo under `<tmp>/dist/…` and under a plain root,
  asserts `files_analysed == 2`, the cross-file cluster, and that the two
  reports agree — the equivalence assertion is what stops a future change
  to the exclusion list from silently reintroducing the bug. It was
  watched failing for the real reason before the fix.
- `built_in_excluded` (`config.rs`) is quarantined with the mandated
  panic; the replacement `corpus_built_in_excluded` makes only components
  **below** the scan root eligible for built-in exclusion, matching the
  principle `scan_root_contains_component_pair` already encoded for the
  report-hide pairs.

#298 (add `out/` to the defaults) is now safe to take: the blast-radius
concern that blocked it was exactly this defect.

## Open work, in order

### 1. ✅ #347 — make the corpus gate boot (fixed on this branch)

Everything below is measured by an instrument that had never been switched
on. `corpus.yml` now installs `typediagram@0.11.0` to match the jobs in
`ci.yml` (pin kept in sync per the dependency-sync rule). Remaining after
merge: `workflow_dispatch` the gate and reconcile
`corpus/known-failures.json` against its first real run — that baseline has
never been confirmed by a passing CI run, though the local determinism
ratchet below already trimmed it.

### 2. ✅ #301 — determinism (fixed on this branch; close on a green corpus run)

The iteration-order defect in `snapshot_corpus` is quarantined with the
mandated panic and replaced by an ordered traversal. Both local
determinism checks are green — `nest` (1293 clusters / 30.0687% on both
runs) and `jellyfin` (1933 / 19.8354% on both runs) — and the
`determinism` entries for both repos are **deleted** from
`corpus/known-failures.json`, flipping the gate from tolerating
non-determinism to demanding its absence. Only `flutter`/`fsharp`
`memory` (#166) remain baselined. Close the issue when the corpus CI
workflow confirms what the local runs measured.

### 3. ✅ #343 — replace sum-then-clamp saturation (fixed on this branch)

`PairScore::fused()` summed three correlated signals and clamped: the
mid-band `ts-mixed-band` cluster at `structural = 0.00, token_jaccard =
0.30, embedding_cos = 0.94` rendered `fused = 1.000` — indistinguishable
from a byte-proven verbatim copy — and was never gated because no
component crossed a corner threshold (`structural ≥ 0.99` / `token ≥
0.95`). Fixed test-first:

- `issue_343_sum_clamp_saturation.rs` pins the contract (rendered
  confidence never exceeds the strongest single axis without a
  byte-identical pair, never saturates at 1.0, and stays act-now-worthy)
  and was watched failing against the sum for exactly that reason.
- `fused()` is quarantined with the mandated panic; the replacement
  `PairScore::bounded_fused()` takes the strongest single axis, bounded
  to `[0,1]`, at both call sites (pair admission and `ReportSignals`
  rendering). [FUSION-CONTENT-GATE] remains the definition of rendered
  confidence for shape-saturating clusters; elsewhere the bounded max is
  the same formula with the content factor at its implicit 1.0.
- `ts-mixed-band` joined the `fused_golden_invariants.rs` sweep (21
  corpora). 33 fused/bucket-adjacent suites (220 tests) and both corpus
  determinism gates ran green; corpus cluster counts and duplication
  percentages are unchanged from the post-#301 baselines, so admission
  behaviour did not shift on real repositories.

### 4. #344 — carry the confidence to every consumer

Surfaces still running on the pre-gate world:

| Surface | Today |
|---|---|
| Pair admission to a cluster (`pair.rs`) | ✅ Fixed by #343 — admission now uses `bounded_fused()` (strongest single axis), no longer the raw clamped sum |
| `metrics.duplication_percent` / exit-code gate (`report.rs`) | Counts lines of visible clusters, unweighted — shape matches breach like verbatim copy-paste |
| VSIX severity / decorations / tree (`severity.ts`) | Rank-derived, never reads `fused` |
| CLI text report (`render/text.rs`) | Prints no signals at all |
| LSP diagnostics / code lens (`deslop-lsp`) | No confidence anywhere |
| Autofix extract / consolidate gates (`refactor/preconditions.rs`) | Bucket pre-filter + byte proof only |

Plus, in the same issue:

- Put `agreement`, `rename_consistency`, and `literal_fraction` on the wire:
  add to `ReportSignals` in [`docs/models/live-ipc.td`](../models/live-ipc.td),
  regenerate, and render them everywhere `fused` renders (HTML footer,
  Markdown, VSIX `SignalStrip`, `HelpBubble`). Until then no black-box test
  can assert the gate's input, and neither humans nor agents can see *why*
  a cluster routed where it did.
- Restore the 17 fixtures that #341 softened from maximal renames to
  partial renames — the engine can carry the originals now, and the golden
  bands suite proves it per language.

### 5. #345 — doc drift

- `REPORTING-CONTEXT.md` (shipped to agents via `schema_doc`) still calls
  `FUSED_THRESHOLD = 0.85` what a pair needs "to enter a cluster"; the
  rendered confidence and the admission threshold are two quantities
  sharing one name.
- `mcp.md` claims `top-offenders` sorts "by fused score"; it sorts by
  weight (now confidence-scaled, still not `fused`).
- `--embeddings` defaults to `off` while `EmbeddingMode` docs call `auto`
  the default — requirement 1 of `root-cause-fusion.md` is unmet by
  default.
- ✅ Done on this branch: `fusion.md`'s two fusion rules
  (`[FUSION-STRATEGY-MAX-SUM]` and `[FUSION-CONTENT-GATE]`) are
  reconciled — the strategy section records the #343 quarantine and
  specifies the bounded max, and the gate section states it is the
  definition of rendered confidence. `SPEC.md`'s strategy row now points
  at `PairScore::bounded_fused`.
- `corpus/known-failures.json` has no confidence check id — add a
  `fused_spread` / `type2_recall` check so the real-repository gate can
  catch saturation and rename recall at scale. Blocked by #347: adding a
  check to a gate that cannot compile is a no-op.

## Requirement status ([`root-cause-fusion.md`](../root-cause-fusion.md))

| # | Requirement | Status |
|---|---|---|
| 1 | Give the ensemble an independent member | 🟡 Content evidence is independent and now steers routing, fusion, and ranking — but it remains a render-stage gate rather than an ensemble member, and the semantic signal is still off by default. |
| 2 | Stop clamping away the top of the range | ✅ #343 fixed on this branch: the sum-then-clamp is quarantined and `bounded_fused()` never exceeds the strongest single axis, so `fused = 1.0` again requires an axis that actually measured 1.0 (byte proof, per the content gate). |
| 3 | Preserve some literal information | 🟡 `ContentEvidence` now compares raw literal bytes positionally (literal preservation is the Type-2 discriminator). The fingerprint and token layers still collapse every literal to `__literal__`. |
