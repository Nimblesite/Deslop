//! E2E coverage for [FACET-HTML] (issue #257): the standalone HTML
//! report must group cluster cards by clone bucket into collapsible
//! `<details>` expanders and carry CSS-only bucket facet controls, so
//! the reader is never forced to scroll one flat page. Labels come from
//! the shared bucket registry so the report and the VS Code panel speak
//! the same words ([CLONE-BUCKETS-DUAL-LABEL]).

use deslop_test_support::write_dart_data_table_fixture;

use super::language_sections::{RUST_A, RUST_B};
use super::support::*;

// Two byte-identical (Type-1) copies of one function — saturates both
// the structural and token signals, routing to the `identical` bucket
// ([CLONE-BUCKETS-ROUTING]). Shaped around a `for` loop so it can never
// merge with the `while`-shaped renamed pair below.
const IDENTICAL_FN: &str = "pub fn checksum(values: &[i64]) -> i64 {\n\
                            let mut hash = 7;\n\
                            for value in values {\n\
                            hash = hash * 31 + value;\n\
                            if hash > 1000000 { hash = hash % 1000003; }\n\
                            }\n\
                            hash\n\
                            }\n";

/// Seeds one exact (Type-1) pair and one renamed (Type-2) pair, runs the
/// CLI, and returns the rendered HTML body plus the parsed JSON report.
fn render_two_bucket_report(tmp: &Path) -> Result<(String, Value)> {
    let scan_root = tmp.join("src");
    fs::create_dir_all(&scan_root)?;
    fs::write(scan_root.join("exact_a.rs"), IDENTICAL_FN)?;
    fs::write(scan_root.join("exact_b.rs"), IDENTICAL_FN)?;
    fs::write(scan_root.join("renamed_a.rs"), RUST_A)?;
    fs::write(scan_root.join("renamed_b.rs"), RUST_B)?;
    let out = outputs_under(tmp);
    let mut cmd = deslop_command(&scan_root, &tmp.join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    Ok((fs::read_to_string(&out.html)?, read_json_report(&out.json)?))
}

/// The `bucket` wire labels of every cluster in `report`, in rank order.
fn cluster_buckets(report: &Value) -> Vec<String> {
    field(report, "clusters")
        .as_array()
        .map(|clusters| {
            clusters
                .iter()
                .filter_map(|cluster| field(cluster, "bucket").as_str())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

// Implements [FACET-HTML] / #257: clusters are grouped by bucket into
// collapsible expanders whose summaries carry the shared plain titles,
// with a CSS-only facet checkbox per bucket present — no JS, so the
// report stays inert in the script-disabled VSIX tab and on file://.
#[test]
fn html_report_groups_clusters_into_bucket_expanders_with_facets() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let (html, json) = render_two_bucket_report(tmp.path())?;

    // Corpus guard: the seeded pairs must land in two distinct buckets,
    // otherwise every assertion below would fail for the wrong reason.
    let buckets = cluster_buckets(&json);
    assert!(
        buckets.iter().any(|bucket| bucket == "identical")
            && buckets.iter().any(|bucket| bucket == "nearly_identical"),
        "corpus must yield one identical and one nearly_identical cluster, got {buckets:?}"
    );

    // Grouping: one collapsible expander per bucket present.
    assert!(
        html.contains("<details class=\"bucket-group kind-identical\""),
        "identical clusters must render inside a collapsible bucket group"
    );
    assert!(
        html.contains("<details class=\"bucket-group kind-nearly-identical\""),
        "nearly-identical clusters must render inside a collapsible bucket group"
    );
    let identical_count = buckets
        .iter()
        .filter(|bucket| *bucket == "identical")
        .count();
    let nearly_count = buckets
        .iter()
        .filter(|bucket| *bucket == "nearly_identical")
        .count();
    assert!(
        html.contains(&format!("Identical code — {identical_count} group(s)")),
        "the identical expander summary must carry the shared plain title and the \
         canonical JSON's live count ({identical_count})"
    );
    assert!(
        html.contains(&format!("Nearly identical code — {nearly_count} group(s)")),
        "the nearly-identical expander summary must carry the shared plain title and \
         the canonical JSON's live count ({nearly_count})"
    );
    assert!(
        html.contains("\" open><summary"),
        "the worst bucket group starts expanded so the top offender stays one glance away"
    );

    // Facet controls: one checkbox per bucket present, labelled with the
    // panel's words; absent buckets get no control.
    assert!(
        html.contains("id=\"facet-identical\"") && html.contains("id=\"facet-nearly-identical\""),
        "a bucket facet checkbox is rendered for each bucket present"
    );
    assert!(
        html.contains("<label class=\"facet-chip\" for=\"facet-identical\">Identical code</label>"),
        "the facet label uses the shared bucket plain title"
    );
    assert!(
        !html.contains("facet-same-behavior") && !html.contains("facet-structural-only"),
        "buckets absent from the report get no facet control"
    );

    // CSS-only contract: unchecking a facet hides its group via a sibling
    // selector — never a script.
    assert!(
        html.contains(".facet-identical:not(:checked)"),
        "the inline CSS carries the facet hide rule"
    );
    assert!(
        !html.contains("<script"),
        "the report must stay script-free ([OUTPUT-HUMAN-HTML])"
    );
    // Single-category corpus: the category axis contributes no controls
    // and leaves zero traces — a filter with one choice filters nothing.
    assert!(
        !html.contains("facet-cat-"),
        "a single-category report gets no category facet controls"
    );
    Ok(())
}

/// The `category` wire labels of every cluster in `report`, in rank order.
fn cluster_categories(report: &Value) -> Vec<String> {
    field(report, "clusters")
        .as_array()
        .map(|clusters| {
            clusters
                .iter()
                .filter_map(|cluster| field(cluster, "category").as_str())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

// Implements [FACET-HTML] / [FACET-MODEL]: the category axis facets the
// report alongside the bucket axis — every card carries a `cat-<wire>`
// class, and a CSS-only chip per category present hides that category's
// cards. Labels come from the shared registry (`group_title`), so the
// chip-less logic category reads "Code clones" here and in the panel.
#[test]
fn html_report_carries_category_facets_and_card_classes() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    write_dart_data_table_fixture(&scan_root)?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let assertion = cmd.args(["--min-nodes", "30"]).assert().success();
    // [FACET-CLI]: the stderr summary carries a category breakdown line
    // for non-logic categories, driven by the same registry as the chips.
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).into_owned();
    assert!(
        stderr.contains("1 × data table"),
        "stderr summary must carry the category breakdown line, got:\n{stderr}"
    );
    let html = fs::read_to_string(&out.html)?;
    let json = read_json_report(&out.json)?;

    // Corpus guard: both categories must be present, otherwise the facet
    // assertions below would fail for the wrong reason.
    let categories = cluster_categories(&json);
    assert!(
        categories.iter().any(|category| category == "data")
            && categories.iter().any(|category| category == "logic"),
        "corpus must yield one data and one logic cluster, got {categories:?}"
    );

    assert!(
        html.contains(" cat-data\"") && html.contains(" cat-logic\""),
        "every cluster card carries its category class"
    );
    assert!(
        html.contains("id=\"facet-cat-data\"") && html.contains("id=\"facet-cat-logic\""),
        "a category facet checkbox is rendered per category present"
    );
    assert!(
        html.contains("<label class=\"facet-chip\" for=\"facet-cat-logic\">Code clones</label>"),
        "the chip-less logic category uses the shared plain group title"
    );
    assert!(
        html.contains("<label class=\"facet-chip\" for=\"facet-cat-data\">data table</label>"),
        "the data category chip reuses the shared category chip label"
    );
    assert!(
        html.contains(
            ".facet-cat-data:not(:checked)~section .cluster-card.cat-data{display:none;}"
        ),
        "the inline CSS hides a category's cards when its facet is unchecked"
    );
    Ok(())
}
