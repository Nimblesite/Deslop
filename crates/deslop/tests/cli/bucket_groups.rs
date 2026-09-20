//! E2E coverage for [FACET-HTML] (issue #257) and [CLONE-KIND-LABELS]:
//! the standalone HTML report must group cluster cards by their folded
//! clone kind into collapsible, kind-coloured `<details>` expanders and
//! title every card with that kind, so the reader sees what relation
//! each group holds without opening a pair view. Labels come from the
//! one kind registry, so the report, the terminal summary and the VS
//! Code panel speak the same words.

use deslop_test_support::write_dart_data_table_fixture;

use super::{
    language_sections::{RUST_A, RUST_B},
    support::*,
};
use crate::common::{
    cluster_kind, clusters, IDENTICAL_KIND, IDENTICAL_TITLE, NEARLY_IDENTICAL_KIND,
    NEARLY_IDENTICAL_TITLE, STRUCTURAL_ONLY_KIND, STRUCTURAL_ONLY_TITLE,
};

// Two byte-identical (Type-1) copies of one function — every occurrence
// is byte-equal to the canonical, so the cluster folds to the
// `identical` kind ([CLONE-KIND-FOLD]). Shaped around a `for` loop so it
// can never merge with the `while`-shaped renamed pair below.
const IDENTICAL_FN: &str = "pub fn checksum(values: &[i64]) -> i64 {\n\
                            let mut hash = 7;\n\
                            for value in values {\n\
                            hash = hash * 31 + value;\n\
                            if hash > 1000000 { hash = hash % 1000003; }\n\
                            }\n\
                            hash\n\
                            }\n";

/// The kind-keyed expander the HTML report opens for one kind: the
/// class carries the kind's CSS suffix so the colour rule can key on it,
/// the tooltip carries the clone-taxonomy name, the summary carries the
/// title and the live group count.
fn kind_expander(css_suffix: &str, taxonomy: &str, title: &str, groups: usize) -> String {
    format!(
        "<details class=\"clone-group clone-group--{css_suffix}\" open>\
         <summary title=\"{taxonomy}\">{title} — {groups} group(s)</summary>"
    )
}

/// One card title: the kind's title, tooltipped with its taxonomy name.
fn card_title(taxonomy: &str, title: &str) -> String {
    format!("<h3 class=\"cluster-card__title\" title=\"{taxonomy}\">{title}</h3>")
}

/// Seeds one exact (Type-1) pair and one renamed (Type-2) pair, runs the
/// CLI, and returns the rendered HTML body plus the parsed JSON report.
fn render_two_kind_report(tmp: &Path) -> Result<(String, Value)> {
    let scan_root = tmp.join("src");
    fs::create_dir_all(&scan_root)?;
    fs::write(scan_root.join("exact_a.rs"), IDENTICAL_FN)?;
    fs::write(scan_root.join("exact_b.rs"), IDENTICAL_FN)?;
    fs::write(scan_root.join("renamed_a.rs"), RUST_A)?;
    fs::write(scan_root.join("renamed_b.rs"), RUST_B)?;
    let out = outputs_under(tmp);
    let mut cmd = deslop_command(&scan_root, &tmp.join("report"))?;
    let _assertion = cmd
        .args([MIN_NODES_FLAG, MIN_NODES_VALUE])
        .assert()
        .success();
    Ok((fs::read_to_string(&out.html)?, read_json_report(&out.json)?))
}

/// The kinds the report folded, in rank order.
fn kinds_in_rank_order(json: &Value) -> Vec<String> {
    field(json, "clusters")
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|cluster| cluster_kind(cluster).to_owned())
        .collect()
}

// Implements [FACET-HTML] / [CLONE-KIND-LABELS] / #257: every reported
// cluster renders inside the expander of its folded kind, one expander
// per kind present, each carrying its live group count — no JS, so the
// report stays inert in the script-disabled VSIX tab and on file://.
// Cards carry the kind title and the cluster's mass; no pair evidence.
#[test]
fn html_report_groups_clusters_by_kind_into_coloured_expanders() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let (html, json) = render_two_kind_report(tmp.path())?;

    // [RANK-MASS-SUM] Exact and renamed clones keep their mass order.
    assert_eq!(
        kinds_in_rank_order(&json),
        vec![IDENTICAL_KIND, NEARLY_IDENTICAL_KIND],
        "corpus must yield the exact pair as identical and the renamed pair as \
         nearly identical, in that rank order: {json:#}"
    );
    for cluster in field(&json, "clusters").as_array().into_iter().flatten() {
        assert_eq!(
            field(cluster, "severity").as_str(),
            Some("warning"),
            "both clone kinds default to warning independently of rank: {cluster:#}"
        );
    }

    // Grouping: one collapsible, kind-coloured expander per kind present,
    // expanded by default so the top offender stays one glance away.
    assert_contains(
        &html,
        &kind_expander("identical", "Type-1 exact clone", IDENTICAL_TITLE, 1),
        "the exact pair renders inside the identical expander with its live count",
    );
    assert_contains(
        &html,
        &kind_expander(
            "nearly-identical",
            "Type-2 / close Type-3 clone",
            NEARLY_IDENTICAL_TITLE,
            1,
        ),
        "the renamed pair renders inside the nearly-identical expander with its live \
         count",
    );
    assert_eq!(
        html.matches("<details class=\"clone-group").count(),
        2,
        "exactly one expander per kind present — no empty groups: {html}"
    );
    assert_eq!(
        html.matches(&card_title("Type-1 exact clone", IDENTICAL_TITLE))
            .count(),
        1,
        "the exact pair's card carries the identical title"
    );
    assert_eq!(
        html.matches(&card_title(
            "Type-2 / close Type-3 clone",
            NEARLY_IDENTICAL_TITLE
        ))
        .count(),
        1,
        "the renamed pair's card carries the nearly-identical title"
    );
    assert!(
        html.contains("cluster-card cluster-card--identical")
            && html.contains("cluster-card cluster-card--nearly-identical"),
        "each card is keyed to its kind so the colour rule can find it: {html}"
    );
    assert_contains(
        &html,
        "mass ",
        "each card names the cluster's mass — the ranking metric",
    );

    // Retired axes: no bucket facet controls, category classes, or
    // neutral verdict remain.
    for retired in [
        "facet-identical",
        "facet-nearly-identical",
        "facet-same-behavior",
        "facet-structural-only",
        "facet-cat-",
        "Duplicate code",
    ] {
        assert!(
            !html.contains(retired),
            "retired bucket facet trace must stay gone: {retired}"
        );
    }

    // CSS-only contract stays intact: the report must remain script-free.
    assert_not_contains(
        &html,
        "<script",
        "the report must stay script-free ([OUTPUT-HUMAN-HTML])",
    );
    Ok(())
}

// [FACET-CLI] Severity is a diagnostic level; each card retains its clone kind and mass.
#[test]
fn html_report_summary_shows_diagnostic_severity_and_cards_carry_the_kind() -> Result<()> {
    let (tmp, scan_root, out) = seeded_scan("src", write_dart_data_table_fixture)?;
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let assertion = cmd.args([MIN_NODES_FLAG, "30"]).assert().success();
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).into_owned();
    assert_contains(
        &stderr,
        "diagnostic severity:",
        "stderr summary carries diagnostic levels, got",
    );
    for retired in ["data table", "code clones", "category"] {
        assert!(
            !stderr.to_lowercase().contains(retired),
            "stderr summary must not carry the retired {retired} breakdown"
        );
    }
    let html = fs::read_to_string(&out.html)?;
    let json = read_json_report(&out.json)?;

    // The scorer class differs only by name; the data table is noise.
    let clusters = field(&json, "clusters")
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(clusters.len(), 1, "corpus yields the single verbatim pair");
    let surviving = clusters
        .first()
        .ok_or_else(|| anyhow::anyhow!("one cluster survives: {json:#}"))?;
    assert_eq!(
        field(surviving, "severity").as_str(),
        Some("warning"),
        "a nearly identical pair defaults to warning"
    );
    assert_eq!(
        cluster_kind(surviving),
        NEARLY_IDENTICAL_KIND,
        "the class-level window differs by its class name, so the pair folds to \
         nearly identical ([CLONE-KIND-FOLD]): {surviving:#}"
    );
    assert_contains(
        &stderr,
        "1 × warning",
        "the breakdown names the diagnostic level, got",
    );

    for retired in [
        "cat-data",
        "cat-logic",
        "facet-cat-",
        "bucket:",
        "\"signals\"",
        "Duplicate code",
    ] {
        assert!(
            !html.contains(retired),
            "cards must not carry the retired classification {retired}"
        );
    }
    // Every card renders its kind title and a mass figure.
    assert_contains(
        &html,
        &card_title("Type-2 / close Type-3 clone", NEARLY_IDENTICAL_TITLE),
        "the card carries the nearly-identical kind title",
    );
    assert_contains(&html, "mass ", "mass figures render on every card");
    Ok(())
}

#[path = "bucket_groups/information.rs"]
pub(super) mod information;

#[path = "bucket_groups/order.rs"]
mod order;
