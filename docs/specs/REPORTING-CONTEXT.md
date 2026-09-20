# Reading a Deslop report

## [REPORT-CONTEXT-SCOPE] Definitions

Use the [category guide](taxonomy.md) for category meanings, research types, display order and thresholds. [Diagnostic severity](severity.md) defines diagnostic defaults and overrides. [Weight](pipeline.md#rank-mass-sum-rank-by-duplicated-mass-only) and [duplication metrics](pipeline.md#metrics-repo-repo-wide-duplication-metrics) have one calculation each in the engine.

## [REPORT-CONTEXT-PIPELINE] How findings form

The engine compares source ranges, applies the [admission rules](admission.md), and groups qualifying matches under [CLONE-KIND-FOLD](taxonomy.md#clone-kind-fold-compare-the-actual-members). Informational matches follow [CLONE-BUCKETS-STRUCTURAL-ONLY](taxonomy.md#clone-buckets-structural-only-shape-only-is-not-duplication). A connection through other members does not prove that two selected occurrences match.

## [REPORT-CONTEXT-CLUSTER] Group fields

| Field | Meaning |
|---|---|
| `id` | Stable group identity. |
| `kind` | Category from [CLONE-KIND-LABELS](taxonomy.md#clone-kind-labels-use-the-same-names-everywhere). |
| `severity` | Default diagnostic level from [SEVERITY-DESLOP-MAP](severity.md#severity-deslop-map-defaults-when-diagnostics-are-enabled); editor overrides apply separately. |
| `rank` | Engine-assigned position among actual clones. |
| `mass` | Clone weight under [RANK-MASS-SUM](pipeline.md#rank-mass-sum-rank-by-duplicated-mass-only). |
| `canonical_node_count` | AST size of the reference occurrence; size alone does not establish a clone. |
| `occurrence_count` | Number of visible occurrences. |
| `occurrences[]` | File paths and exact source ranges. |

Informational records follow the exclusions in [CLONE-BUCKETS-STRUCTURAL-ONLY]. Consumers display the engine's counts and order without recalculating them.

## [REPORT-CONTEXT-PAIR] Comparing two occurrences

A pair response identifies both source ranges. Its evidence explains that comparison only:

| Field | Meaning |
|---|---|
| `structural` | Similarity of parsed code structure. |
| `token_jaccard` | Similarity of normalized token sets. |
| `embedding_cos` | Embedding similarity, when measured. |
| `content_measurement` | `measured` or `unmeasured`. Unmeasured content is unknown; do not display the next three numeric placeholders as zero similarity. |
| `agreement` | Matching source content. |
| `rename_consistency` | Support for consistent renaming. |
| `literal_fraction` | Share of literal values. |
| `text_identity` | `byte_identical`, `indentation_only`, or `different`. |
| `admitted` | Whether this pair passed admission. |
| `classification` | Pair category under [CLONE-BUCKETS-ROUTING](taxonomy.md#clone-buckets-routing-classification-must-match-the-definitions). |

Do not apply one pair's measurements to the whole group. Exact calculations belong to [fused.md](fused.md); [admission.md](admission.md) is the short guide.

## [REPORT-CONTEXT-METRIC] Duplication percentage

`Report.metrics` contains the engine's [METRICS-REPO](pipeline.md#metrics-repo-repo-wide-duplication-metrics) values. All renderers use these values, including their exclusions and denominator. The configured threshold and exit behaviour follow [EXIT-CODES].

## [REPORT-CONTEXT-ACTION] Reviewing findings

Review actual clones in rank order. Compare the concrete source ranges before deciding how to merge them. Category meanings and informational handling come from the [category guide](taxonomy.md); diagnostic settings do not change clone status.

## [REPORT-CONTEXT-METADATA] Rendering

Every surface uses the shared category titles and engine figures. Show pair measurements only when both endpoints are identified. Do not recalculate admission, weight or percentages in clients.

`Report.routing` and live deltas carry the effective [category thresholds](taxonomy.md#clone-buckets-thresholds-defaults-and-toml-settings) for the current analysis.
