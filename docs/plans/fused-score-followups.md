# Fused confidence — follow-ups for the next release

This plan tracks remaining `[FUSION-CONTENT-GATE]` work after `fusedhardening` and the v0.31.0 triage. Requirements live in [`root-cause-fusion.md`](../root-cause-fusion.md); the shipped mechanism is specified in [`fusion.md`](../specs/fusion.md#fusion-content-gate) and pinned by `fused_golden_bands.rs` and `fused_golden_invariants.rs`.

Repairing the quarantine panics is planned separately in [`quarantine-repair-plan.md`](quarantine-repair-plan.md), which also absorbs the P0/P1 regressions from `BRANCH_REVIEW.md`.

## Status ledger — 12 Aug 2026, current branch

What is fixed on this branch versus still outstanding. "Fixed" means the accuracy quarantine landed, the pinning test exists and is green, and the relevant suites passed locally — not that the issue is closed (closure waits on a green corpus CI run per the #331/#336 rule below).

| Issue | State | Evidence on this branch |
|---|---|---|
| **#347** corpus gate never boots | ✅ Fixed | `corpus.yml` now installs `typediagram@0.11.0` (pin matches `ci.yml`). Needs a `workflow_dispatch` after merge to produce the gate's first real measurement. |
| **#301** corpus determinism | ✅ Fixed | `snapshot_corpus` iteration-order defect quarantined with a mandated panic; ordered replacement landed. `corpus_determinism_nest_typescript` (1293 clusters / 30.0687% both runs) and `corpus_determinism_jellyfin_csharp` (1933 / 19.8354% both runs) green. `known-failures.json` ratcheted: `nest`/`jellyfin` `determinism` entries deleted; only `flutter`/`fsharp` `memory` (#166) remain. |
| **#343** sum-then-clamp saturation | ✅ Fixed | `PairScore::fused()` quarantined (mandated panic, `pair.rs`); `bounded_fused()` — max of the three axes, bounded to `[0,1]` — replaces it at every call site (admission in `survival_decision`, rendering in `ReportSignals`). Pinned by `issue_343_sum_clamp_saturation.rs` (mid-band `ts-mixed-band` fixture: st 0.00 / tj 0.30 / emb 0.94 rendered fused 1.000 before the fix; the test watched that failure). `ts-mixed-band` added to the `fused_golden_invariants.rs` sweep (now 21 corpora). 33 fused/bucket suites (220 tests) green; corpus cluster counts and percentages unchanged from the post-#301 baselines. |
| **#342** ancestor excludes → zero files | ✅ Fixed | `built_in_excluded` quarantined (mandated panic, `config.rs`); `corpus_built_in_excluded` replacement excludes only components below the scan root. Pinned by `issue_342_scan_root_under_excluded_ancestor.rs`, which asserts the `dist/`-rooted and plain-rooted reports agree — green. |
| **#344** carry confidence to every consumer | 🔴 Open | Two of its rows are closed: admission uses `bounded_fused` (side-effect of #343), and **VSIX severity** now resolves colour from the engine's post-gate bucket (Defect D). Still untouched: metrics gate, text report, LSP, autofix preconditions, the wire fields, the 17 softened fixtures. |
| **#345** doc drift | 🟡 Partial | `fusion.md` and `SPEC.md` reconciled on this branch: `[FUSION-STRATEGY-BOUNDED-MAX]` now records the quarantine and specifies the bounded max; `[FUSION-EMBED-PROVIDER]` says "ensemble by max, never sum or average"; `[FUSION-CONTENT-GATE]` states it is the definition of rendered confidence. Outstanding: `REPORTING-CONTEXT.md` threshold naming, `mcp.md` top-offenders claim, `--embeddings` default discrepancy, `fused_bounded_max`/`type2_recall` corpus check ids. |
| **Six skipped VSIX tests** (A, B1, B2, C, D, E) | 🔴 Open | All six still `test.skip`-ed; all six defects still in the shipped VSIX. A (act-now near miss withheld from the bubble) is the release-introduced regression and remains first in line. |
| **#331 / #336** | 🟡 Mechanism fixed, awaiting corpus proof | Synthetic suites green since #341; real-repository confirmation now unblocked because #347's fix lets the gate boot. Close only on a green corpus run. |
| **#339** LSH fallback signatures render `token_jaccard = 0.0` | 🔴 Open, **pinned red** | Now isolated at the layer where it is provable: `deslop-core::pipeline::signatures::tests::issue_339_sibling_window_signature_is_offset_invariant` parses two F# modules whose shared window is byte-identical at shifted offsets, gives both the same structural hash, and asserts their signatures match. It does not — they fall through to `blake3(hash, byte_range)` and share nothing. **The test is left red**: quarantining `fallback_signature` behind a `panic!` would abort every scan containing an unresolvable range, which is most of them. Evidence posted to #339. Not fixed on this branch. |
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

## ✅ Skipped VSIX tests — all six restored

Six valid assertions were `test.skip`-ed under an owner-approved release exception. **All six are now un-skipped and green; `grep -c "test.skip" clients/vscode/src/test/unit/*.test.ts` returns zero.** None was weakened — every one gained assertions. The table below is the original statement of each defect, kept because the fixes are recorded against it.

| # | Test | File | Defect |
|---|---|---|---|
| A | `an act-now near miss below the fused cutoff still reaches the bubble` | `live-bubble-fused.unit.test.ts` | `bestBubbleCluster` gates on a UI-local `fused >= FUSED_THRESHOLD` (0.85) instead of the engine's bucket, so act-now clusters below the cutoff are silently withheld from the live surface. |
| B1 | `classifyCluster must not call a content-gated rename byte-identical` | `report-schema.unit.test.ts` | `classifyCluster` reads a proven rename's corrected signals as `identical` and tells the user "Safe to extract — every copy is the same" about code whose identifiers all differ. |
| B2 | `classifyCluster must not promote a shape-only family the content gate demoted` | `report-schema.unit.test.ts` | A shape-only family with a non-trivial token signal falls through to the `structural >= 0.99` arm and is promoted to an act-now bucket — the false positive #341 exists to stop. |
| C | `the signal strip distinguishes a proven rename from a verbatim copy` | `live-bubble-fused.unit.test.ts` | `signalStrip` never draws the fused confidence, so a verbatim copy and a proven rename both render `██▁`. |
| D | `a demoted shape-only family is not painted with act-now severity` | `severity.unit.test.ts` | Severity is pure rank, so a large demoted family that still sorts first gets the loudest decoration in the editor. **Confirmed and worse:** the paint was *inverted* — crimson on the demoted family, blue on the byte-proven clone. |
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
| `metrics.duplication_percent` / exit-code gate (`report.rs`) | Counts lines of visible clusters, unweighted — shape matches breach like verbatim copy-paste. Design settled: side-by-side evidence-weighted metric + second gate, specced in [pipeline.md §METRICS-REPO-WEIGHTED](../specs/pipeline.md#metrics-repo-weighted), sequenced in [weighted-metrics-plan.md](weighted-metrics-plan.md) |
| VSIX severity / decorations / tree (`severity.ts`) | ✅ Fixed by D — colour is bucket-derived via `resolveSeverity(bucket, percentile)` per [SEVERITY-COLOR](../specs/severity.md#severity-color); glyph density stays percentile-derived. It deliberately reads the **bucket**, not `fused`: the bucket is the engine's verdict *after* the content gate, so a surface that trusts it needs no second opinion — the same conclusion §2 reached for routing |
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
  (`[FUSION-STRATEGY-BOUNDED-MAX]` and `[FUSION-CONTENT-GATE]`) are
  reconciled — the strategy section records the #343 quarantine and
  specifies the bounded max, and the gate section states it is the
  definition of rendered confidence. `SPEC.md`'s strategy row now points
  at `PairScore::bounded_fused`.
- `corpus/known-failures.json` has no confidence check id — add a
  `fused_bounded_max` / `type2_recall` check so the real-repository gate can
  catch saturation and rename recall at scale. Blocked by #347: adding a
  check to a gate that cannot compile is a no-op.

## Requirement status ([`root-cause-fusion.md`](../root-cause-fusion.md))

| # | Requirement | Status |
|---|---|---|
| 1 | Give the ensemble an independent member | 🟡 Content evidence is independent and now steers routing, fusion, and ranking — but it remains a render-stage gate rather than an ensemble member, and the semantic signal is still off by default. |
| 2 | Stop clamping away the top of the range | ✅ #343 fixed on this branch: the sum-then-clamp is quarantined and `bounded_fused()` never exceeds the strongest single axis, so `fused = 1.0` again requires an axis that actually measured 1.0 (byte proof, per the content gate). |
| 3 | Preserve some literal information | 🟡 `ContentEvidence` now compares raw literal bytes positionally (literal preservation is the Type-2 discriminator). The fingerprint and token layers still collapse every literal to `__literal__`. |

## ✅ Checklist

The executable form of everything above. Ticked only when the assertion exists, is green, and the surface it pins actually ships — never on "code written". Branch: `worktree-fused-score-followups`.

### Verified already done (re-confirmed 17 Aug against the tree, not the ledger)

- [x] **#347** corpus gate boots — `corpus.yml` installs `typediagram@0.11.0`
- [x] **#301** determinism — ordered `snapshot_corpus`; `nest`/`jellyfin` `determinism` entries deleted from `known-failures.json`
- [x] **#343** sum-then-clamp — `bounded_fused()` at admission and rendering; `issue_343_sum_clamp_saturation.rs` green
- [x] **#342** ancestor excludes — `corpus_built_in_excluded`; `issue_342_scan_root_under_excluded_ancestor.rs` green
- [x] **#351** measured cosines discarded — `add_embedding_pairs` now calls `record_cosine` unconditionally; no quarantine panic remains
- [x] **#372** `f32` cosine drift — fixed by #384 (`cosine_from_parts`, `f64` accumulation); three width-sweep tests on `main`. **Issue still open — close it.**
- [x] No quarantine `panic!` remains anywhere in `crates/` — every mandated quarantine from this plan has landed its accurate replacement

### 1. Six skipped VSIX tests — outranks everything below

Contracts settled 17 Aug by reading the shipped code and every *passing*
test that constrains it. Each row records the fix that satisfies its own
assertions **and** the neighbouring green tests — the constraint that
made five of the six tractable and the sixth impossible.

**Whole VSIX suite after the fixes: 453 passing, 0 failing, 1 pending (D).**

- [x] **A** `live-bubble-fused.unit.test.ts` — `bestBubbleCluster` gated on a UI-local `signals.fused >= FUSED_THRESHOLD`. **Fixed:** `bubbleAdmits` applies two gates, not one — an act-now bucket (`identical` / `nearly_identical`) is the engine's verdict and needs no second opinion; everything below the act-now bands keeps the fused cutoff. Pinned in both directions by the green `a sub-threshold hint bucket stays off the live surface at the exact cutoff` (a `loosely_similar` hint at exactly 0.85 must show, at 0.84 must not) and `a demoted shape-only family never wins the bubble over a proven clone`.
- [x] **B1** `report-schema.unit.test.ts` — a content-gated rename was labelled `identical`. Fixed by §2 and re-stated against `resolveBucket`.
- [x] **B2** `report-schema.unit.test.ts` — a demoted shape-only family was promoted. Same fix, same re-statement.
- [x] **C** `live-bubble-fused.unit.test.ts` — `signalStrip` drew `structural | token_jaccard | embedding_cos`, so a verbatim copy and a proven rename both rendered `█▁█`. The strip had to stay **three bars wide** — `assert.equal(signalStrip(verbatim).length, 3)` in the test itself and `signalStrip clamps inputs to the bar range` in `bubble.unit.test.ts` both demand it — so a fourth bar was not available. **Fixed** by drawing **shape | semantic | confidence**: `max(structural, token_jaccard)`, `embedding_cos`, `fused`. Collapsing the first two is what the engine already says they are worth — *"`structural` and `token_jaccard` are two views of one normalised representation, so summing them says nothing beyond 'the shapes matched'"* ([`buckets.rs:304`](../../crates/deslop-core/src/buckets.rs#L304)) — and it buys the third slot for the only axis that separates the two.
- [x] **D** `severity.unit.test.ts` — **fixed. Not a product decision after all: a category error with a spec that already resolved it.** D was right that a demoted family must not wear act-now paint and wrong about which channel carries the paint. [SEVERITY-COLOR](../specs/severity.md#severity-color) defines *two* channels — colour from the **bucket**, glyph density from the **weight percentile** — and the VSIX drove both from the ranking, so the colour channel carried no bucket information at all. Shipped `resolveSeverity(bucket, percentile)` (the resolver that spec has always named), re-keyed the paint, re-stated D against colour. **The suite now has zero skips.** See below.
- [x] **E** `live-bubble.unit.test.ts` — **contract settled, then fixed.** The `byId.get(id) ?? cluster` fallback served two populations `bestBubbleCluster` could not tell apart, and each has a test: a cluster **the report never saw** may bubble on the probe's own evidence (green `deslop.bubble.dismissCluster …` renders `c-dismiss`, absent from the seeded snapshot, and requires it to bubble), while a cluster **a delta explicitly removed** must stay gone (E). The discriminator is retraction, not absence: `ReportStore` now records `clusters_removed` in a `retractedClusters` signal instead of dropping it, `setSnapshot` clears it (a full snapshot re-states the corpus on its own authority), a later `added`/`updated` un-retracts, and `bestBubbleCluster` filters on it before anything else.

#### Two impossible fixtures found while restoring them

Both were green tests asserting a state the engine cannot produce, and both were the last thing holding the fused-only gate in place:

- `inline mode renders the bubble decoration` staged `identical` at `fused 0.2`, and `render clears the bubble when no cluster passes the threshold` staged `identical` at `fused 0.5`. An `Identical` cluster is byte-proven: `content_gated_signals` returns its signals untouched, and `Identical` requires `structural >= 0.99` **and** `token_jaccard >= 0.99`, so its `bounded_fused` is `>= 0.99` by construction. Neither pairing can occur.
- Both fixtures now carry `loosely_similar` — the population the cutoff actually governs — and each grew an assertion rather than losing one: the first test now also proves a hint at exactly `FUSED_THRESHOLD` *does* render, so "the hint disappeared" cannot pass by hints being banned outright.

#### D — the spec picked the winner, and the loser was the *channel*, not the test

`severity.unit.test.ts` held two tests that could not both pass **so long as
severity was one channel**:

- `severity never brightens as rank worsens, at any confidence` (green) asserts the severity band is non-brightening **down the report**.
- `a demoted shape-only family is not painted with act-now severity` (D) hands it `shape-giant(0.31, structural_only)` at rank 1 and `proven(0.95, identical)` at rank 2, asserting `shape-giant` is quiet **and** `proven` is not.

Any function of `(rank, cluster)` that pays D its quiet answer for a demoted
rank-1 entry must, by monotonicity, give the same answer to every entry below
it — exactly what D's `notEqual` forbids. Re-ranking by confidence fails
monotonicity directly; a running floor satisfies monotonicity and fails D.

**The contradiction was real and the conclusion drawn from it was wrong.** It
is not evidence that the two tests state opposite contracts; it is evidence
that they are talking about *different channels*, and
[SEVERITY-COLOR](../specs/severity.md#severity-color) had already said so:

> **Colour** = the cluster's Deslop severity … **Glyph density** = the cluster's weight percentile.
> A faint identical clone therefore renders as a red `○`, while a high-impact loosely-similar cluster renders as a blue `●●`.

That sentence is D's fixture, inverted. The monotonicity test owns the
**percentile band** and is correct — a band that is a pure function of rank
*must* be monotonic down the ranking. D owns the **colour** and was
expressing itself in the band's vocabulary: the same category error as
[§2](#2--new-defect-found-17-aug--the-ui-re-derives-a-routing-it-cannot-see-the-inputs-to),
one channel answering a question only the other holds the inputs for.

**The defect D was pointing at was live, and worse than D described.** Both
`SEVERITY_COLOR[band]` call sites — the decoration underline/ruler
(`decorations/manager.ts:102`) and the live bubble's inline colour
(`bubble/live.ts:216`) — painted from the *rank band*. Measured on D's own
fixture, the colours came out **inverted**:

| cluster | bucket | rank band | old paint | new paint |
|---|---|---|---|---|
| `shape-giant` | `structural_only` | `worst` | **`primaryContainer` (crimson)** | `onSurfaceMuted` (grey) |
| `proven` | `identical` | `mid` | `tertiary` (blue) | `primaryContainer` (crimson) |

The content-gated family wore the colour that means *"Safe to extract — every
copy is the same"*, and the byte-proven clone one row below it wore blue.

**Shipped:**

- `types/report.ts` — `DeslopSeverity` (`error · warning · information · hint`) and `DESLOP_SEVERITIES`, orthogonal to `Severity` (the percentile band).
- `severity.ts` — `BUCKET_SEVERITY`, `deslopSeverityOf(bucket)`, `clusterSeverity(cluster)`, and **`resolveSeverity(bucket, percentile)`**, the resolver [SEVERITY-COLOR] has always named and which did not exist. It returns `{ level, band }` together so a caller cannot reach for one and render the other. `severityForRank` now delegates to `severityOf`, deleting a duplicated threshold ladder.
- `design.ts` — `DESLOP_SEVERITY_COLOR` keyed by level. Same four tokens; crimson is now earned by evidence rather than by position.
- `decorations/manager.ts` — decoration types keyed by level. An underline has no glyph, so it carries the bucket channel only. Colour no longer depends on the ranking, so `SeverityCache`/`severitiesFor`/`indexedSeverity` are gone from this file — the memoisation [VSIX-PERF] described exists because there is no longer a ranking to memoise.
- `bubble/live.ts` — inline colour from the level; the `SEVERITY_DOT` glyph stays on the band, so the bubble carries **both** facts at once.
- `severity.md` — `StructuralOnly → hint (muted)` added to [SEVERITY-DESLOP-MAP] (the table listed four of the five buckets); the status note now records that the map ships and that rank-percentile colour was a defect; [SEVERITY-COLOR] gains the unsatisfiability argument so the next reader does not re-derive it.

**Tests — nothing weakened, four assertions added and two tests added:**

- D restored and un-skipped, asserting on colour: `shape-giant → hint`, `proven → error`, the two differ, and their **colour tokens** differ. Its original fixture assertions are byte-for-byte intact.
- D **gained** the orthogonality that makes both contracts survivable: `shape-giant` still holds the `worst` band and still renders `●●`. Muted `●●` — loud about impact, quiet about kind.
- New `the colour channel is a pure function of the bucket, at every rank` — sweeps every bucket across six percentiles and asserts the level never moves while the band tracks the percentile. This is the invariant that makes D and monotonicity compatible.
- New `only act-now buckets may wear an act-now colour` — asserts `error` implies `isActNow(bucket)` and that **exactly one** bucket earns crimson, so a future remap cannot quietly hand it back.
- `design.unit.test.ts` gained `DESLOP_SEVERITY_COLOR covers every level and is a distinct token each` — no two levels may share a token, and `error` is pinned to `COLOR.primaryContainer`.

`severity.unit.test.ts` 14/14, `design.unit.test.ts` 9/9, `report-schema.unit.test.ts` 26/26 — **49 passing, zero skips.** The full extension-host suite could not be re-run in this session (VS Code was open and `vscode-test` requires an exclusive instance; killing it is prohibited). Typecheck and lint are clean, and no test asserts the internals removed from `decorations/manager.ts`.

### 2. 🛑 New defect found 17 Aug — the UI re-derives a routing it cannot see the inputs to

`clients/vscode/src/types/report.ts:223` `classifyCluster` claims byte-for-byte parity with `deslop-core::buckets::classify_signals` and does not have it:

| Signals | Engine | VSIX |
|---|---|---|
| `structural 0.10, token 0.96` | `loosely_similar` | `nearly_identical` |
| `structural 0.00, token 0.92` | `loosely_similar` | `nearly_identical` |

The engine gates on `structural >= 0.20`; the UI gates on `structural > 0.0` and carries an extra `structural <= 0.01 && token >= 0.9` arm. A hint is promoted to act-now on the flagship surface.

**Root cause — the divergence is a symptom, and closing the two rows would not fix it.** The engine does not route from the signal triple. [`report_bucket_kind`](../../crates/deslop-core/src/report_render.rs#L277) weighs four inputs: the **raw** triple, measured `ContentEvidence`, raw-source byte-equivalence, and the member spread. The triple the client receives is the *post-gate projection* of that decision — [`content_gated_signals`](../../crates/deslop-core/src/buckets.rs#L316) overwrites `token_jaccard` to `1.0` for a shape-identical near miss (#232) and rewrites `fused`. Running the engine's raw-signal table over rendered signals is therefore a category error, and it is the **same** root cause as B1 and B2:

- **B1** — a proven rename renders `structural 1.0 / token_jaccard 1.0` *because of* the #232 correction, so the triple reads `identical` and the user is told "Safe to extract — every copy is the same" about code whose identifiers all differ.
- **B2** — a demoted family's `structural >= 0.99` falls through to the act-now arm because `lacks_content_support` is invisible from the triple.
- **This defect** — two arms that were never in the engine's table at all.

**Fix: delete `classifyCluster`.** `resolveBucket` becomes the single routing surface and returns the engine's `cluster.bucket`; a report carrying no valid label yields `loosely_similar`, the only bucket whose action sentence makes no claim beyond "treat as a hint". The UI cannot re-derive this routing and every arm that tried has been a defect.

- [x] Failing test pinning both rows, watched failing for the real reason. Watched without a VS Code launch: the pre-fix `classifyCluster` was extracted verbatim from `8c5bd2ada` and evaluated beside `classify_signals` transcribed from `buckets.rs:356-370`. **4 of 5 rows diverge** — `(0.10, 0.96, 0)` and `(0.00, 0.92, 0)` route `nearly_identical` against the engine's `loosely_similar`; B1's `(1.0, 1.0, 0, fused 0.9)` routes `identical` against the engine's wire label `nearly_identical`; B2's `(1.0, 0.3, 0, fused 0.31)` routes `nearly_identical` against `structural_only`.
- [x] Delete `classifyCluster`; `resolveBucket` is the only routing surface (retires B1 and B2 with it). An unlabelled or unrecognised cluster resolves to `loosely_similar`, the only bucket whose action sentence claims nothing beyond "treat as a hint".
- [x] ⚠️ Two green tests asserted the defective contract and were **inverted, not weakened** — `classifyCluster nearly_identical on high jaccard + low structural` (`signals(0.0, 0.95, 0)`, which the engine calls `loosely_similar`) and `resolveBucket falls back to signals when v3 JSON has no bucket` (asserted a manufactured `identical`). Both are re-stated against `resolveBucket` with every assertion kept and the expected value corrected to agree with the engine; each carries an `⚠️ INVERTED` comment saying so.
- [x] Anti-regression assertion added: a cluster whose triple saturates on **every** axis but carries no engine label must still resolve to the hint bucket. Any surviving re-derivation answers `identical` there. The label-set table walk keeps its `structural 0.00 / token 0.95 → nearly_identical` row on purpose — a row whose triple and label disagree is exactly the row that catches a re-derivation coming back.

### 3. #344 — carry the confidence to every consumer

- [ ] `agreement` / `rename_consistency` / `literal_fraction` onto `ReportSignals` in [`live-ipc.td`](../models/live-ipc.td), regenerate (never hand-write). **Population point located:** `impl From<PairScore> for ReportSignals` ([`report.rs:104`](../../crates/deslop-core/src/report.rs#L104)) converts the raw triple *before* content is measured and cannot carry them; [`content_gated_signals`](../../crates/deslop-core/src/buckets.rs#L316) already holds the `ContentEvidence` and is the one place every rendered cluster passes through. It must stamp the three fields on **both** of its paths — today it early-returns unchanged for `Identical` and for non-saturating shapes.
- [ ] `resolveBucket` trusts the engine's `cluster.bucket` unconditionally — **superseded and strengthened**: `classifyCluster` is deleted outright rather than corrected, so there is no second routing table left to drift. Tracked in §2.
- [x] `severity.ts` — **done (Defect D).** Colour now resolves from the engine's bucket, which is the content gate's own verdict; the percentile band keeps the ranking channel. Does not need the three new wire fields.
- [ ] `render/text.rs` — prints no signals at all
- [ ] `deslop-lsp` diagnostics / code lens — no confidence anywhere
- [ ] `refactor/preconditions.rs` — bucket pre-filter + byte proof only
- [ ] Render the three new fields everywhere `fused` renders (HTML footer, Markdown, VSIX `SignalStrip`, `HelpBubble`)
- [ ] Restore the 17 fixtures #341 softened from maximal to partial renames
- [x] Pair admission — fixed by #343
- [ ] `metrics.duplication_percent` / exit-code gate — **delegated** to [weighted-metrics-plan.md](weighted-metrics-plan.md) under [METRICS-REPO-WEIGHTED]; not this plan's work

### 4. #345 — doc drift

- [x] `REPORTING-CONTEXT.md:100` — the admission bar and the rendered confidence are now separated by name, and agents are told to filter on `bucket`, never on `fused >= FUSED_THRESHOLD`. That instruction is not cosmetic: the shipped VSIX had exactly that bug (defect A).
- [x] `mcp.md:248` — documents report order (confidence-scaled ranking weight) and says why it is not `fused`.
- [x] `--embeddings` default discrepancy — `fusion.md` states the shipped default (`off`, `nomic-embed-text`) with the reason (no reachable Ollama on a first run) and names the recall cost rather than hiding it.
- [x] `fused_bounded_max` / `type2_recall` — added to `corpus/known-failures.json` **and implemented** in `crates/deslop-test-support/src/corpus_confidence.rs`, wired into `corpus_repos.rs::gate` + `GATE_CHECKS`, 14 unit tests green. Ids without checks behind them would be placeholders in the one file whose purpose is honesty about what is verified. Both predicates were rewritten after the regression audit — see the quarantine plan's R6 row for why the originals were unsound in both directions.
- [x] `fusion.md` + `SPEC.md` reconciled
- [x] Both public `how-it-works.md` pages (EN + ZH) still taught `clamp(structural + token_jaccard + embedding_cos, 0, 1)` and linked `PairScore::fused` as shipped code — the quarantined arm, documented publicly. Both now describe the bounded max.

### 5. Fused-related open bugs

- [ ] **#339** F# `token_jaccard` from the issue-86 fallback signature — byte-offset luck, not token evidence. Named in the ledger above; confirmed empirically during #343
- [ ] **#71 #79 #103 #283 #284 #285** — unblocked by #343; each needs its own verification against the bounded fusion before closing
- [ ] **#336** numeric array literals rank #1 on `dotnet/fsharp` — data-table classification is Dart-only

### 6. Issue close-outs (evidence first, never on a run CI has not seen)

- [ ] `workflow_dispatch` the corpus gate; reconcile `known-failures.json` against its first real run
- [ ] Close **#301**, **#331**, **#336** only on a green corpus run
- [ ] Close **#343**, **#342**, **#351**, **#372** — fixed and pinned

