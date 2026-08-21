//! [RANK-STRUCTURAL-ONLY-FORWARDING] The forwarding proof must fail open.
//!
//! This predicate deletes findings. Everything it cannot read must
//! therefore keep its cluster visible, and the two ways it could
//! over-reach both get a control here.
//!
//! **Computation dressed as a wrapper.** `dart-forwarding-fail-open`
//! holds two one-statement methods that each contain a call — the same
//! surface shape as the meilisearch REST family. Between the input and
//! the call sits a multiplication and an addition. Any rule counting
//! statements, or counting calls, hides them. The allowlist does not
//! contain arithmetic, so the proof fails and the liftable pair stays on
//! the report.
//!
//! **A copy-paste bug inside a real family.**
//! `dart-forwarding-duplicate-route` holds five wrappers that all prove
//! forwarding. Two of them DELETE the same route, so one of those calls
//! is dead or misaimed. Their declarations differ only in the method
//! name, so the reported windows never compare equal and a window-level
//! distinctness check passes straight over the bug. Comparing the proven
//! *bodies* is what sees it, and one shared body disqualifies the
//! suppression for the whole family.
//!
//! **Business calls wearing the wrapper shape.**
//! `dart-forwarding-business-pair` holds two sibling pairs that each
//! make one allowlisted call and differ only in an integer literal —
//! the exact surface of the meilisearch wrappers. But the calls go to a
//! *sibling helper on the same class*, not to a collaborator the class
//! holds, so parameterising the helper call lifts the pair. A proof
//! that accepts any call, wherever it goes, hides both; forwarding must
//! mean handing the data to collaborator state — a field or parameter
//! receiver. The pairs vary an int rather than a string because
//! same-callee *string*-literal variation is already suppressed by
//! design ([CLONE-NOISE-LITERAL-VARIATION-CALLS]); what is pinned here
//! is the reach the forwarding proof added beyond that filter.
//!
//! **A collaborator call beside a same-class call.** Proving that *a*
//! call in the body delegates says nothing about the others.
//! `dart-forwarding-transform-after-delegation` binds the delegated
//! response and then runs it through a sibling helper; `dart-forwarding-
//! transform-before-delegation` computes with a sibling helper and
//! submits the result. In both the delegating call is byte-identical
//! across the pair, so it carries no duplication whatsoever — every
//! liftable thing lives in the same-class call. A proof that stops at
//! the first delegation reads the statement that is not the duplication
//! and excuses the one that is.

use anyhow::Result;

mod common;
use crate::common::{verdict::*, *};

// What a suppression of each fixture would prove. Every control states
// its own, so a failure names the defect rather than the number.
const COMPUTING_PAIR_WHY: &str =
    "two one-statement Dart methods that multiply and add are liftable \
     logic, not API scaffolding. Hiding them proves the forwarding \
     allowlist leaked arithmetic — or that a statement count stood in for \
     the proof.";

const BUSINESS_PAIR_WHY: &str =
    "both business pairs call a sibling helper — parameterisable logic, \
     not API scaffolding. The bound-result pair is the regression pin: \
     the branch's forwarding proof hid it because it accepted any call \
     without proving a collaborator receiver. The renamed arrow pair is \
     the visibility boundary: content evidence keeps a consistent rename \
     visible as nearly_identical on either side of the fix.";

const AFTER_DELEGATION_WHY: &str =
    "both bodies hand the same byte-identical request to the injected \
     client and then diverge inside `applyMarkup` — the one call that \
     reaches back into the class is the one that differs, and it is \
     liftable by parameterising it. Hiding the pair proves the forwarding \
     proof stopped at the first delegating call.";

const BEFORE_DELEGATION_WHY: &str =
    "the class computes on its own inputs through `normalise` and only \
     then submits the result. No REST wrapper computes before it \
     forwards; hiding this pair proves a trailing delegation excused the \
     computation that preceded it.";

const DUPLICATE_ROUTE_WHY: &str =
    "two of these five wrappers DELETE the same route — one call is dead \
     or misaimed. Hiding the family erases a real finding, and the \
     reported windows cannot show it because the method names differ.";

/// The route two of the five wrappers both DELETE.
const SHARED_ROUTE: &str = "/indexes/dup/settings";

// What each piece of reported evidence proves, so a missing needle fails
// with the reason it mattered rather than with the string.
const MEMBERS_WHY: &str = "each member below is part of the duplication";
const LIFTABLE_HALF_WHY: &str = "each member below is half of a liftable pair";
const TIER_LITERALS_WHY: &str =
    "the differing tier literals are the parameter the lift would take, so \
     both must be reported";
const BASE_LITERALS_WHY: &str =
    "the differing base literals are what parameterising `normalise` would \
     absorb, so both must be reported";

/// Scans `fixture_name` at the subtree-size floor every control here
/// shares, then asserts the report publishes exactly `families` visible
/// clusters and hides none, that each covers `size` occurrences all
/// inside `file`, and that at least `minimum_loc` duplicated lines reach
/// the metrics. `why` states what a suppression would prove, so a
/// failure names the defect rather than the number.
///
/// Returns every reported occurrence text, so each control still pins
/// the evidence that varies across its own pair.
fn expect_visible_families(
    fixture_name: &str,
    file: &str,
    families: usize,
    size: u64,
    minimum_loc: u64,
    why: &str,
) -> Result<Vec<String>> {
    let scan_root = fixture(fixture_name);
    let report = run_report(&scan_root, 12)?;
    let mut texts = Vec::new();
    for cluster in expect_visible_only(&report, families, why) {
        assert_single_file_cluster(cluster, size, file);
        texts.extend(occurrence_texts(&scan_root, cluster)?);
    }
    assert_duplicated_loc_at_least(&report, minimum_loc);
    Ok(texts)
}

/// Asserts every string in `evidence` reached the reported occurrence
/// text. `why` says what the evidence is, so a failure names the missing
/// proof rather than the needle.
fn assert_reported(texts: &[String], evidence: &[&str], why: &str) {
    for needle in evidence {
        assert!(
            texts.iter().any(|text| text.contains(needle)),
            "{why}; {needle} must be reported: {texts:#?}"
        );
    }
}

#[test]
fn one_statement_bodies_that_compute_are_not_forwarding() -> Result<()> {
    let texts = expect_visible_families(
        "dart-forwarding-fail-open",
        "Calc.dart",
        1,
        2,
        4,
        COMPUTING_PAIR_WHY,
    )?;
    assert_reported(&texts, &["scaledDomestic", "scaledExport"], MEMBERS_WHY);
    Ok(())
}

#[test]
fn same_class_helper_calls_are_not_forwarding() -> Result<()> {
    let texts = expect_visible_families(
        "dart-forwarding-business-pair",
        "Pricing.dart",
        2,
        2,
        4,
        BUSINESS_PAIR_WHY,
    )?;
    let members = &[
        "quarterlyFee",
        "annualCharge",
        "standardTotal",
        "premiumTotal",
    ];
    assert_reported(&texts, members, LIFTABLE_HALF_WHY);
    Ok(())
}

#[test]
fn a_same_class_call_after_delegation_is_not_forwarding() -> Result<()> {
    let texts = expect_visible_families(
        "dart-forwarding-transform-after-delegation",
        "Ledger.dart",
        1,
        2,
        4,
        AFTER_DELEGATION_WHY,
    )?;
    let members = &["standardTotal", "premiumTotal", "applyMarkup"];
    assert_reported(&texts, members, MEMBERS_WHY);
    assert_reported(&texts, &["\"standard\"", "\"premium\""], TIER_LITERALS_WHY);
    Ok(())
}

#[test]
fn a_same_class_call_before_delegation_is_not_forwarding() -> Result<()> {
    let texts = expect_visible_families(
        "dart-forwarding-transform-before-delegation",
        "Billing.dart",
        1,
        2,
        2,
        BEFORE_DELEGATION_WHY,
    )?;
    let members = &["quarterlyFee", "annualCharge", "normalise"];
    assert_reported(&texts, members, MEMBERS_WHY);
    assert_reported(&texts, &["100", "250"], BASE_LITERALS_WHY);
    Ok(())
}

#[test]
fn wrappers_sharing_a_body_keep_the_family_visible() -> Result<()> {
    let texts = expect_visible_families(
        "dart-forwarding-duplicate-route",
        "Api.dart",
        1,
        5,
        10,
        DUPLICATE_ROUTE_WHY,
    )?;
    let members = &[
        "resetAlpha",
        "resetBeta",
        "resetGamma",
        "resetDelta",
        "resetEpsilon",
    ];
    assert_reported(&texts, members, MEMBERS_WHY);
    let duplicate_route = texts
        .iter()
        .filter(|text| text.contains(SHARED_ROUTE))
        .count();
    assert_eq!(
        duplicate_route, 2,
        "both same-route wrappers must be among the reported occurrences — \
         they are the reason this family is not noise: {texts:#?}"
    );
    Ok(())
}
