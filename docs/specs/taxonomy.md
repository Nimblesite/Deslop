# Code similarity categories

## [CLONE-BUCKETS-NORTH-STAR] What the four categories mean

| Category  |  What to expect  |  Research taxonomy  |  Counts as duplication? |
| --- | --- | --- | --- |
| **Identical code**  |  The same source text, apart from whitespace.  |  **Type-1** exact copies. Research also allows comment differences; Deslop's text-identity rule is stricter.  |  Yes |
| **Nearly identical code**  |  The same work in almost the same code. Names, inputs, constants or messages may differ; small edits may be present.  |  **Type-2** renamed/parameterized copies and close **Type-3** copies with small edits.  |  Yes |
| **Similar code**  |  Substantial copied work remains, but statements or control flow have changed enough that it is no longer a near-copy.  |  More extensively edited **Type-3** clones. This is not Type-4.  |  Yes, for established clone occurrences |
| **Same shape, different content**  |  A similar layout, but none or almost none of the actual content is shared. Only the shape matches. **Not a clone; listed for interest.**  |  No research clone type. This is Deslop's informational category.  |  **No** |

The corpus pins known examples: CLEARLY IN must be found and CLEARLY OUT must not be reported as clones. Other code can qualify through sufficiently strong evidence; it need not already appear in the corpus. Shape without reasonable content similarity does not qualify.

Repeating the same calls in the same order with different arguments or test messages belongs in **Nearly identical code**. Changing names or values does not, by itself, make code shape-only. A systematic rename or parameter substitution is still a copy.

The optional fifth category, **Same behavior, different code**, corresponds to **Type-4**: equivalent work implemented in different ways. Deslop uses embedding evidence to find candidates; similarity between embeddings alone is not proof that a refactor preserves behaviour.

### [CLONE-BUCKETS-STRUCTURAL-ONLY] Shape-only is not duplication

**Shape-only means matching structure with no or negligible content similarity. It is not a clone.** It is always the last category, below Similar, regardless of its size or number of matches.

Shape-only contributes nothing to duplicated lines, duplicated files, clone counts, duplicated mass, repository/file/folder/diff percentages or threshold breaches. Its source lines remain in the analysed-line denominator. Showing it, hiding it or enabling its diagnostics cannot change those figures ([METRICS-REPO](pipeline.md#metrics-repo-repo-wide-duplication-metrics)).

Keep genuine clones inside a mixed group. An unrelated member must neither make its lines count nor hide a real copied subset. If a real clone and an informational finding cover the same line, that line counts once for the real clone.

Shape-only publishes no diagnostic by default, even when clone diagnostics are enabled. The user can explicitly choose another severity; it remains a non-clone ([SEVERITY-DIAGNOSTICS-STRUCTURAL-ONLY](severity.md#severity-diagnostics-structural-only-shape-only-is-silent-by-default)).

## [CLONE-TYPE-TAXONOMY] How this relates to the research

The standard taxonomy describes pairs of code fragments:

- **Type-1:** identical apart from whitespace and comments.
- **Type-2:** the same structure with renamed identifiers, types or changed literals.
- **Type-3:** a copy with added, removed or modified statements.
- **Type-4:** equivalent behaviour implemented with different code.

Deslop splits Type-3 between Nearly identical and Similar for readability. That boundary is a product choice, not a fifth research type. Shape-only is outside the clone taxonomy. Source: [Roy, Cordy and Koschke, 2009](https://doi.org/10.1016/j.scico.2009.02.007).

## [CLONE-KIND] How a group gets its label

### [CLONE-KIND-FOLD] Compare the actual members

Compare each member with the group's reference occurrence. Use the weakest **established** relation to describe the group, while preserving stronger clone subsets. A chain of matches does not prove that its first and last members match. Separate groups where necessary; a rejected or unmeasured comparison must not turn near-copies into Similar code.

Classify and separate informational findings before applying [RANK-MASS-SUM](pipeline.md#rank-mass-sum-rank-by-duplicated-mass-only) and [METRICS-REPO].

### [CLONE-KIND-LABELS] Use the same names everywhere

| Kind | Display title | Research label |
|---|---|---|
| `identical` | Identical code | Type-1 exact clone |
| `nearly_identical` | Nearly identical code | Type-2 / close Type-3 clone |
| `same_behavior` | Same behavior, different code | Type-4 semantic clone |
| `loosely_similar` | Similar code | Type-3 clone with larger edits |
| `structural_only` | Same shape, different content | Informational non-clone |

This is the category display order. `loosely_similar` remains the machine key; its human title is **Similar code**. Every renderer, tree, hover, diagnostic and Copy Context For AI uses this registry. Titles describe the relation; they do not promise that merging is safe. Diagnostic defaults and user overrides come from [SEVERITY-DESLOP-MAP](severity.md#severity-deslop-map-defaults-when-diagnostics-are-enabled), not from mass rank.

### [CLONE-KIND-COLOR] Colour describes the category

Use crimson for Identical, amber for Nearly identical, violet for Same behavior, blue for Similar, and muted grey for shape-only. Keep an icon for each so colour is not the only distinction.

### [CLONE-KIND-TESTING] Required examples and assertions

CLI fixtures must cover exact copies, consistent renames, the same calls with changed inputs/messages, small edits, larger edits, and shape-only matches. Assert category, occurrence count, paths and order. Include a mixed group so unrelated content cannot hide near-copies.

A corpus containing only shape-only information must report zero duplication. A mixed corpus must count only its genuine clone lines, once, with exact repository/file/folder/diff percentages and threshold verdicts. Shape-only appears last and changing its visibility or diagnostic severity changes none of those figures. Diagnostic defaults and overrides must pass [SEVERITY-TESTING](severity.md#severity-testing-required-checks).

## [CLONE-IMPLEMENTATION] For AI: implementation contract

### [CLONE-BUCKETS] Pair classifications

Classifications describe two explicit source ranges. `Identical`, `NearlyIdentical`, `SameBehavior` and `LooselySimilar` require the corresponding clone evidence. `StructuralOnly` is informational and never admitted as a clone. Other rejected or unmeasured pairs receive no clone classification; they must not fall into a catch-all LooselySimilar bucket.

### [CLONE-BUCKETS-DUAL-LABEL] Label ownership

Pair records identify both endpoints and may include their evidence and admission decision. Group records carry their kind and membership, never one selected pair's measurements. Clone records also carry mass and rank; informational records do not claim those duplicate quantities.

### [CLONE-BUCKETS-ROUTING] Classification must match the definitions

Use the shared pair measurement and admission rules ([FUSED-CONTENT-GATE], [FUSED-STRATEGY-BOUNDED-MAX]). Exact text proof selects Identical. A consistent rename, parameter substitution or near-copy selects NearlyIdentical. An established copy with substantial edits selects LooselySimilar. Independent evidence of equivalent behaviour with different code selects SameBehavior.

Identical normalized structure with no or negligible content similarity and no independent evidence strong enough to establish a clone selects StructuralOnly. Failing `content_gate.support_floor` or `content_gate.promote_floor` is not enough: moderate content similarity is not negligible. Near-zero content uses `routing.shape_only_max_content`; category defaults and TOML keys are listed below. Missing measurements are unknown, not proof of zero similarity.

Do not lower admission thresholds merely to repair a misleading label. Consistent renaming, parameter substitutions and shared copied operations are content evidence even when many raw names or literals differ. Classification must consider that evidence. Every relation lacking meaningful content similarity and independent evidence strong enough to establish a clone is excluded from metrics regardless of its label.

### [CLONE-BUCKETS-THRESHOLDS] Defaults and TOML settings

These settings classify a pair after the clone-admission checks. They never make a rejected pair into a clone. Exact copies and proven consistent renames/parameter substitutions keep their Identical/Nearly identical classification without a raw-name similarity penalty.

| TOML key under `[tuning.routing]` | Default | Meaning |
|---|---|---|
| `nearly_identical_min_shape` | `0.90` | Minimum code-structure similarity for the general near-copy route. |
| `nearly_identical_min_content` | `0.70` | Minimum content support for that route. |
| `similar_min_content` | `0.50` | Minimum content support for the general Similar route; the pair must also pass clone admission. |
| `shape_only_max_content` | `0.05` | At most this much content support counts as none or negligible, provided shape matches and no independent clone evidence qualifies. |

**These configurable defaults remain provisional pending corpus validation.** They are Deslop settings, not research-mandated boundaries. Content support uses matching content or consistent-renaming evidence, not just equal identifier spellings. Pairs between the Similar minimum and the shape-only maximum are not silently counted as clones. Missing evidence is not zero evidence.

```toml
[tuning.routing]
nearly_identical_min_shape = 0.90
nearly_identical_min_content = 0.70
similar_min_content = 0.50
shape_only_max_content = 0.05
```

All values must be finite and within `[0, 1]`; require `shape_only_max_content < similar_min_content <= nearly_identical_min_content`. Reject invalid settings. [FUSED-TUNING-LEVERS] lists the existing admission, structural, token and embedding thresholds with their defaults; all remain configurable through `.deslop.toml`. Classification must not confuse admission floors with the near-zero ceiling. Record effective settings in reports. Before implementation is accepted, corpus and CLI assertions must verify every default, boundary and override, including changed arguments/messages remaining Nearly identical.

### [CLONE-BUCKETS-IDENTICAL] Identity needs source text

`Identical` requires equal source content after ASCII-whitespace folding. Whitespace inside a literal is content and must still match. Equal normalized trees or token signatures cannot prove identity because names and literals have been removed. Missing source cannot prove identity. Every member must satisfy this rule against its reference. An unchanged method does not make its containing class Identical if the class name differs.

### [CLONE-CATEGORY-REGISTRY] Other finding kinds

These describe dedicated findings rather than research clone types. They never turn an informational finding into a clone or alter the clone mass formula.

| Finding kind | Wire label | Purpose |
|---|---|---|
| `Logic` | `logic` | Ordinary code repetition. |
| `DataTable` | `data` | Repeated data-table shape. |
| `MagicLiteral` | `magic_literal` | Repeated inline literal. |
| `ShadowedConstant` | `shadowed_constant` | Inline value already named by a constant. |
| `ConstantDuplicate` | `constant_duplicate` | Same constant declared repeatedly. |
| `ConstantDrift` | `constant_drift` | Same constant name resolves to conflicting values. |
| `ConstantAlias` | `constant_alias` | One value has several constant names. |

Dedicated literal records follow [LITERAL-WIRE]. Generated wire models must keep informational findings distinct from clone counts and ranking; update `docs/models/live-ipc.td`, never hand-written wire types.
