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

// Implements [FACET-HTML] / #257, re-pinned to the mass-only wire: every
// reported cluster renders inside ONE neutral collapsible expander whose
// summary carries the live group count — no JS, so the report stays inert
// in the script-disabled VSIX tab and on file://. Cards show the neutral
// verdict and the cluster's mass; no similarity classification exists.
#[test]
fn html_report_groups_clusters_into_one_neutral_expander() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let (html, json) = render_two_bucket_report(tmp.path())?;

    // Corpus guard: the seeded pairs must yield two ranked clusters, and
    // the engine stamps every cluster with a mass band.
    let clusters = field(&json, "clusters")
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(clusters.len(), 2, "corpus must yield two clusters");
    for cluster in &clusters {
        let band = cluster.get("rank_band").and_then(|v| v.as_str());
        assert!(
            band.is_some(),
            "every cluster carries the engine's rank band"
        );
    }

    // Grouping: one collapsible neutral expander holding both groups,
    // expanded by default so the top offender stays one glance away.
    assert!(
        html.contains(
            "<details class=\"clone-group\" open><summary>Duplicate code — 2 group(s)</summary>"
        ),
        "all clusters render inside the single neutral expander with the live count"
    );
    assert_eq!(
        html.matches(">Duplicate code</h3>").count(),
        2,
        "each cluster card carries the neutral verdict title"
    );
    assert!(
        html.contains("mass "),
        "each card names the cluster's mass — the ranking metric"
    );

    // Retired axes: no bucket/category facet controls or classes remain.
    for retired in [
        "facet-identical",
        "facet-nearly-identical",
        "facet-same-behavior",
        "facet-structural-only",
        "facet-cat-",
        "kind-identical",
        "Identical code",
        "Nearly identical code",
    ] {
        assert!(
            !html.contains(retired),
            "retired bucket facet trace must stay gone: {retired}"
        );
    }

    // CSS-only contract stays intact: the report must remain script-free.
    assert!(
        !html.contains("<script"),
        "the report must stay script-free ([OUTPUT-HUMAN-HTML])"
    );
    Ok(())
}

// Implements [FACET-HTML] / [FACET-CLI], re-pinned to the mass-only wire:
// the stderr summary breaks the report down by mass severity band — never
// by similarity category — and no card carries a category class or a
// category facet control ([FACET-MODEL]: the category axis is retired).
#[test]
fn html_report_summary_breaks_down_by_mass_severity_and_cards_stay_neutral() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    write_dart_data_table_fixture(&scan_root)?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let assertion = cmd.args(["--min-nodes", "30"]).assert().success();
    // [FACET-CLI]: the stderr summary carries the mass-severity breakdown.
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).into_owned();
    assert!(
        stderr.contains("mass severity:"),
        "stderr summary must carry the mass-severity breakdown line, got:\n{stderr}"
    );
    for retired in ["data table", "code clones", "category"] {
        assert!(
            !stderr.to_lowercase().contains(retired),
            "stderr summary must not carry the retired {retired} breakdown"
        );
    }
    let html = fs::read_to_string(&out.html)?;
    let json = read_json_report(&out.json)?;

    // Corpus guard: the engine reports the verbatim scorer pair and stamps
    // its band; the data table no longer survives the noise/collapse rules.
    let clusters = field(&json, "clusters")
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(clusters.len(), 1, "corpus yields the single verbatim pair");
    assert!(
        clusters
            .first()
            .and_then(|cluster| cluster.get("rank_band"))
            .and_then(|v| v.as_str())
            == Some("worst"),
        "the surviving pair is the report's worst cluster"
    );
    assert!(
        stderr.contains("1 × worst"),
        "the breakdown names the surviving band, got:\n{stderr}"
    );

    for retired in [
        "cat-data",
        "cat-logic",
        "facet-cat-",
        "bucket:",
        "\"signals\"",
    ] {
        assert!(
            !html.contains(retired),
            "cards must not carry the retired classification {retired}"
        );
    }
    // Every card still renders the neutral verdict and a mass figure.
    assert!(
        html.contains(">Duplicate code</h3>"),
        "neutral card titles render"
    );
    assert!(html.contains("mass "), "mass figures render on every card");
    Ok(())
}
