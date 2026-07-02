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

/// Writes the facet controls: one visually-hidden checkbox per bucket
/// present (direct children of the report shell, so the sibling
/// selectors in [`facet_css`] can reach the sections that follow) plus
/// a labelled chip row. A report with fewer than two buckets gets no
/// controls — a filter with one choice filters nothing.
pub(super) fn write_facet_controls(out: &mut String, clusters: &[ReportCluster]) {
    let present = buckets_present(clusters);
    if present.len() < 2 {
        return;
    }
    for kind in &present {
        let suffix = bucket_labels(*kind).css_suffix;
        let _ = write!(
            out,
            "<input type=\"checkbox\" id=\"facet-{suffix}\" class=\"facet-input facet-{suffix}\" checked>",
        );
    }
    out.push_str("<div class=\"facet-bar\"><span class=\"facet-bar__label\">Show:</span>");
    for kind in &present {
        let labels = bucket_labels(*kind);
        let _ = write!(
            out,
            "<label class=\"facet-chip\" for=\"facet-{suffix}\">{title}</label>",
            suffix = labels.css_suffix,
            title = escape(labels.plain_title),
        );
    }
    out.push_str("</div>");
}

/// Per-bucket facet CSS: one hide rule (an unchecked facet hides its
/// bucket's groups in every following section) and one checked-chip
/// highlight rule per bucket present in the report. Derived from the
/// canonical registry via [`buckets_present`] so the selector set can
/// never drift, and absent buckets leave zero facet traces in the page.
pub(super) fn facet_css(clusters: &[ReportCluster]) -> String {
    buckets_present(clusters)
        .into_iter()
        .fold(String::new(), |mut css, kind| {
            let suffix = bucket_labels(kind).css_suffix;
            let _ = write!(
                css,
                ".facet-{suffix}:not(:checked)~section .bucket-group.kind-{suffix}{{display:none;}}\
                 .facet-{suffix}:checked~.facet-bar>[for=facet-{suffix}]{{background:var(--secondary-container);color:var(--on-secondary-container);}}"
            );
            css
        })
}
