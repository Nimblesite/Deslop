//! [FUSED-CONTENT-GATE-CALL-TARGET-PADDING] A judged command factory retains its copied setup.

use super::*;

const GROUP_SIDES: [&str; 2] = ["together.go", "one.go"];
const FIXTURE: &str = "grouped-command-factories";
const FACTORY_START: u64 = 4;
const FACTORY_END: u64 = 28;
const FACTORY_CONTEXT: &str = "the complete copied command factory";
const FACTORY_NODES: u64 = 165;
const FACTORY_RANK: u64 = 2;
const NODES_FIELD: &str = "canonical_node_count";
const RANK_FIELD: &str = "rank";

fn factory_cluster(report: &Value) -> Result<Value> {
    clone_findings(report)
        .into_iter()
        .find(|cluster| {
            GROUP_SIDES
                .iter()
                .all(|side| has_side_span(cluster, side, FACTORY_START, FACTORY_END))
        })
        .with_context(|| format!("{FACTORY_CONTEXT}: {report:#}"))
}

#[test]
fn grouped_command_factories_retain_the_complete_copied_setup() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    let cluster = factory_cluster(&report)?;
    for side in GROUP_SIDES {
        assert!(
            has_side_span(&cluster, side, FACTORY_START, FACTORY_END),
            "{report:#}"
        );
    }
    assert_eq!(cluster_size(&cluster), OCCURRENCE_COUNT);
    assert_eq!(cluster_kind(&cluster), NEARLY_IDENTICAL_KIND);
    assert_eq!(field(&cluster, NODES_FIELD).as_u64(), Some(FACTORY_NODES));
    assert_eq!(field(&cluster, RANK_FIELD).as_u64(), Some(FACTORY_RANK));
    Ok(())
}
