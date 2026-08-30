# Clone Type Taxonomy

## [CLONE-BUCKETS-NORTH-STAR] Two audiences, three surface classes

Deslop writes for **two** readers (humans and AI agents) across **three** classes of surface. Every output decision flows from which class a given surface falls into:

- **Pure-visual surfaces** (HTML report body, VS Code webview, live bubble decoration) — **humans only**. Agents rarely scrape rendered HTML. Plain-English labels; no `Type-N`, no signal triples, no enum names in prose. Drift between the HTML card and a webview is a bug.
- **Shared-text surfaces** (CLI stderr summary, LSP diagnostics `message`, VS Code Problems window, hover tooltips) — **humans first, AI secondary**. These are textual and agents regularly scrape them. Use the **hybrid** form: plain-English title first, academic taxonomy in brackets. E.g. `"Identical code [Type-1/2]"`, `"Same behavior, different code [Type-4, AI match]"`. The human reads the plain prefix; the agent parses the bracketed suffix.
- **AI-only surfaces** (JSON `report.json` fields `interpretation` / `action_hints` / `schema_doc`, MCP tool responses, machine-readable logs) — **agents only**. Precise and technical: plain title + bracketed taxonomy + signal context + routing rationale. Drop nothing — agent prompts in the wild depend on `Type-N`.

**User mandate (verbatim):** *"Shoot for human readable, but include technical terms in brackets for the ai"* — applies to shared-text surfaces. *"Diagnostics that appear as CLI output or in the vscode problems window are primarily for humans. But, AI will read these too."* — why shared-text is hybrid, not split.

All three classes point at the **same** bucket identity (the Rust enum variant). Only the *rendered text* differs. [CLONE-BUCKETS-DUAL-LABEL] formalises which class each concrete surface belongs to — never mix classes on a single surface.

## [CLONE-BUCKETS] Canonical buckets (single source of truth)

**This table is the canonical definition of every clone bucket Deslop reports. Every renderer — HTML, CLI, VS Code, MCP — must agree with it. If a surface disagrees, the surface is wrong, not the table.**

| Bucket (enum)     | Plain title (pure-visual)                       | Hybrid title (shared-text)                          | Evidence sentence                                                                              | Colour band        | Taxonomy ref                |
|-------------------|-------------------------------------------------|-----------------------------------------------------|------------------------------------------------------------------------------------------------|--------------------|-----------------------------|
| `Identical`       | **Identical code**                              | `Identical code [Type-1/2]`                         | Every copy is the same after normalisation.                                                    | green / crimson    | Type-1, Type-2              |
| `NearlyIdentical` | **Nearly identical code**                       | `Nearly identical code [Type-3]`                    | The copies differ in small ways.                                                               | yellow / blue      | Type-3                      |
| `StructuralOnly`  | **Same shape, different content**               | `Same shape, different content [structural-only]`   | Only the code shape matches; the measured content does not agree. Commonly sibling boilerplate. | muted / outline    | structural-only (unverified Type-2/3 candidate) |
| `LooselySimilar`  | **Loosely similar code**                        | `Loosely similar code [weak LSH]`                   | Loose textual overlap, with no other axis corroborating it.                                    | neutral            | weak LSH-only (sub-Type-3)  |
| `SameBehavior`    | **Same behavior, different code** *(AI match)*  | `Same behavior, different code [Type-4, AI match]`  | The embedding pass matched these as the same behaviour written two ways; no deterministic axis corroborates. | purple / cyan      | Type-4                      |

Each sentence states what was measured and stops there ([PRINCIPLES-REPORT-NOT-DICTATE](principles.md#principles-report-not-dictate)).

`green / crimson`, `yellow / blue` etc. are light-theme / dark-theme pairs. Exact CSS variables live alongside the renderer in `crates/deslop-core/src/render/html.rs`; this table governs which colour family, not the specific hex.

### [CLONE-BUCKETS-DUAL-LABEL] Dual-labelling policy

Every bucket has **one bucket identity** (the enum variant) and **three rendered forms** pointing at it: a plain title, a hybrid title with bracketed taxonomy, and a full agent-facing sentence. Surfaces pick their form by **class**, per [CLONE-BUCKETS-NORTH-STAR]:

- **Pure-visual surface** → Plain title + Action sentence (no `Type-N`, no enum names, no signal triples).
- **Shared-text surface** → Hybrid title (`"Plain title [Type-N]"`) + Action sentence. Plain prose first, bracketed taxonomy suffix for AI scrapers.
- **AI-only surface** → Plain title + Action sentence + `Type-N` reference + signal context. Precision over brevity.

Surface routing:

| Surface                                               | Class          | Rendered form                                    |
|-------------------------------------------------------|----------------|--------------------------------------------------|
| HTML report card title + action                       | Pure-visual    | **Plain title** + Action sentence                |
| VS Code live bubble decoration                        | Pure-visual    | **Plain title**                                  |
| VS Code cluster detail / report webviews              | Pure-visual    | **Plain title** + Action sentence                |
| VS Code tree view node labels                         | Pure-visual    | **Plain title**                                  |
| CLI stderr summary row                                | Shared-text    | **Hybrid title**                                 |
| LSP `diagnostic.message`                              | Shared-text    | **Hybrid title** + Evidence sentence               |
| VS Code Problems panel (mirrors LSP)                  | Shared-text    | **Hybrid title** + Evidence sentence               |
| LSP hover tooltip                                     | Shared-text    | **Hybrid title** + Evidence sentence               |
| JSON `cluster.interpretation`                         | AI-only        | **Plain title** + Evidence sentence + `Type-N`     |
| JSON `action_hints[*].recommendation`                 | AI-only        | **Plain title** + Evidence sentence + `Type-N` |
| `REPORTING-CONTEXT.md` (`schema_doc`)                 | AI-only        | Full table with all three forms                  |
| MCP tool descriptions, resources                      | AI-only        | **Plain title** + Evidence sentence + `Type-N`     |
| Source code identifiers, spec IDs, tests              | n/a (dev)      | **Enum variant** (`ClusterKind::Identical` etc.) |

**Rules:**

1. **The enum is the identity.** `ClusterKind::Identical`, `ClusterKind::NearlyIdentical`, `ClusterKind::StructuralOnly`, `ClusterKind::LooselySimilar`, `ClusterKind::SameBehavior`. These names appear in code, tests, and CSS class suffixes. Never `Exact`, never `Near`, never `Weak`, never `Semantic`.
2. **Pure-visual is pure.** HTML card, bubble, webviews, tree view — developers see the plain title and evidence sentence, never `Type-N`. If you feel pulled toward a "technical mode" toggle on a pure-visual surface, the toggle is the bug.
3. **Shared-text is hybrid.** CLI stderr, LSP diagnostics, Problems panel, hover — plain prose prefix so humans read it naturally, bracketed `Type-N` suffix so AI scrapers can still classify. `"Identical code [Type-1/2]"` on one line; `"Same behavior, different code [Type-4, AI match]"` on another.
4. **AI-only retains everything.** JSON `interpretation`, `action_hints`, `schema_doc`, MCP responses keep the full plain-title + evidence-sentence + `Type-N` form. Dropping `Type-N` would break agent prompts already in the wild.
5. **`SameBehavior` carries the `(AI match)` badge.** Shown as `"Same behavior, different code (AI match)"` on pure-visual surfaces and `"Same behavior, different code [Type-4, AI match]"` on shared-text surfaces. It is the AI-specific value-add; users deserve to know which clusters came from the embedding pass vs the deterministic pipeline.
6. **One helper, three forms.** A single function in `deslop-core::buckets` returns the `(plain_title, hybrid_title, evidence_sentence, taxonomy_label, css_suffix, ai_match)` sextuple keyed by `ClusterKind`. Every renderer pulls the form it needs from that struct. Drift is a bug.

### [CLONE-BUCKETS-ROUTING] Signal-to-bucket routing

The canonical signal thresholds that map a cluster's `(structural, token_jaccard, embedding_cos)` triple onto a bucket live in `deslop-core::buckets::classify_signals` and `deslop-core::report_render::report_bucket_kind` (the byte-proof up/downgrades of [CLONE-BUCKETS-IDENTICAL]). Both must match this table:

| Condition (evaluated top-down)                                 | Bucket            |
|----------------------------------------------------------------|-------------------|
| `structural ≥ 0.99 ∧ token_jaccard ≥ 0.99`                     | `Identical`       |
| `embedding_cos ≥ 0.80 ∧ structural < 0.5`                      | `SameBehavior`    |
| `structural ≥ 0.99 ∧ token_jaccard < 0.05 ∧ embedding_cos < 0.05` | `StructuralOnly` |
| `token_jaccard ≥ 0.90` (row 4)                                  | `NearlyIdentical` |
| `structural ≥ 0.75 ∧ (token_jaccard ≥ 0.65 ∨ embedding_cos ≥ 0.80)` (row 4b) | `NearlyIdentical` |
| `structural ≥ 0.99`                                            | `NearlyIdentical` |
| else                                                           | `LooselySimilar`  |

`StructuralOnly` is tested **before** the near-miss rows so a shape-only triple never absorbs into `NearlyIdentical` ([RANK-STRUCTURAL-ONLY], issues #134/#154/#197).

**Row 4b is the shared-subtree near-miss** ([FUSION-SHARED-SUBTREE](fusion.md#fusion-shared-subtree), #408). `structural` is measured ordered subtree overlap, not Merkle equality, so a Type-3 clone whose one inserted statement rehashes every ancestor still measures 0.84–0.91 against the larger method. Shape must be corroborated by an axis that does not read the normalised tree — normalisation makes scaffolding Merkle-identical across unrelated files — but **either** measured axis qualifies. Requiring the token axis specifically was a hole: a pair measuring `structural = 0.91` *and* `embedding_cos = 0.91` fell to `loosely_similar`, which the renderer hides, purely because its `token_jaccard` was 0.55. A `while` loop and a `for` loop over one accumulator chain are exactly that case — identical statements, different loop keyword, so the k-gram set diverges far more than either the shape or the meaning does.

Because the embedding axis can now carry a cluster into an act-now bucket by this second door, [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH](#clone-noise-embedding-role-mismatch) applies to it as well as to row 2. That gate exists because embedding evidence can pair role-incompatible code — a reader against a writer the model scores alike — and it was keyed on the `same_behavior` bucket only because that was the sole route such evidence had. Row 4 correspondingly lost its old `structural ≤ 0.01` leg: that leg predates the measurement, when any non-zero value meant a Merkle anchor, and additional shape evidence must never *hide* a cluster the token axis already carries. The spread and unmeasured demotions below still apply to token-carried clusters whose overlap stays under the row-4b floor.

Row 2's `0.80` is `deslop-core::pair::EMBEDDING_SUPPORT_FLOOR` — the single operating point at which a measured cosine counts as the embedding pass *vouching for* a cluster rather than merely having measured one. The same line admits a pair as an ANN candidate at all (`embedding/pairs.rs`) and releases a shape-saturating cluster from the content gate (`report_render::route_shape_identical`). It is one number because it answers one question, and it is named rather than written inline in each of those places: the functions this table binds — `buckets::classify_signals` and `report_render::report_bucket_kind` — are required above to agree, and a literal repeated per call site is exactly how they would silently stop agreeing. Do not confuse it with `STRUCTURAL_ONLY_MAX_SUPPORT` (0.05), which is a ceiling *below* which a signal counts as **absent**; reading that ceiling as a support floor is the defect gh #356 fixed.

**Two routes reach it, and a consumer that knows only the first will misread the second.**

1. **Evidence-free** (`is_structural_only_signals`) — token and embedding support both below `deslop-core::buckets::STRUCTURAL_ONLY_MAX_SUPPORT` (0.05, tolerating MinHash collision noise). Shape is the only positive signal; the #197 REST settings family is the canonical case.
2. **Content-gated** (`lacks_content_support`, [FUSION-CONTENT-GATE]) — the deterministic signals saturate *by construction* and therefore prove nothing about content. The token LSH pass hashes the same normalised representation the structural pass does, so a shape-saturating family reads `structural = 1.00, token_jaccard = 1.00` while its raw collapsed leaves disagree and no corroborated substitution explains them. `ContentEvidence::support()` below `CONTENT_SUPPORT_FLOOR` routes it here.

**Row 4 is routed on spread, not on content.** It reaches an act-now bucket on token overlap *alone*, and two shapes of cluster carry that estimate without earning the verdict. A **cross-file spread** (3+ members over 3+ files) is the #134 scaffolding pattern arriving through the token door instead of the structural one: six distinct Flutter widgets whose `build` bodies share nothing measure `structural = 0.00, token_jaccard = 0.93` over whole-file spans, because the framework-mandated declaration is most of each file (#331). An **unmeasured** cluster is one the content pass could not compare at all — the anchored routes may take one on trust because their Merkle equality is itself proof, but row 4 has no such signal, so unmeasured there means nothing is known (#108's `structural = 0.00, token_jaccard = 0.96` JSON-schema pair). Both demote to `LooselySimilar`, which the renderer hides — never `StructuralOnly`, which would claim a shape match `structural = 0.00` says does not exist. A **single-file** row-4 cluster is *not* demoted on spread — a file count is not evidence about what a member is — and instead faces the same [RANK-STRUCTURAL-ONLY-FORWARDING] proof the `StructuralOnly` door runs ([pipeline.md](pipeline.md#rank-structural-only-forwarding)): the report's noise gate consults `is_single_file_declaration_family` for the anchor-free near-miss exactly as it does for the shape-only family, judged on the raw signal triple the routing itself used. The #197 meilisearch-dart REST settings surface is one family whichever door it arrives through — the offset-invariant sibling-window signatures ([FUSION-SIGNALS-THREE-LAYER], #339) inverted its triple from `structural = 1.00, token_jaccard = 0.00` to `structural = 0.00, token_jaccard = 0.91`, and gating the proof on the `structural_only` label alone let the very wrappers it was written to convict ride row 4 into the act-now tier at 50% of `index.dart`. The proof is the discriminator, not the bucket: a genuine in-file Type-3 copy whose body binds locals, loops or branches fails the forwarding proof and stays visible, so recall pays nothing.

A measured *pair* is left alone even at low agreement, because that is the renamed Type-3 clone the content gate is structurally unable to vouch for ([fusion.md §FUSION-CONTENT-GATE](fusion.md#fusion-content-gate)). Both directions are pinned together: `crates/deslop/tests/lsh_only_nearmiss_recall.rs` asserts a genuine LSH-only pair keeps `fused ≥ 0.85` and `cli/detection.rs::detects_type3_clone_in_csharp_fixture` asserts the renamed C# pair still surfaces, while `crates/deslop/tests/issue_331_336_shape_only_saturation.rs`, `crates/deslop-core/tests/issue_98_99_108_120_122_thresholds.rs` and `crates/deslop/tests/dart_issue_197_single_file_structural_only.rs` assert the scaffold family, the unmeasured noise and the single-file sibling-method family do not.

Route 2 is why `token_jaccard < 0.05` is **not** a property of the bucket: the `js-classes` delegating-method pair lands here with `structural = 1.00, token_jaccard = 1.00`, because the token pass is echoing the structural pass's own normalised representation rather than measuring anything new. What routes a cluster through this door is the content gate refusing to vouch for it — never a token reading, and never a scarcity of literal anchors. A corroborated Type-2 rename with the identical triple is promoted instead (`js-type2-loop` / `ts-type2-loop`, `crates/deslop/tests/js_ts_clone_buckets.rs`); demoting one is the false negative `[REPAIR-RENAME-ANCHOR-MASS]` (#405) removed. The invariant that does hold on both routes is that a `StructuralOnly` cluster carries no semantic support and stays below the act-now routing floor; `common::signals::assert_structural_only_contract` is the shared assertion.

Two report-render refinements sit on top of the raw signal routing: a structural-only cluster whose raw source slices are byte-equivalent is **upgraded to `Identical`** (byte proof beats the unscored token signal, [CLONE-BUCKETS]), and a cross-file ≥3-member/≥3-file scaffolding spread is demoted to `LooselySimilar` (#134) which the renderer hides. Ranking: `StructuralOnly` clusters are weight-demoted by default via the `[ranking] structural_only` policy ([RANK-STRUCTURAL-ONLY]).

`SameBehavior` is tested **before** `NearlyIdentical` so a strong AI signal on two syntactically divergent implementations gets the AI label rather than being absorbed into near-miss. It is only reachable when the embedding pass ran (`--embeddings=auto|required`). When the pass is disabled, `embedding_cos` is `0.00` across the whole report and the `SameBehavior` branch is dead.

**Literal-family clusters bypass signal routing.** Clusters produced by the value-level join
([LITERAL-CATEGORY], literals.md) carry no similarity signals; their `bucket` is stamped from
raw-text equality of the **matched-value byte ranges** (literal tokens / declaration value ranges —
never whole-occurrence ranges) — all compared texts byte-equal → `Identical`, else
`NearlyIdentical`; `constant_drift` is always `NearlyIdentical` by construction. The wire `bucket`
field is authoritative. They never land in `StructuralOnly` / `LooselySimilar` / `SameBehavior`
([LITERAL-WIRE]).

### [CLONE-BUCKETS-IDENTICAL] Byte-equivalence proof for the Identical bucket
The `Identical` bucket asserts that every copy is byte-for-byte the same, so it is
awarded only on raw-source proof, never on the signal triple alone — structural
normalisation collapses identifiers and literals, so two snippets that differ in
routes, handlers, or policy literals still reach `structural=1.00,
token_jaccard=1.00`. `report_bucket_kind` is the single source of truth for the
downgrade/upgrade: a cluster routed to `Identical` whose raw slices are *not*
byte-equivalent (after collapsing ASCII whitespace runs) is downgraded to
`NearlyIdentical`, and conversely a `NearlyIdentical` or `StructuralOnly` cluster
at `structural ≥ 0.99` whose raw slices *are* byte-equivalent is upgraded to
`Identical` ([CLONE-BUCKETS-ROUTING]) — byte proof beats the unscored token
signal. When a member's source bytes are unavailable the cluster cannot prove
equivalence and takes the downgrade, so every `Identical` label downstream
(including the agent-facing `interpret` summary) reflects byte-equivalent source.

## [CLONE-CATEGORY-REGISTRY] Clone categories (single source of truth)

> **Status: partially shipped.** The `CloneCategory` enum today ships `Logic` and
> `DataTable` only. The five literal-family categories (`MagicLiteral`,
> `ShadowedConstant`, `ConstantDuplicate`, `ConstantDrift`, `ConstantAlias`) and
> the `CloneCategory::all()`-derived facet/schema enums land with the planned
> literal feature ([LITERAL-CATEGORY], literals.md); the table below is the target
> registry.

The **category** axis is orthogonal to the bucket: bucket answers *"how textually similar?"*,
category answers *"what kind of repetition?"*. The Rust `CloneCategory` enum is the identity; this
table is canonical for every renderer, schema enum, and facet surface (all derived from
`CloneCategory::all()` — never hand-listed, [FACET-MODEL]). Ranking policy per category:
[RANK-CATEGORY], [RANK-LITERAL-FAMILY].

| Category (enum) | Wire label | Chip (pure-visual) | Action sentence | Defined by |
|---|---|---|---|---|
| `Logic` | `logic` | — (default, no chip) | Ordinary duplicated code — extract the shared implementation. | [RANK-CATEGORY] |
| `DataTable` | `data` | data table | Consider a builder with default args, or move the rows to a JSON/CSV/asset. | [RANK-CATEGORY] |
| `MagicLiteral` | `magic_literal` | magic value | The same literal value is repeated inline — name it once as a constant. | [LITERAL-CATEGORY-MAGIC] |
| `ShadowedConstant` | `shadowed_constant` | use the existing constant | A constant already names this value — replace the inline literals with it. | [LITERAL-CATEGORY-SHADOWED] |
| `ConstantDuplicate` | `constant_duplicate` | duplicate constant | The same constant is declared in several places — hoist one shared declaration. | [LITERAL-CATEGORY-CONST-DUP] |
| `ConstantDrift` | `constant_drift` | conflicting values | Same constant name resolves to different values — confirm which is correct and consolidate. | [LITERAL-CATEGORY-CONST-DRIFT] |
| `ConstantAlias` | `constant_alias` | same value, different names | One value lives under several names — pick the canonical name and delete the rest. | [LITERAL-CATEGORY-CONST-ALIAS] |

Chips and evidence sentences come from the same one-helper pattern as the bucket sextuple (rule 6 of
[CLONE-BUCKETS-DUAL-LABEL]): a single function in `deslop-core::clone_category` keyed by variant;
every surface pulls from it. The **wire label and chip columns are normative**; the action
sentences above are paraphrases whose exact copy lives in `deslop-core::clone_category` (the same
deferral [CLONE-BUCKETS] makes for hex colours). A cluster carries exactly one category; the five
literal-family categories are produced only by the value-level join (literals.md), never by signal
routing.

## [CLONE-TYPE-TAXONOMY] Academic ground rules (reference only)

The `Type-1 → Type-4` taxonomy is standard in clone-detection literature (Bellon/Koschke, Roy/Cordy 2007). Deslop surfaces it verbatim on **AI-only** surfaces and in **bracketed form on shared-text** surfaces (see [CLONE-BUCKETS-DUAL-LABEL]). It never appears on **pure-visual** surfaces (HTML card, VS Code webview, bubble decoration).

- **Type-1** — identical code, ignoring whitespace/comments. Maps to `Identical`.
- **Type-2** — identical up to renaming of identifiers/literals/types. Maps to `Identical` when raw source slices are byte-equivalent after whitespace folding; an *unverified* shape-only candidate (no token/semantic support, bytes differ) maps to `StructuralOnly`.
- **Type-3** — Type-2 + added/removed/modified statements ("near-miss" clones). Maps to `NearlyIdentical`, or `LooselySimilar` when the signal is weak (LSH-only, sub-threshold).
- **Type-4** — semantically equivalent, syntactically different (same behavior, different structure/algorithm). Maps to `SameBehavior`.

Recent work reframes Type-4 specifically as *"code segments deliver identical functionality through syntactically distinct implementations, such as differing algorithmic approaches or data structure choices that yield substantially varied program structures."* ([PMC — Semantic code clone detection via hybrid IR + BiLSTM, 2025](https://pmc.ncbi.nlm.nih.gov/articles/PMC12818651/))

**Implication for Deslop:** Types 1–3 are the sweet spot for the deterministic static pipeline. Type-4 is handled by the optional embedding pass (P5) and surfaces as `SameBehavior` under [CLONE-BUCKETS]; absent the embedding pass the `SameBehavior` branch is empty but the other three remain fully populated.
