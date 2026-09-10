//! [CORPUS-PRECISION] What the data-table check says when it fires.
//!
//! A gate is only worth what its message is worth: a reader who is told a
//! finding is empty, or sent to the wrong rank, cannot act on it. These pin
//! the three things `data_table_rank` got wrong (gh #540) — all three already
//! correct in the sibling `boilerplate_rank` check.

use anyhow::{anyhow, Context, Result};
use deslop_core::wire_generated::{ReportCluster, ReportOccurrence};
use serde_json::Value;

use super::{data_character_ratio, data_table_failure, DATA_TABLE_RATIO};
use crate::corpus::Failure;

/// A font-metrics table: digits, brackets, commas and dots, and nothing else.
const NUMERIC_TABLE: &str =
    "989:[.08167,.58167,0,0,.77778],1008:[0,.43056,.04028,0,.66667],8245:[0,.54986,0,0,.275]";
/// Ordinary duplicated logic, which must never be called a data table.
const REAL_LOGIC: &str = "fn summarise(rows: &[Row]) -> Summary { let total = rows.iter()\
     .map(|row| row.amount).sum(); Summary::new(total) }";

/// How many copies the cluster under test reports. The number a reader is
/// told must be this one.
const OCCURRENCES: usize = 34;
/// The zero-based index of the last cluster the check looks at, whose rank in
/// the report — and in the message — is one greater.
const LAST_HEAD_POSITION: usize = 9;
const LAST_HEAD_RANK: usize = 10;

const FIXTURE_PATH: &str = "internal/warpc/js/renderkatex.bundle.js";
const CHECK_NAME: &str = "data_table_rank";
const CLUSTER_ID: &str = "f88c8af320ac7e79";
const RANK_BAND: &str = "worst";
const MASS: u64 = 1584;
const CANONICAL_NODES: usize = 56;

/// A rendered cluster carrying `OCCURRENCES` copies, as JSON, exactly as the
/// check receives it from a report.
///
/// Built from the wire model rather than hand-written JSON, so the field names
/// the check reads are the field names a real report emits — reading one that
/// is not there is the whole of gh #540.
fn cluster() -> Result<Value> {
    let occurrences: Vec<ReportOccurrence> = (0..OCCURRENCES)
        .map(|index| ReportOccurrence {
            path: FIXTURE_PATH.into(),
            start_byte: index,
            end_byte: index.saturating_add(1),
            start_line: 1,
            end_line: 1,
            hidden: false,
            in_diff: None,
        })
        .collect();
    let cluster = ReportCluster {
        id: CLUSTER_ID.to_owned(),
        rank: 1,
        rank_band: RANK_BAND.to_owned(),
        mass: MASS,
        canonical_node_count: CANONICAL_NODES,
        occurrences_total: occurrences.len(),
        occurrence_count: occurrences.len(),
        occurrences_truncated: false,
        occurrences,
        intersects_diff: None,
        is_newly_introduced: None,
    };
    serde_json::to_value(cluster).context("a wire cluster serialises")
}

/// The failure the check raises for `text` at `position`, as an error when it
/// raises none. Taken with `?` rather than `expect`, so a fixture that stopped
/// tripping the check fails by name instead of through a denied panic.
fn failure_for(position: usize, text: &str) -> Result<Failure> {
    data_table_failure(position, &cluster()?, text)
        .ok_or_else(|| anyhow!("a table of digits at the head of the report is a breach"))
}

#[test]
fn a_ranked_data_table_is_reported_with_the_number_of_copies_it_actually_has() -> Result<()> {
    let failure = failure_for(0, NUMERIC_TABLE)?;
    assert_eq!(
        failure.check, CHECK_NAME,
        "the failure must keep its check id"
    );
    assert!(
        failure
            .detail
            .contains(&format!("{OCCURRENCES} occurrences")),
        "the message must name the {OCCURRENCES} copies the cluster reports. A reader told \
         a finding has 0 occurrences concludes the report contains an empty cluster, which \
         is not what happened. Message: {}",
        failure.detail,
    );
    assert!(
        !failure.detail.contains("0 occurrences"),
        "reading a field the report does not emit is what produced `0 occurrences` for \
         every breach. Message: {}",
        failure.detail,
    );
    Ok(())
}

#[test]
fn the_rank_in_the_message_is_the_rank_in_the_report() -> Result<()> {
    let failure = failure_for(LAST_HEAD_POSITION, NUMERIC_TABLE)?;
    assert!(
        failure.detail.contains(&format!("rank {LAST_HEAD_RANK}")),
        "ranks are one-based in the report, so the cluster at index {LAST_HEAD_POSITION} is \
         rank {LAST_HEAD_RANK}. A message naming rank {LAST_HEAD_POSITION} sends the reader \
         to the wrong finding. Message: {}",
        failure.detail,
    );
    Ok(())
}

#[test]
fn the_message_never_claims_a_category_the_report_does_not_carry() -> Result<()> {
    let failure = failure_for(0, NUMERIC_TABLE)?;
    for absent in ["categorised", "absent"] {
        assert!(
            !failure.detail.contains(absent),
            "no cluster carries a category — [RANK-CATEGORY] says a detection-time finding \
             kind is not carried as cluster metadata — so a message that reports one is \
             telling the reader about a field that does not exist, and the exemption it \
             implies can never open. Message: {}",
            failure.detail,
        );
    }
    Ok(())
}

#[test]
fn ordinary_duplicated_logic_is_not_called_a_data_table() -> Result<()> {
    assert!(
        data_character_ratio(REAL_LOGIC) < DATA_TABLE_RATIO,
        "identifiers and keywords must keep real logic far below the {DATA_TABLE_RATIO} \
         data-character ratio; measured {}",
        data_character_ratio(REAL_LOGIC),
    );
    assert!(
        data_table_failure(0, &cluster()?, REAL_LOGIC).is_none(),
        "a cluster of real duplicated logic is exactly what the report is for; failing it \
         here would make the gate demand that genuine clones be demoted",
    );
    Ok(())
}
