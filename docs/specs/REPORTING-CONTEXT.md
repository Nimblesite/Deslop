# Report context — cluster mass and explicit pair evidence

## [REPORT-CONTEXT-SCOPE] What the report says

Deslop detects duplicated code with normalized AST fingerprints, token MinHash/LSH, and optional embeddings. The report lists closure components worst-first by duplicated mass. It does not assign a similarity score, evidence verdict, or pair classification to a component.

## [REPORT-CONTEXT-PIPELINE] How a finding forms

Candidate generation proposes concrete pairs through exact fingerprints, token LSH, or embedding neighbours. For each candidate pair, the engine measures structural similarity, token Jaccard, embedding similarity, content agreement, and rename consistency. The complete pair admission rule decides whether that pair becomes an edge. Connected components of admitted edges are the cluster inputs. After closure, [CLONE-NOISE-VERBATIM-SUBGROUP] may replace a component that a noise filter actually convicts with its qualifying byte-identical families and drop outsiders; a component no filter convicts remains untouched. This is the only post-closure partition and it neither changes admission nor creates cluster evidence.

Cross-language comparison is off by default. `.deslop.toml` may enable `[analysis] allow_cross_language_comparison = true`. Boilerplate and noise filters operate under their own explicit contracts; they do not manufacture a cluster score.

## [REPORT-CONTEXT-CLUSTER] How to read a cluster

A cluster record contains identity, occurrence membership, canonical extent, duplicated mass, and rank. Its essential fields are:

| Field | Meaning |
|---|---|
| `id` | Stable 16-character cluster identity. |
| `rank` | One-based position in the engine's mass-descending order. |
| `mass` | Duplicated mass exactly. |
| `canonical_node_count` | Normalized AST nodes in the canonical extent. |
| `occurrence_count` | Number of visible occurrences. |
| `occurrences[]` | Exact file paths and byte ranges belonging to the component. |

The mass formula is:

$$
\mathrm{mass}(c)=\mathrm{canonical\_node\_count}(c)\times\max(\mathrm{visible\_occurrences}(c)-1,0)
$$

A component with fewer than two visible occurrences has zero mass and is not a reported duplicate. Clusters sort by mass descending and cluster id ascending. Similarity evidence, pair classification, category, confidence, file spread, and policy multipliers never change mass or order.

## [REPORT-CONTEXT-PAIR] How to read pair evidence

Pair evidence exists only for two explicitly identified occurrences. A pair response names both endpoints and may contain:

| Field | Meaning |
|---|---|
| `structural` | Normalized-AST structural similarity for this pair. |
| `token_jaccard` | MinHash estimate of normalized token-set Jaccard for this pair. |
| `embedding_cos` | Embedding cosine similarity for this pair, when measured. |
| `agreement` | Raw content agreement for this pair. |
| `rename_consistency` | Consistent-renaming support for this pair. |
| `literal_fraction` | Literal share measured for this pair. |
| `admitted` | Whether this exact pair passed the admission contract. |
| `classification` | Optional presentation classification of this exact pair. |

These values explain why the exact edge was or was not admitted. They do not describe the other edges in a transitive closure and must never be copied onto a cluster. There is no automatic pair selection for a cluster.

## [REPORT-CONTEXT-METRIC] Repository duplication percentage

The report carries one repository duplication percentage:

$$
\mathrm{duplication\_percent}=\begin{cases}0 & \mathrm{analysed\_loc}=0\\100\times\mathrm{duplicated\_loc}/\mathrm{analysed\_loc} & \mathrm{otherwise}\end{cases}
$$

`duplicated_loc` counts physical lines covered by at least two non-hidden fragment-clone occurrences, deduplicated per file. `analysed_loc` counts physical lines in analyzed files. Pair evidence never weights a line. There is no evidence-weighted companion percentage.

`--fail-over <percent>` or `[threshold] max_duplication_percent` exits with code `3` when the engine-computed percentage exceeds the configured threshold. No threshold means no duplication gate.

## [REPORT-CONTEXT-ACTION] How to act on a report

Work from highest mass downward. Inspect the occurrence membership and exact byte ranges. Before a refactor depends on similarity or consistent renaming, explicitly compare the concrete source and target occurrences involved in that edit. Never treat one pair's evidence as proof about the entire component.

Generated-code exclusions and `report_hide` rules affect visibility, not pair evidence or the mass formula. Import-only repetition is boilerplate hygiene rather than a clone finding.

## [REPORT-CONTEXT-METADATA] Canonical rendering contract

The text and HTML reports are renderers over the canonical engine model. Cluster surfaces render identity, membership, mass, and rank only. Pair surfaces render the exact two endpoints and their engine-computed evidence. Consumers do not recompute percentages, mass, admission, or evidence.
