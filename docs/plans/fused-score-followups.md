# Fused confidence — follow-ups for the next release

What remains of the `[FUSION-CONTENT-GATE]` audit (formerly
`fused-score-rollout.md`) after the `fusedhardening` branch shipped, plus
the issues that came out of the v0.31.0 release triage — two of which
(#347, #342) turned out to gate the accuracy work described here. The
standing requirements live in
[`docs/root-cause-fusion.md`](../root-cause-fusion.md); the mechanism shipped
in this release is specified in
[`docs/specs/fusion.md`](../specs/fusion.md) under `[FUSION-CONTENT-GATE]`
and pinned by `fused_golden_bands.rs` and `fused_golden_invariants.rs`.

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

**The recall change — the user-visible headline.** A maximal Type-2 rename
whose anchors prove its mapping now reaches the act-now `nearly_identical`
bucket instead of being demoted to "same shape, different content". Seven
existing fixture families moved bucket, across five suites: `csharp-small`,
`javascript-small`, `js-regex`, `js-destructuring`, `js-type2-pipeline`,
`ts-type3-reorder`, `jsx-js-components`. The gate discriminates rather than
blanket-promoting — anchor-poor renames (`js-classes`, `js-async`,
`js-template-literals`, `js-optional-chaining`, both `*-type2-loop` pairs)
still route `structural_only`, and the shape-only families that motivated
#341 still rank last. Renamed clones that a Type-1-only reading would miss
are the common case in real repositories, so this is where the release's
accuracy gain lands.

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

**Six assertions-in-waiting are currently `test.skip`-ed. They are correct;
the code they test is wrong.** They were skipped under an explicit owner
mandate to unblock this release — a deliberate, one-time exception to the
`CLAUDE.md` rule "never delete a failing test, never skip one". Nothing
about them is negotiable on restore: **un-skip, do not weaken, do not
delete.** Each carries a `🛑 SKIPPED — DEFECT <x>` comment at its
definition pointing back to this section.

Every one of these defects is **live in the shipped VSIX today**. Skipping
removed the *signal*, not the bug.

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
| **#342** | Bug | `built_in_excluded` matches path components *above* the scan root, so a repo living under any folder named `dist`/`build`/`target`/`vendor`/`node_modules` analyses as zero files: clean report, exit code 0. See the section below — its pinning test does not exist. |

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

## 🛑 #342 — a total false negative with no test pinning it

Issue #342 states:

> A failing E2E test
> (`crates/deslop/tests/issue_<this>_scan_root_under_excluded_ancestor.rs`)
> accompanies this issue: it scans a clone-bearing repo seeded under
> `<tmp>/dist/` and asserts `files_analysed == 2` plus the reported clone
> pair. It is red on `main` for exactly the defect above.

**That test is not in the tree.** All 69 files under `crates/deslop/tests/`
were searched for the filename, `excluded_ancestor`, `scan_root_under`, and
`BUILTIN_EXCLUDE_COMPONENTS`. None exists. The unresolved placeholder in the
issue's own filename (`issue_<this>_…`) is the tell: it was described, never
written.

So the most severe defect on the board has **no assertion pinning it**,
while the tracker claims it does — and the defect is the quietest failure
mode this product has. `built_in_excluded`
(`crates/deslop-core/src/config.rs`) tests every component of the absolute
discovered path, including components above the scan root, so a user whose
checkout happens to sit under `~/build/` or `~/dist/` gets
`files_analysed: 0`, `clusters: []`, exit code 0, and concludes their code
has no duplication. Nothing in CI would notice this returning.

**Write the test first and watch it fail** — per `CLAUDE.md`, the assertion
outranks the fix. Roughly twenty lines:

- Seed two files containing a genuine cross-file clone under
  `<tmp>/dist/innocent-repo/src/`; run with `--no-incremental --min-nodes 8
  --embeddings off`; assert `files_analysed == 2`, a non-empty cluster list,
  and that the reported cluster spans both files.
- Then seed the **same** corpus under `<tmp>/innocent-repo/` and assert the
  two reports agree. The delta between the two roots *is* the bug, and
  asserting the equivalence is what stops a future change to the exclusion
  list from silently reintroducing it — a single-root test would not.

The fix the issue already describes: make only components **below** the scan
root eligible for built-in exclusion, matching
`scan_root_contains_component_pair`, which encodes exactly this principle
for the report-hide pairs ("the user intentionally asked to analyse that
corpus") and which `built_in_excluded` has no equivalent of.

## Open work, in order

### 1. #347 — make the corpus gate boot

Everything below is measured by an instrument that has never been switched
on. Add the `typediagram@0.11.0` install step to `corpus.yml` to match the
four jobs in `ci.yml` (keeping the pin in sync per the dependency-sync
rule), `workflow_dispatch` the gate, and reconcile
`corpus/known-failures.json` against its first real run — that baseline has
never been confirmed by a passing run, so entries may be stale in either
direction. The workflow's own contract, "anything NOT recorded in
`known-failures.json` fails this workflow — that is the regression gate",
has been vacuous since it was written.

### 2. Close #301 — determinism

`nest` and `jellyfin` remain baselined as `determinism` failures in
`corpus/known-failures.json`. While identical runs disagree, no
before/after accuracy measurement of the content gate is trustworthy. Close
this before tuning any threshold introduced below. Blocked by #347: the
determinism checks are corpus checks, so the evidence for closing this
cannot exist until the gate runs.

### 3. #343 — replace sum-then-clamp saturation

`PairScore::fused()` still sums three correlated signals and clamps: a
cluster at `structural = 0.62, token_jaccard = 0.80` sums to 1.42, clamps
to 1.00, and is never gated because neither component crossed a corner
threshold (`structural ≥ 0.99` / `token ≥ 0.95`). Replace the sum with a
bounded fusion over independent axes — `shape = max(structural, token)`,
content, embedding — and make the gate the *definition* of `fused` rather
than a render-time correction. `fused_golden_invariants.rs`
(`fused == 1.0` requires a byte-identical pair) is the tripwire; add a
mixed-band fixture the moment the arithmetic changes.

### 4. #344 — carry the confidence to every consumer

Surfaces still running on the pre-gate world:

| Surface | Today |
|---|---|
| Pair admission to a cluster (`pair.rs`) | Raw `structural + token + embedding`, clamped |
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
- `fusion.md` carries two fusion rules (`[FUSION-STRATEGY-MAX-SUM]` and
  `[FUSION-CONTENT-GATE]`) that disagree about what `fused` is — reconcile
  when #343 lands. `SPEC.md` still marks the clamp as the strategy.
- `corpus/known-failures.json` has no confidence check id — add a
  `fused_spread` / `type2_recall` check so the real-repository gate can
  catch saturation and rename recall at scale. Blocked by #347: adding a
  check to a gate that cannot compile is a no-op.

## Requirement status ([`root-cause-fusion.md`](../root-cause-fusion.md))

| # | Requirement | Status |
|---|---|---|
| 1 | Give the ensemble an independent member | 🟡 Content evidence is independent and now steers routing, fusion, and ranking — but it remains a render-stage gate rather than an ensemble member, and the semantic signal is still off by default. |
| 2 | Stop clamping away the top of the range | 🔴 `PairScore::fused()` unchanged — #343. |
| 3 | Preserve some literal information | 🟡 `ContentEvidence` now compares raw literal bytes positionally (literal preservation is the Type-2 discriminator). The fingerprint and token layers still collapse every literal to `__literal__`. |
