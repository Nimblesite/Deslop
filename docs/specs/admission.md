# [ADMISSION-GUIDE] How Deslop decides what is a clone

Deslop compares two pieces of code at a time. It checks their structure and content; optional embeddings provide another signal. Matching structure alone does not establish a clone.

1. **Find possible copies.** Compare syntax fingerprints, token patterns and, when enabled, embeddings.
2. **Check the evidence.** Apply the size, similarity and content checks. A consistent rename can establish a copy even when many names differ. A pair below the initial similarity threshold can still qualify when its shared code passes the stricter rescue checks.
3. **Assign a category.** Use the [category definitions and configurable defaults](taxonomy.md#clone-buckets-thresholds-defaults-and-toml-settings). Rejection is not a fallback Similar label. Matching structure with no or negligible content similarity is informational, not a clone.
4. **Build groups.** Join established copies and preserve real copied subsets when unrelated code shares their shape. A chain of matches does not prove that every member matches every other member.
5. **Measure duplication.** Count only real clones. Their weight uses duplicated AST mass; their percentages use duplicated source lines. Informational findings have zero weight and contribute nothing to duplication figures.

The complete admission rules and calculations live in [fused.md](fused.md). In particular, [the initial admission score](fused.md#fused-pre-rescue-score-keep-the-initial-score-separate-from-measured-overlap) is distinct from the overlap measured later: finding stronger overlap must not undo an otherwise valid rescue.

Use these definitions rather than repeating them elsewhere:

- [Categories, research taxonomy, thresholds and TOML settings](taxonomy.md).
- [How actual members determine a group's category](taxonomy.md#clone-kind-fold-compare-the-actual-members).
- [Weight and ranking](pipeline.md#rank-mass-sum-rank-by-duplicated-mass-only).
- [Duplication percentages](pipeline.md#metrics-repo-repo-wide-duplication-metrics).
- [Diagnostic severity defaults and overrides](severity.md).
