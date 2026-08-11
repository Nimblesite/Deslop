# Fused confidence — follow-ups for the next release

What remains of the `[FUSION-CONTENT-GATE]` audit (formerly
`fused-score-rollout.md`) after the `fusedhardening` branch shipped. The
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

## Open work, in order

### 1. Close #301 — determinism first

`nest` and `jellyfin` remain baselined as `determinism` failures in
`corpus/known-failures.json`. While identical runs disagree, no
before/after accuracy measurement of the content gate is trustworthy. Close
this before tuning any threshold introduced below.

### 2. #343 — replace sum-then-clamp saturation

`PairScore::fused()` still sums three correlated signals and clamps: a
cluster at `structural = 0.62, token_jaccard = 0.80` sums to 1.42, clamps
to 1.00, and is never gated because neither component crossed a corner
threshold (`structural ≥ 0.99` / `token ≥ 0.95`). Replace the sum with a
bounded fusion over independent axes — `shape = max(structural, token)`,
content, embedding — and make the gate the *definition* of `fused` rather
than a render-time correction. `fused_golden_invariants.rs`
(`fused == 1.0` requires a byte-identical pair) is the tripwire; add a
mixed-band fixture the moment the arithmetic changes.

### 3. #344 — carry the confidence to every consumer

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

### 4. #345 — doc drift

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
  catch saturation and rename recall at scale.

## Requirement status ([`root-cause-fusion.md`](../root-cause-fusion.md))

| # | Requirement | Status |
|---|---|---|
| 1 | Give the ensemble an independent member | 🟡 Content evidence is independent and now steers routing, fusion, and ranking — but it remains a render-stage gate rather than an ensemble member, and the semantic signal is still off by default. |
| 2 | Stop clamping away the top of the range | 🔴 `PairScore::fused()` unchanged — #343. |
| 3 | Preserve some literal information | 🟡 `ContentEvidence` now compares raw literal bytes positionally (literal preservation is the Type-2 discriminator). The fingerprint and token layers still collapse every literal to `__literal__`. |
