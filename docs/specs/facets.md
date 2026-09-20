# Facets — grouping and filtering cluster membership

### [FACET-MODEL] Cluster facets use cluster-owned data only

A cluster list may group or filter by language, path, visibility, configured diagnostic severity, and the engine-stamped clone kind. Structural, Jaccard, embedding, content, rename, and literal values belong to explicit pair records and are forbidden cluster facets.

| Axis | Values | Source of truth |
|---|---|---|
| `language` | Registered parser ids | Core language registry. |
| `path` | Workspace-relative path prefixes | Cluster occurrence membership. |
| `severity` | Configured diagnostic levels | [SEVERITY-CONFIG](severity.md#severity-config-configuration), independent of the master on/off switch. |
| `kind` | [CLONE-KIND-LABELS](taxonomy.md#clone-kind-labels-use-the-same-names-everywhere) | Engine-stamped kind through [CLONE-KIND-FOLD](taxonomy.md#clone-kind-fold-compare-the-actual-members). |
| `visibility` | Visible or hidden occurrence state | [EXCLUSION-CONFIG]. |

Facet filters are presentation-only. They never mutate the canonical report, renumber ranks, recalculate mass, or trigger analysis.

### [FACET-TOP-OFFENDERS-FILTER] Top Offenders filter

The Top Offenders filter supports language, path, and diagnostic severity. Options are derived from values present in the current report. Unknown values are ignored with fallback-to-all. Filtering happens after the engine has ranked the full report, so a filtered view keeps global ranks and may show gaps.

The filter applies to cluster-list surfaces: the Top Offenders tree, full-report webview, and the status-bar count that summarizes the list. It does not hide live prevention surfaces such as diagnostics, decorations, or code lenses.

#### [FACET-TOP-OFFENDERS-FILTER-EMPTY] A filtered-empty tree says so

When a filter is active, the first root row states the active filter and offers `Clear filter`. A filtered-empty tree says `No clusters match this filter`; it never says that no duplication exists.

### [FACET-GROUP-BY-KIND] Clone kind is a cluster grouping mode

Top Offenders supports cluster, file, folder and kind grouping. Use the names and order in [CLONE-KIND-LABELS](taxonomy.md#clone-kind-labels-use-the-same-names-everywhere) and the informational handling in [CLONE-BUCKETS-STRUCTURAL-ONLY] in every mode. Omit empty groups. Preserve engine rank for clone rows; show informational match counts separately from clone counts.

### [FACET-GROUP-BY-TYPE] Every clone category has a plain group title

Grouping and facet surfaces title a category with the chip its category already carries, so one concept is spelled one way everywhere. The logic category carries no chip — it is the ordinary case, not a special one — and titles as `Code clones` rather than rendering an empty heading or leaking an enum name into the UI.

The title is a property of the category, resolved once by `CloneCategory::group_title`, so the tree, the webview facet list and the HTML report cannot title the same category three ways. This is orthogonal to [FACET-GROUP-BY-KIND]: kind is how strongly two occurrences match, category is what sort of code they are.

### [FACET-REPORT-WEBVIEW] Full-report webview filters

The full-report webview filters by diagnostic severity, kind and path. It preserves engine ordering and applies [CLONE-BUCKETS-STRUCTURAL-ONLY] to informational findings. Filtering never changes reported totals.

### [FACET-HTML] HTML report facets

HTML uses one collapsible group per kind, following [FACET-GROUP-BY-KIND] within each language section. Cards use the shared title and colour; display membership and the engine-provided clone weight where applicable. Apply [CLONE-BUCKETS-STRUCTURAL-ONLY] to informational findings. Pair measurements require an explicit comparison.

### [FACET-CLI] CLI summary breakdown

The CLI summary may break cluster counts down by language and diagnostic severity, and titles each listed cluster row by its clone kind ([CLONE-KIND-LABELS]). Pair evidence is available only through the explicit pair-comparison command or tool.

### [FACET-MCP] MCP filters

MCP cluster-list tools accept language, path, and diagnostic-severity filters. An explicit pair-comparison tool accepts two occurrence endpoints and returns pair evidence; pair fields are not accepted as cluster filters.

### [FACET-TESTING] Proof

Required tests preserve engine rank and totals under filtering, handle unknown and empty filters, and verify [CLONE-KIND-TESTING](taxonomy.md#clone-kind-testing-required-examples-and-assertions) across every grouping mode. No client calculates pair classifications or duplication figures.
