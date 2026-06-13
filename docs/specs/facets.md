# Facets — grouping, filtering, and sorting by finding type, everywhere

One facet model across every surface: humans and agents isolate any finding family in the same
vocabulary on the VS Code Top Offenders tree, the report webviews, the HTML report, the CLI
summary, and the MCP tools ([MCP-TOOL-FILTERS]). Rationale lives in issue #195: *"agents can
already ask for identical only; humans in the editor cannot. That asymmetry is the bug."*

### [FACET-MODEL] Axes, registries, and the anti-drift rule

A facet is `(axis, allowed wire values)`. The axes:

| Axis | Values (wire labels) | Single source of truth |
|---|---|---|
| `bucket` | `ClusterKind::all()` wire labels (e.g. `identical`) | canonical list: [CLONE-BUCKETS] → TS `BUCKETS` mirror |
| `category` | `CloneCategory::all()` wire labels (e.g. `magic_literal`) | canonical list: [CLONE-CATEGORY-REGISTRY] → TS `CATEGORIES` mirror |
| `language` | the registered parser ids | core language registry (the #170/#198 fix) |
| `severity` / `path` | existing | severity.md / path glob |

This table deliberately does **not** enumerate the bucket/category values — the registry sections
own the lists; a second copy here would be exactly the hand-listed drift this model prohibits.

Hard rules:

1. **Values come only from the canonical registries** — `wire_label()` on the Rust enums and their
   generated/mirrored TS constants. Never hand-typed strings, never a hardcoded enum in a schema or
   a `<select>` (the #170/#198 anti-drift lesson). Adding a registry value flows into every facet
   surface with zero per-surface edits.
2. **Facet labels come from the existing single-helper label functions** (`buckets` sextuple,
   `categoryLabels`) — one renderer, every surface (UI consistency hard rule).
3. **Facets are presentation-only** on editor surfaces per [VSIX-VIEW-STATE-UI-ONLY]: no LSP call,
   no re-scan, no cache invalidation. The MCP/CLI facets filter the same canonical report
   server-side; same vocabulary, same semantics (a filter never mutates the report).
4. **The user's vocabulary**: "type"/"family" in user-speak maps to the `bucket` axis (how similar)
   plus the `category` axis (what kind of repetition). The canonical phrase mappings, identical
   here and in [MCP-TOOL-FILTERS]: *"only IDENTICAL code"* = `buckets: ["identical"]`;
   *"identical literals"* = `categories: ["magic_literal"]` optionally + `buckets: ["identical"]`
   (`shadowed_constant` is the separate prevention category — name it explicitly when wanted).
   Every surface's filter copy uses the plain-English labels with the wire label available to AI
   per [CLONE-BUCKETS-DUAL-LABEL].

### [FACET-TOP-OFFENDERS-FILTER] Top Offenders multi-select filter (#195)

Two persisted, workspace-scoped settings (array-valued, symmetric with the MCP filter params):

- `deslop.topOffenders.filterBuckets: string[]` — default `[]` = show all.
- `deslop.topOffenders.filterCategories: string[]` — default `[]` = show all.

Unknown values are ignored with fallback-to-all (the [VSIX-TOP-OFFENDERS-GROUPING] unknown-value
rule — never panic, never an empty tree from a typo). The filter applies as one slice in the
provider's root-building step **after** global rank is computed, so rank #N is never renumbered
([VSIX-TOP-OFFENDERS-RANK-GLOBAL] extends to filtering): a filtered view legitimately shows gaps
`#1, #4, #9`.

**Choose Filter** (`deslop.topOffenders.chooseFilter`) is a view-title action at **`navigation@4`**
— immediately after the grouping/sort/split toggles, ahead of Expand All / Collapse All / Refresh
at `@5`/`@6`/`@7` (the order [VSIX-TOP-OFFENDERS-TOOLBAR] defines). It opens a multi-select
QuickPick listing **only the buckets and categories present in the current report**
(`same_behavior` appears only when the embedding pass ran, #195), each row showing the plain title
and live cluster count. A `deslop.topOffendersFiltered` context key drives an active-filter icon
state on the toolbar button.

**Surface scope.** The filter applies wherever clusters are *listed*: the Top Offenders tree, the
report webview, and the status-bar `dedup` cluster count (#195's consistency clause — the count
must agree with the filtered tree it summarises). It deliberately does **not** apply to the live
bubble, diagnostics, decorations, or code lenses: those are the prevention surfaces, and hiding
real duplication while the user types it would break the product's defining moment
([VSIX-PRINCIPLES] principle 1).

#### [FACET-TOP-OFFENDERS-FILTER-EMPTY] A filtered-empty tree must say so

When any filter is active, a non-collapsible status row renders as the **first** root:
`Filtered: Identical code · magic values — Clear filter`, with the clear action bound. A
filtered-empty tree must never be mistakable for "No duplication detected" — the empty state and
the filtered state are distinct renders.

### [FACET-GROUP-BY-TYPE] `type` grouping mode

In the `"type"` grouping mode ([VSIX-TOP-OFFENDERS-GROUPING] owns the `groupBy` enum), roots are
one group per **category** present in the report (registry order, empty groups omitted). Grouping
by category — not by bucket, which #195's wording asked for — is a recorded decision: bucket
isolation is the filter's job ([FACET-TOP-OFFENDERS-FILTER]) and file mode already nests bucket
sub-groups, so the grouping axis answers the orthogonal "what kind of repetition" question the
filter cannot express. Group roots are labelled by the shared category chip + live count; the
chip-less `logic` category uses the plain group title **"Code clones"** (its cluster rows carry no
chip). Each group contains its clusters worst-first and composes with the sort axis and language
split exactly like the other modes. Cluster rows inside a type group keep their bucket icon/colour
so the two axes stay distinguishable. Type groups and file-mode bucket groups render through the
same group-node machinery — one implementation, two group axes.

### [FACET-REPORT-WEBVIEW] Full-report webview filters

The full-report webview ([VSIX-REPORT-WEBVIEW]) filters on `bucket` and `category` (single-select
each, `null` = all) beside language / severity / path-glob. Every `<select>`'s options derive from
the TS registry mirrors + shared label helpers (rules 1/2): the language options come from the
language registry (every registered language, Dart included, is selectable), the severity options
enumerate every severity level, and severity resolution goes through the one shared severity
helper — never a webview-local map. Sort is fixed worst-first (the product premise).

### [FACET-HTML] HTML report facets (CSS-only)

The static HTML report ([OUTPUT-HUMAN-HTML]) facets via checkbox inputs + sibling selectors over
the per-card classes — the no-JS invariant holds. Every cluster card carries both a bucket class
(`kind-<css_suffix>`) and a category class (`cat-<wire_label>`); one selector rule per registry
value, counted against the inlined report CSS budget. The intro breakdown sentence includes a
literal-family clause when those clusters exist ("…plus 12 magic values, 3 duplicate constants,
1 conflicting constant"), and drift cards render per-occurrence values inline (from
`constant_value`).

### [FACET-CLI] CLI summary breakdown

The stderr summary's cluster breakdown includes a second line counting literal-family categories
when non-zero, driven by `CloneCategory::all()` through the existing breakdown plumbing. The CLI
has no presentation filter flags — agents consume the JSON or the MCP surface
([MCP-TOOL-FILTERS]); the summary is for humans.

### [FACET-MCP] MCP

The MCP filter block ([MCP-TOOL-FILTERS]) uses the same array-valued `buckets` / `categories` /
`languages` params over the same registry-derived enums — the agent-side spelling of this exact
model. Specced in mcp.md so the tool surface stays in one document.

### [FACET-TESTING] Proof

- **Tree**: filterBuckets `["identical"]` hides others; status row appears with a working clear
  action; rank gaps preserved; unknown values fall back to all; QuickPick lists only present values
  with counts; `groupBy: "type"` renders category roots in registry order, empty groups omitted;
  the toolbar order matches [VSIX-TOP-OFFENDERS-TOOLBAR] exactly (pin assertions).
- **Webview**: bucket + category selects render registry-derived options; filtering to a bucket
  shows only matching cluster cards; Dart appears in the language options.
- **HTML**: rendered report contains the facet inputs, `cat-*` classes, and the literal-family
  breakdown clause; toggling a facet checkbox hides non-matching cards (assert via the CSS
  selector contract, no JS).
- **Cross-surface consistency**: one fixture, same filter on tree / webview / MCP `duplicates` —
  identical cluster id sets ([VSIX-PRINCIPLES] "every surface speaks the same schema").
