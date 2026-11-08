//! [FUSED-CONTENT-GATE-RENAME] [FUSED-SHARED-SUBTREE-CORE] Two Osprey effect
//! handlers with one shape, eight renamed names, five kept names and no
//! surviving literal are a different vocabulary over one shape, not a copy.
//! Neither the aligned pair nor a lopsided window of it may be reported; the
//! verbatim pair beside them stays the only finding.

use super::*;

const FIXTURE: &str = "osprey-renamed-handlers";
const HANDLERS: [&str; 2] = ["drafts.osp", "notes.osp"];
const COPIES: [&str; 2] = ["journal.osp", "ledger.osp"];
const EXPECTED_FILES: u64 = 4;
const EXPECTED_CLUSTER_COUNT: usize = 1;
const EXPECTED_RANK: u64 = 1;
const EXPECTED_KIND: &str = "identical";
const EXPECTED_LANGUAGE: &str = "osprey";

#[test]
fn renamed_osprey_handlers_with_no_surviving_literal_are_not_reported() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(FIXTURE)?;
    let prefix = tmp.path().join(REPORT_OUTPUT_STEM);
    run_scan(&root, &prefix, &[])?;
    let json = fs::read_to_string(with_ext(&prefix, "json"))?;
    assert_eq!(
        files_analysed(&json)?,
        EXPECTED_FILES,
        "every fixture file is analysed"
    );
    let clusters = report_clusters(&json)?;
    for cluster in &clusters {
        let files = cluster_file_basenames(cluster);
        for handler in HANDLERS {
            assert!(
                !files.contains(handler),
                "{handler} must be in no cluster: {} holds {files:?}",
                cluster_id(cluster)
            );
        }
    }
    assert_eq!(
        clusters.len(),
        EXPECTED_CLUSTER_COUNT,
        "only the verbatim copy is a finding: {clusters:#?}"
    );
    assert_verbatim_copy(
        require_cluster_spanning(&clusters, COPIES[0], COPIES[1])?,
        &root,
    )
}

fn assert_verbatim_copy(cluster: &Value, root: &Path) -> Result<()> {
    assert_eq!(field(cluster, "rank").as_u64(), Some(EXPECTED_RANK));
    assert_eq!(field(cluster, "kind").as_str(), Some(EXPECTED_KIND));
    let occurrences = require_array(cluster, "/occurrences", "verbatim copy")?;
    assert_eq!(occurrences.len(), COPIES.len());
    for occurrence in occurrences {
        assert_eq!(
            field(occurrence, "language").as_str(),
            Some(EXPECTED_LANGUAGE)
        );
        let source = occurrence_source(root, occurrence)
            .ok_or_else(|| anyhow::anyhow!("the occurrence must resolve to fixture bytes"))?;
        assert_eq!(
            String::from_utf8(source)?.trim_end(),
            fs::read_to_string(root.join(COPIES[0]))?.trim_end(),
            "the copy is the whole file"
        );
    }
    Ok(())
}
