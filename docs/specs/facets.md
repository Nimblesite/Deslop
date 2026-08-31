# Facets — grouping and filtering cluster membership

### [FACET-MODEL] Cluster facets use cluster-owned data only

A cluster list may group or filter by language, path, visibility, and engine-stamped mass severity. Structural, Jaccard, embedding, content, rename, literal, and pair classification values belong to explicit pair records and are forbidden cluster facets.

| Axis | Values | Source of truth |
|---|---|---|
| `language` | Registered parser ids | Core language registry. |
| `path` | Workspace-relative path prefixes | Cluster occurrence membership. |
| `severity` | `error`, `warning`, `information`, `hint` | Engine-stamped mass rank band through [SEVERITY-MODEL]. |
| `visibility` | Visible or hidden occurrence state | [EXCLUSION-CONFIG]. |

Facet filters are presentation-only. They never mutate the canonical report, renumber ranks, recalculate mass, or trigger analysis.

### [FACET-TOP-OFFENDERS-FILTER] Top Offenders filter

The Top Offenders filter supports language, path, and mass severity. Options are derived from values present in the current report. Unknown values are ignored with fallback-to-all. Filtering happens after the engine has ranked the full report, so a filtered view keeps global ranks and may show gaps.

The filter applies to cluster-list surfaces: the Top Offenders tree, full-report webview, and the status-bar count that summarizes the list. It does not hide live prevention surfaces such as diagnostics, decorations, or code lenses.

#### [FACET-TOP-OFFENDERS-FILTER-EMPTY] A filtered-empty tree says so

When a filter is active, the first root row states the active filter and offers `Clear filter`. A filtered-empty tree says `No clusters match this filter`; it never says that no duplication exists.

### [FACET-GROUP-BY-TYPE] Pair classification is not a cluster grouping mode

The retired `type` and bucket grouping modes are invalid because they project pair classification onto closure components. Top Offenders supports cluster, file, folder, and language grouping. Every cluster row keeps the engine-stamped global rank and mass.

### [FACET-REPORT-WEBVIEW] Full-report webview filters

The full-report webview exposes language, path, and mass-severity filters. Sort is fixed to engine rank. The webview performs no calculation and receives no pair evidence until the user opens an explicit two-occurrence comparison.

### [FACET-HTML] HTML report facets

The static HTML report groups cluster cards by language or path prefix and may filter them by mass severity using CSS-only controls. Cards contain membership and mass only. No pair-evidence class or pair-classification class is emitted on a card.

### [FACET-CLI] CLI summary breakdown

The CLI summary may break cluster counts down by language and mass severity. It does not print a similarity-class breakdown. Pair evidence is available only through the explicit pair-comparison command or tool.

### [FACET-MCP] MCP filters

MCP cluster-list tools accept language, path, and mass-severity filters. An explicit pair-comparison tool accepts two occurrence endpoints and returns pair evidence; pair fields are not accepted as cluster filters.

### [FACET-TESTING] Proof

Tests assert that every cluster filter consumes only cluster-owned fields, filtered views preserve global rank and mass, unknown filters fall back safely, filtered-empty differs from truly empty, and pair evidence never appears in cluster facet payloads or HTML classes.
