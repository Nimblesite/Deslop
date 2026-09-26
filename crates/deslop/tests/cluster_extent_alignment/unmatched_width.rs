//! [PIPELINE-CLUSTER-EXACT-SCOPE-MATCHED] A wider view is the finding
//! only when another copy matches the extra width (#505).
//!
//! Three page generators each hold the same pair of page-writing
//! functions. The yaml generator also declares the option type its pages
//! list, directly above the pair. The functions are the duplication; the
//! type exists once. The report used to publish the yaml copy as "type
//! plus functions" beside two bare pairs of functions — cobra's
//! `doc/yaml_docs.go` did the same with `cmdOption` and `cmdDoc` — which
//! counted the type's rows in `duplicated_loc`.

use std::fs;

use anyhow::Result;
use serde_json::Value;

use crate::common::{go_scope::*, scan_dir::temp_scan_dir, *};

/// The three generators.
const FIXTURE: &str = "go-cluster-extent-unmatched-width";

/// A generator with no type above its functions.
const MARKDOWN: &str = "markdown_pages.go";
/// A second generator without the type, so the cluster has more than one
/// partner.
const REST: &str = "rest_pages.go";
/// The generator that declares its option type above its functions.
const YAML: &str = "yaml_pages.go";

/// The type only the yaml generator declares. No occurrence may hold it.
const YAML_ONLY_TYPE: &str = "pageOption";

/// Every floor the functions clear, so the rule holds wherever the type
/// and the functions are separate candidates.
const THRESHOLDS: [u32; 4] = [8, 12, 20, 30];

/// The rows every copy must report, and nothing more: same-shape copies
/// cover the same rows, and the yaml file counts only its functions.
fn assert_walker_only(
    scan_root: &std::path::Path,
    report: &Value,
    files: &[&str],
    label: &str,
) -> Result<()> {
    let cluster = expect_cluster_spanning(report, files)?;
    assert_eq!(
        occurrences(cluster).len(),
        files.len(),
        "{label}: one copy of the functions in each generator: {cluster:#}"
    );
    assert_cluster_scope(scan_root, cluster, label)?;
    assert_no_unshared_symbol(scan_root, cluster, YAML_ONLY_TYPE, label)?;
    assert_symmetric_rows(cluster, label);
    assert_cluster_opens_declarations(scan_root, cluster, label)?;
    assert_eq!(
        duplicated_loc_for(report, YAML),
        duplicated_loc_for(report, MARKDOWN),
        "{label}: the yaml generator duplicates exactly the functions the \
         markdown generator does, so the type's rows must not reach \
         duplicated_loc: {report:#}"
    );
    Ok(())
}

#[test]
fn three_generators_publish_the_shared_functions_not_the_type_above_one() -> Result<()> {
    let scan_root = fixture(FIXTURE);
    for min_nodes in THRESHOLDS {
        let label = format!("{FIXTURE} --min-nodes {min_nodes}");
        let report = run_report(&scan_root, min_nodes)?;
        assert_walker_only(&scan_root, &report, &[MARKDOWN, REST, YAML], &label)?;
    }
    Ok(())
}

#[test]
fn two_generators_publish_the_shared_functions_not_the_type_above_one() -> Result<()> {
    let (_tmp, scan_root) = temp_scan_dir(FIXTURE)?;
    for file in [MARKDOWN, YAML] {
        let _bytes = fs::copy(fixture(FIXTURE).join(file), scan_root.join(file))?;
    }
    for min_nodes in THRESHOLDS {
        let label = format!("{MARKDOWN} + {YAML} --min-nodes {min_nodes}");
        let report = run_report(&scan_root, min_nodes)?;
        assert_walker_only(&scan_root, &report, &[MARKDOWN, YAML], &label)?;
    }
    Ok(())
}
