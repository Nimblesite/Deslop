# Facets — grouping and filtering cluster membership

### [FACET-MODEL] Cluster facets use cluster-owned data only

A cluster list may group or filter by language, path, visibility, engine-stamped mass severity, and the engine-stamped clone kind. Structural, Jaccard, embedding, content, rename, and literal values belong to explicit pair records and are forbidden cluster facets.

| Axis | Values | Source of truth |
|---|---|---|
| `language` | Registered parser ids | Core language registry. |
| `path` | Workspace-relative path prefixes | Cluster occurrence membership. |
| `severity` | `worst`, `top10`, `mid`, `faint` | Engine-stamped mass rank band through [SEVERITY-MODEL]. |
| `kind` | `identical`, `nearly_identical`, `same_behavior`, `structural_only`, `loosely_similar` | Engine-stamped clone kind through [CLONE-KIND-FOLD]. |
| `visibility` | Visible or hidden occurrence state | [EXCLUSION-CONFIG]. |

Facet filters are presentation-only. They never mutate the canonical report, renumber ranks, recalculate mass, or trigger analysis.

### [FACET-TOP-OFFENDERS-FILTER] Top Offenders filter

The Top Offenders filter supports language, path, and mass severity. Options are derived from values present in the current report. Unknown values are ignored with fallback-to-all. Filtering happens after the engine has ranked the full report, so a filtered view keeps global ranks and may show gaps.

The filter applies to cluster-list surfaces: the Top Offenders tree, full-report webview, and the status-bar count that summarizes the list. It does not hide live prevention surfaces such as diagnostics, decorations, or code lenses.

#### [FACET-TOP-OFFENDERS-FILTER-EMPTY] A filtered-empty tree says so

When a filter is active, the first root row states the active filter and offers `Clear filter`. A filtered-empty tree says `No clusters match this filter`; it never says that no duplication exists.

### [FACET-GROUP-BY-KIND] Clone kind is a cluster grouping mode

Top Offenders supports cluster, file, folder, and kind grouping. Kind mode shows one flat group per clone kind present, strongest kind first (`Identical code` down to `Loosely similar code`), each titled and coloured by its kind ([CLONE-KIND-LABELS], [CLONE-KIND-COLOR]) and carrying its live cluster count; absent kinds render no group. File mode nests the same kind groups under each file, ordered by each group's worst cluster. The kind is the engine's fold ([CLONE-KIND-FOLD]); the client projects no pair classification of its own. Every cluster row keeps the engine-stamped global rank and mass.

### [FACET-GROUP-BY-TYPE] Every clone category has a plain group title

Grouping and facet surfaces title a category with the chip its category already carries, so one concept is spelled one way everywhere. The logic category carries no chip — it is the ordinary case, not a special one — and titles as `Code clones` rather than rendering an empty heading or leaking an enum name into the UI.

The title is a property of the category, resolved once by `CloneCategory::group_title`, so the tree, the webview facet list and the HTML report cannot title the same category three ways. This is orthogonal to [FACET-GROUP-BY-KIND]: kind is how strongly two occurrences match, category is what sort of code they are.

### [FACET-REPORT-WEBVIEW] Full-report webview filters

The full-report webview exposes mass-severity, clone-kind, and path filters, each option list derived from the shared registries. Sort is fixed to engine rank. The webview performs no calculation and receives no pair evidence until the user opens an explicit two-occurrence comparison.

### [FACET-HTML] HTML report facets

The static HTML report groups cluster cards into one collapsible expander per clone kind present, in the order kinds first appear down the worst-first list, inside the language sections when `split_by_language` is on. Each expander and card is titled by the kind and carries the kind's class suffix so the kind colour keys on it ([CLONE-KIND-LABELS], [CLONE-KIND-COLOR]). Cards contain the kind title, membership, and mass. No pair-evidence class is emitted on a card.

### [FACET-CLI] CLI summary breakdown

The CLI summary may break cluster counts down by language and mass severity, and titles each listed cluster row by its clone kind ([CLONE-KIND-LABELS]). Pair evidence is available only through the explicit pair-comparison command or tool.

### [FACET-MCP] MCP filters

MCP cluster-list tools accept language, path, and mass-severity filters. An explicit pair-comparison tool accepts two occurrence endpoints and returns pair evidence; pair fields are not accepted as cluster filters.

### [FACET-TESTING] Proof

Tests assert that every cluster filter consumes only cluster-owned fields, filtered views preserve global rank and mass, unknown filters fall back safely, filtered-empty differs from truly empty, kind groups appear only for kinds present and strongest first, and pair evidence never appears in cluster facet payloads or HTML classes.
