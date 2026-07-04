//! Bucket-group expanders and CSS-only facet controls for the HTML
//! report ([FACET-HTML], issue #257).
//!
//! Cluster cards are grouped into one collapsible `<details>` per clone
//! bucket so the reader is never forced to scroll one flat page, and a
//! row of checkbox-driven facet chips filters the groups without a
//! single line of JS — the report must stay inert both on `file://`
//! and inside the VSIX's script-disabled webview tab. Every label and
//! CSS class derives from the canonical bucket registry
//! ([`ClusterKind`]) so this surface can never drift from the VS Code
//! panel's vocabulary ([CLONE-BUCKETS-DUAL-LABEL]).

use std::{collections::HashMap, fmt::Write as _, hash::Hash};

use crate::{
    buckets::{bucket_labels, classify, ClusterKind},
    clone_category::CloneCategory,
    render::{
        html::{write_cluster_card, SnippetLoader},
        html_escape::escape,
    },
    report::ReportCluster,
};

/// Buckets `clusters` by `key`, preserving the input worst-first order
/// within each group. Group order is first-seen — and because the input
/// is globally worst-first, the first key seen owns the worst cluster,
/// so groups come out ordered by worst weight desc.
pub(super) fn group_by_first_seen<'c, K, I>(
    clusters: I,
    key: impl Fn(&ReportCluster) -> K,
) -> Vec<(K, Vec<&'c ReportCluster>)>
where
    K: Copy + Eq + Hash,
    I: IntoIterator<Item = &'c ReportCluster>,
{
    let mut order: Vec<K> = Vec::new();
    let mut groups: HashMap<K, Vec<&ReportCluster>> = HashMap::new();
    for cluster in clusters {
        let group_key = key(cluster);
        let group = groups.entry(group_key).or_insert_with(|| {
            order.push(group_key);
            Vec::new()
        });
        group.push(cluster);
    }
    order
        .into_iter()
        .filter_map(|group_key| groups.remove(&group_key).map(|list| (group_key, list)))
        .collect()
}

/// Writes one collapsible `<details class="bucket-group">` expander per
/// bucket present in `clusters`, worst-first across and within groups.
/// The first (worst) group renders expanded so the top offender stays
/// one glance away; the rest start collapsed.
pub(super) fn write_bucket_groups<'c>(
    out: &mut String,
    clusters: impl IntoIterator<Item = &'c ReportCluster>,
    snippets: &mut SnippetLoader<'_>,
) {
    let grouped = group_by_first_seen(clusters, classify);
    for (position, (kind, members)) in grouped.into_iter().enumerate() {
        let labels = bucket_labels(kind);
        let _ = write!(
            out,
            "<details class=\"bucket-group kind-{suffix}\"{open}><summary>{title} — {count} group(s)</summary>",
            suffix = labels.css_suffix,
            open = if position == 0 { " open" } else { "" },
            title = escape(labels.plain_title),
            count = members.len(),
        );
        for cluster in members {
            write_cluster_card(out, cluster, snippets);
        }
        out.push_str("</details>");
    }
}

/// Buckets present in `clusters`, in canonical registry order.
fn buckets_present(clusters: &[ReportCluster]) -> Vec<ClusterKind> {
    ClusterKind::all()
        .into_iter()
        .filter(|kind| clusters.iter().any(|cluster| classify(cluster) == *kind))
        .collect()
}

/// Categories present in `clusters`, in canonical registry order.
fn categories_present(clusters: &[ReportCluster]) -> Vec<CloneCategory> {
    CloneCategory::all()
        .into_iter()
        .filter(|category| {
            clusters
                .iter()
                .any(|cluster| CloneCategory::from_wire_label(&cluster.category) == *category)
        })
        .collect()
}

/// One CSS-only facet chip: a checkbox input class, its visible label,
/// and the selector fragment its unchecked state hides.
struct FacetChip {
    /// Class (and id) of the hidden checkbox, e.g. `facet-identical`.
    input_class: String,
    /// Human label from the shared registry helpers.
    label: &'static str,
    /// Selector the unchecked state hides, e.g. `.bucket-group.kind-identical`.
    hide_target: String,
}

/// Chips for both facet axes ([FACET-MODEL]), each axis contributing
/// only when at least two of its values are present — a filter with one
/// choice filters nothing. Bucket chips hide their whole group;
/// category chips hide individual cards, so the two axes compose as an
/// AND without any shared state.
fn facet_chips(clusters: &[ReportCluster]) -> Vec<FacetChip> {
    let buckets = buckets_present(clusters);
    let categories = categories_present(clusters);
    let mut chips = Vec::new();
    if buckets.len() >= 2 {
        chips.extend(buckets.into_iter().map(|kind| {
            let labels = bucket_labels(kind);
            FacetChip {
                input_class: format!("facet-{}", labels.css_suffix),
                label: labels.plain_title,
                hide_target: format!(".bucket-group.kind-{}", labels.css_suffix),
            }
        }));
    }
    if categories.len() >= 2 {
        chips.extend(categories.into_iter().map(|category| FacetChip {
            input_class: format!("facet-cat-{}", category.wire_label()),
            label: category.group_title(),
            hide_target: format!(".cluster-card.cat-{}", category.wire_label()),
        }));
    }
    chips
}

/// Writes the facet controls: one visually-hidden checkbox per chip
/// (direct children of the report shell, so the sibling selectors in
/// [`facet_css`] can reach the sections that follow) plus a labelled
/// chip row. A report with no filterable axis gets no controls.
pub(super) fn write_facet_controls(out: &mut String, clusters: &[ReportCluster]) {
    let chips = facet_chips(clusters);
    if chips.is_empty() {
        return;
    }
    for chip in &chips {
        let _ = write!(
            out,
            "<input type=\"checkbox\" id=\"{id}\" class=\"facet-input {id}\" checked>",
            id = chip.input_class,
        );
    }
    out.push_str("<div class=\"facet-bar\"><span class=\"facet-bar__label\">Show:</span>");
    for chip in &chips {
        let _ = write!(
            out,
            "<label class=\"facet-chip\" for=\"{id}\">{label}</label>",
            id = chip.input_class,
            label = escape(chip.label),
        );
    }
    out.push_str("</div>");
}

/// Per-chip facet CSS: one hide rule (an unchecked facet hides its
/// chip's target in every following section) and one checked-chip
/// highlight rule. Derived from the canonical registries via
/// [`facet_chips`] so the selector set can never drift, and absent
/// values leave zero facet traces in the page.
pub(super) fn facet_css(clusters: &[ReportCluster]) -> String {
    facet_chips(clusters)
        .into_iter()
        .fold(String::new(), |mut css, chip| {
            let _ = write!(
                css,
                ".{id}:not(:checked)~section {target}{{display:none;}}\
                 .{id}:checked~.facet-bar>[for={id}]{{background:var(--secondary-container);color:var(--on-secondary-container);}}",
                id = chip.input_class,
                target = chip.hide_target,
            );
            css
        })
}
