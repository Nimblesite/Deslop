//! [LANG-CAND-OSPREY-RECOVERY] An unfinished declaration cannot hide intact clones.

use super::*;

const FIXTURE: &str = "osprey-ml";
const FILES: [&str; 2] = ["alpha.ospml", "beta.ospml"];
const UNFINISHED_DECLARATION: &str = "\nunfinished value =\n    if value > then\n        value +\n";
const MIN_NODES: &str = "10";
const EXPECTED_CLUSTER_COUNT: usize = 1;
const EXPECTED_RANK: u64 = 1;
const EXPECTED_KIND: &str = "identical";
const EXPECTED_LANGUAGE: &str = "osprey-ml";

#[test]
fn intact_osprey_duplicates_survive_an_unfinished_neighboring_declaration() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(FIXTURE)?;
    let prefix = tmp.path().join(REPORT_OUTPUT_STEM);
    scan_intact_pair(&root, &prefix)?;
    let intact = fs::read_to_string(root.join(FILES[0]))?;
    fs::write(
        root.join(FILES[0]),
        format!("{intact}{UNFINISHED_DECLARATION}"),
    )?;
    scan_intact_pair(&root, &prefix)
}

fn scan_intact_pair(root: &Path, prefix: &Path) -> Result<()> {
    run_scan(root, prefix, &[MIN_NODES_FLAG, MIN_NODES])?;
    let json = fs::read_to_string(with_ext(prefix, "json"))?;
    assert_intact_pair(&json)
}

fn assert_intact_pair(json: &str) -> Result<()> {
    let clusters = report_clusters(json)?;
    assert_eq!(
        clusters.len(),
        EXPECTED_CLUSTER_COUNT,
        "intact functions still form their clone cluster"
    );
    let cluster = require_cluster_spanning(&clusters, FILES[0], FILES[1])?;
    assert_eq!(field(cluster, "rank").as_u64(), Some(EXPECTED_RANK));
    assert_eq!(field(cluster, "kind").as_str(), Some(EXPECTED_KIND));
    let occurrences = require_array(cluster, "/occurrences", "intact ML pair")?;
    assert_eq!(occurrences.len(), FILES.len());
    for occurrence in occurrences {
        assert_intact_occurrence(occurrence)?;
    }
    Ok(())
}

fn assert_intact_occurrence(occurrence: &Value) -> Result<()> {
    assert_eq!(
        field(occurrence, "language").as_str(),
        Some(EXPECTED_LANGUAGE)
    );
    let source = occurrence_source(&fixture(FIXTURE), occurrence)
        .expect("the reported occurrence must resolve to the intact fixture function");
    assert_eq!(source, fs::read(fixture(FIXTURE).join(FILES[0]))?);
    Ok(())
}
