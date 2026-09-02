//! Neutral HTML grouping and diff-only facets.

use std::{collections::HashMap, fmt::Write as _, hash::Hash};

use crate::{
    render::html::{write_cluster_card, SnippetLoader},
    report::ReportCluster,
};

/// Groups clusters by `key`, preserving mass order and first-seen group order.
pub(super) fn group_by_first_seen<'c, K, I>(
    clusters: I,
    key: impl Fn(&ReportCluster) -> K,
) -> Vec<(K, Vec<&'c ReportCluster>)>
where
    K: Copy + Eq + Hash,
    I: IntoIterator<Item = &'c ReportCluster>,
{
    let mut order = Vec::new();
    let mut groups: HashMap<K, Vec<&ReportCluster>> = HashMap::new();
    for cluster in clusters {
        groups
            .entry(key(cluster))
            .or_insert_with(|| {
                order.push(key(cluster));
                Vec::new()
            })
            .push(cluster);
    }
    order
        .into_iter()
        .filter_map(|group_key| groups.remove(&group_key).map(|items| (group_key, items)))
        .collect()
}

/// Writes one neutral duplicate-code group in mass order.
pub(super) fn write_bucket_groups<'c>(
    out: &mut String,
    clusters: impl IntoIterator<Item = &'c ReportCluster>,
    snippets: &mut SnippetLoader<'_>,
) {
    let clusters: Vec<&ReportCluster> = clusters.into_iter().collect();
    let _ = write!(
        out,
        "<details class=\"clone-group\" open><summary>Duplicate code — {} group(s)</summary>",
        clusters.len()
    );
    for cluster in clusters {
        write_cluster_card(out, cluster, snippets);
    }
    out.push_str("</details>");
}

/// One CSS-only diff facet.
struct FacetChip {
    /// CSS class shared by the input and label.
    input_class: &'static str,
    /// Human-readable facet label.
    label: &'static str,
    /// Selector hidden when this facet is disabled.
    hide_target: &'static str,
}

/// Returns diff facets only when both populations exist.
fn facet_chips(clusters: &[ReportCluster]) -> Vec<FacetChip> {
    let touched = clusters
        .iter()
        .any(|cluster| cluster.intersects_diff == Some(true));
    let untouched = clusters
        .iter()
        .any(|cluster| cluster.intersects_diff == Some(false));
    if !(touched && untouched) {
        return Vec::new();
    }
    vec![
        FacetChip {
            input_class: "facet-diff-touched",
            label: "Touched by diff",
            hide_target: ".cluster-card.in-diff",
        },
        FacetChip {
            input_class: "facet-diff-untouched",
            label: "Untouched by diff",
            hide_target: ".cluster-card:not(.in-diff)",
        },
    ]
}

/// Writes the diff facet controls.
pub(super) fn write_facet_controls(out: &mut String, clusters: &[ReportCluster]) {
    let chips = facet_chips(clusters);
    if chips.is_empty() {
        return;
    }
    for chip in &chips {
        let _ = write!(
            out,
            "<input type=\"checkbox\" id=\"{0}\" class=\"facet-input {0}\" checked>",
            chip.input_class
        );
    }
    out.push_str("<div class=\"facet-bar\"><span class=\"facet-bar__label\">Show:</span>");
    for chip in &chips {
        let _ = write!(
            out,
            "<label class=\"facet-chip\" for=\"{}\">{}</label>",
            chip.input_class, chip.label
        );
    }
    out.push_str("</div>");
}

/// Returns CSS for the active diff facets.
pub(super) fn facet_css(clusters: &[ReportCluster]) -> String {
    facet_chips(clusters).into_iter().fold(String::new(), |mut css, chip| {
        let _ = write!(css, ".{0}:not(:checked)~section {1}{{display:none;}}.{0}:checked~.facet-bar>[for={0}]{{background:var(--secondary-container);color:var(--on-secondary-container);}}", chip.input_class, chip.hide_target);
        css
    })
}
