//! E2E regression for [REPAIR-RENAME-LITERAL-ECHO] — the boundary of the
//! literal echo that certifies a rename ([FUSED-CONTENT-GATE],
//! [TECH-PMATCH-BAKER], gh #409, gh #410).
//!
//! A literal that spells the name of a renamed symbol is part of the
//! rename: `"OrderService"` becoming `"UserService"` alongside the
//! `OrderService` symbol is what makes the rename complete, and
//! [`rename_literal_monotonicity`] pins that it must never score below
//! the half-finished version.
//!
//! Substituting the elected identifier *anywhere in the bytes* buys that
//! certification far too cheaply. `id -> key` rewrites `"invalid
//! request"` into `"invalkey request"` — a changed message no developer
//! renamed and no reader would accept — and the echo rule then reads the
//! difference as proof of a thorough rename. The substitution must land
//! on symbol boundaries.
//!
//! Both directions live in ONE scan surface so a fix for either can never
//! trade away the other, and the separation asserted here is **strict**:
//! `>=` is satisfied by a detector that certifies both fixtures at 1.0,
//! which is precisely the defect. `ts-rename-literal-consistent` is the
//! intended full-symbol echo and must stay a proven rename;
//! `ts-rename-literal-substring` is the same rename with one mangled
//! message and must stay a demoted, uncertified `structural_only` match.

use crate::common::{signals::*, *};

use serde_json::Value;

/// Node floor matching the rename suites, so the class body qualifies as
/// a candidate on both sides of both fixtures.
const MIN_NODES: u32 = 12;

/// The fixture whose changed literal is a full-symbol echo of the rename.
const CONSISTENT: &str = "ts-rename-literal-consistent";

/// The fixture whose changed literal is only a byte-substring collision.
const SUBSTRING: &str = "ts-rename-literal-substring";

/// The canonical half of the rename, present in both fixtures.
const ORDER_GATEWAY: &str = "order_gateway.ts";

/// The renamed half, present in both fixtures.
const USER_GATEWAY: &str = "user_gateway.ts";

/// Both gateways, and nothing else, in each fixture.
const FILES_ANALYSED: u64 = 2;

/// One occurrence per gateway.
const OCCURRENCES: u64 = 2;

/// First line of the matched view in every gateway — the whole class.
const VIEW_FIRST_LINE: u64 = 1;

/// Last line of the matched view in the echo fixture.
const CONSISTENT_LAST_LINE: u64 = 15;

/// Last line of the matched view in the substring fixture, three lines
/// longer for the guard clause carrying the mangled message.
const SUBSTRING_LAST_LINE: u64 = 18;

/// Scans one fixture and returns the report plus its one cluster
/// spanning both gateways.
fn gateway_cluster(report: &Value) -> Result<&Value> {
    expect_cluster_spanning(report, &[ORDER_GATEWAY, USER_GATEWAY])
}

/// Asserts every occurrence of `cluster` spans exactly `first..=last`.
fn assert_view(cluster: &Value, first: u64, last: u64, label: &str) {
    for occurrence in occurrences(cluster) {
        assert_eq!(
            field(occurrence, "start_line").as_u64(),
            Some(first),
            "{label}: the reported view begins at the class declaration in \
             every occurrence — {dump}",
            dump = signal_dump(cluster)
        );
        assert_eq!(
            field(occurrence, "end_line").as_u64(),
            Some(last),
            "{label}: the reported view covers the whole class in every \
             occurrence — {dump}",
            dump = signal_dump(cluster)
        );
    }
}

#[test]
fn a_substring_collision_never_certifies_a_rename_the_way_a_symbol_echo_does() -> Result<()> {
    let echo_root = fixture(CONSISTENT);
    let substring_root = fixture(SUBSTRING);
    let echo_report = run_report(&echo_root, MIN_NODES)?;
    let substring_report = run_report(&substring_root, MIN_NODES)?;
    let echo = gateway_cluster(&echo_report)?;
    let mangled = gateway_cluster(&substring_report)?;

    for (report, label) in [(&echo_report, CONSISTENT), (&substring_report, SUBSTRING)] {
        assert_eq!(
            field(report, "files_analysed").as_u64(),
            Some(FILES_ANALYSED),
            "{label}: both gateways must be parsed — a scan that skipped one \
             could satisfy this test by measuring nothing: {report:#}"
        );
        assert_eq!(
            cluster_count(report),
            1,
            "{label}: the two gateways are the only duplication in the \
             fixture: {lines:#?}",
            lines = visible_cluster_lines(report)
        );
    }

    // The positive control: renaming the literal *with* its symbol is the
    // rename done properly, and must stay a certified act-now clone.
    assert_eq!(
        cluster_size(echo),
        OCCURRENCES,
        "the echo fixture has exactly two occurrences — {dump}",
        dump = signal_dump(echo)
    );
    assert_no_pair_surface_on_cluster(echo, CONSISTENT);
    assert_view(echo, VIEW_FIRST_LINE, CONSISTENT_LAST_LINE, CONSISTENT);
    assert_proven_rename_contract(&echo_root, echo, CONSISTENT)?;

    // The boundary: one message mangled mid-word by the same substitution
    // is a content difference the rename does not explain.
    assert_eq!(
        cluster_size(mangled),
        OCCURRENCES,
        "the substring fixture has exactly two occurrences — {dump}",
        dump = signal_dump(mangled)
    );
    assert_no_pair_surface_on_cluster(mangled, SUBSTRING);
    assert_view(mangled, VIEW_FIRST_LINE, SUBSTRING_LAST_LINE, SUBSTRING);
    assert_structural_only_contract(mangled, SUBSTRING);

    // Strict separation. The old oracle — rendered rename consistency —
    // was pair-scoped evidence; the mass-only wire carries no rename
    // surface on clusters ([PIPELINE-CLUSTER-CLOSURE]), so the
    // certification distinction is asserted as the clean-surface
    // negative: neither cluster may carry a signals block, a bucket, or
    // a verdict a consumer could read as a certified rename. The echo's
    // admission half is the proven-rename contract; the mangled one is
    // the plain structural contract.
    assert_no_pair_surface_on_cluster(echo, CONSISTENT);
    assert_no_pair_surface_on_cluster(mangled, SUBSTRING);
    Ok(())
}
