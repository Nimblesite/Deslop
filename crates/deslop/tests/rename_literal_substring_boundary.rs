//! E2E regression for [REPAIR-RENAME-LITERAL-ECHO] — the boundary of the
//! literal echo that certifies a rename ([FUSION-CONTENT-GATE],
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

use deslop_core::buckets::CONTENT_SUPPORT_FLOOR;
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

/// The bucket a certified consistent rename lands in.
const NEARLY_IDENTICAL: &str = "nearly_identical";

/// The demoted bucket a rename with an unexplained content difference
/// lands in.
const STRUCTURAL_ONLY: &str = "structural_only";

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
    assert_eq!(
        cluster_bucket(echo),
        NEARLY_IDENTICAL,
        "a rename whose every literal is accounted for is nearly \
         identical — {dump}",
        dump = signal_dump(echo)
    );
    assert_view(echo, VIEW_FIRST_LINE, CONSISTENT_LAST_LINE, CONSISTENT);
    assert_proven_rename_contract(&echo_root, echo, CONSISTENT)?;
    assert!(
        signal(echo, "rename_consistency") >= CONTENT_SUPPORT_FLOOR,
        "{CONSISTENT}: a full-symbol echo is the certification case and \
         must clear the content-support floor — {dump}",
        dump = signal_dump(echo)
    );

    // The boundary: one message mangled mid-word by the same substitution
    // is a content difference the rename does not explain.
    assert_eq!(
        cluster_size(mangled),
        OCCURRENCES,
        "the substring fixture has exactly two occurrences — {dump}",
        dump = signal_dump(mangled)
    );
    assert_eq!(
        cluster_bucket(mangled),
        STRUCTURAL_ONLY,
        "`\"invalid request\"` becoming `\"invalkey request\"` is a changed \
         message, not a renamed symbol; certifying it promotes a cluster \
         whose content evidence contradicts the rename — {dump}",
        dump = signal_dump(mangled)
    );
    assert_view(mangled, VIEW_FIRST_LINE, SUBSTRING_LAST_LINE, SUBSTRING);
    assert_structural_only_contract(mangled, SUBSTRING);
    assert!(
        !ACT_NOW_BUCKETS.contains(&cluster_bucket(mangled)),
        "{SUBSTRING}: an act-now bucket tells a `find-similar` consumer to \
         refuse to write the copy; an unexplained message change has not \
         earned that — {dump}",
        dump = signal_dump(mangled)
    );
    assert!(
        signal(mangled, "rename_consistency") < CONTENT_SUPPORT_FLOOR,
        "{SUBSTRING}: the mangled message must leave the rename \
         uncertified, below the content-support floor — {dump}",
        dump = signal_dump(mangled)
    );

    // Strict separation. `>=` alone is no oracle here: a replacement that
    // substitutes anywhere in the bytes returns 1.0 for *both* fixtures
    // and satisfies every monotonic assertion while the defect is live.
    assert!(
        signal(echo, "rename_consistency") > signal(mangled, "rename_consistency"),
        "a full-symbol echo must measure as strictly more rename evidence \
         than a mid-word byte collision: echo={echo_rename:.4} \
         substring={mangled_rename:.4}\n  echo: {echo_dump}\n  \
         substring: {mangled_dump}",
        echo_rename = signal(echo, "rename_consistency"),
        mangled_rename = signal(mangled, "rename_consistency"),
        echo_dump = signal_dump(echo),
        mangled_dump = signal_dump(mangled),
    );
    assert!(
        signal(echo, "fused") > signal(mangled, "fused"),
        "the rendered confidence must separate the two as well, or the \
         report advises identically about a clean rename and a mangled \
         one: echo={echo_fused:.4} substring={mangled_fused:.4}\n  \
         echo: {echo_dump}\n  substring: {mangled_dump}",
        echo_fused = signal(echo, "fused"),
        mangled_fused = signal(mangled, "fused"),
        echo_dump = signal_dump(echo),
        mangled_dump = signal_dump(mangled),
    );
    Ok(())
}
