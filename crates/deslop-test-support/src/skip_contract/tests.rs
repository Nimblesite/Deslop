//! [TEST-SELECTION-SKIP] Every way a stated skip can be wrong.
//!
//! These cases were found by mutation: the whole-tree gate was run against a
//! deliberately broken tree, once per breach, and each mutation was confirmed
//! to turn it red on the assertion that owns it. Proving that once proves
//! nothing durable — the tree was already compliant, so weakening `breaches`
//! would leave the whole-tree gate green. Each mutation is a case here so the
//! check that caught it cannot be removed without a red test.
//!
//! The compliant reason is the control. It is judged first in every case, so a
//! permutation that fails for an unrelated reason cannot be mistaken for the
//! breach it is meant to demonstrate.

use super::{
    bracketed_ids, breaches, issue_mentions, plan_paths, registry_diff, Breach, PolicyContext,
};
use crate::skip_policy::IgnoredTest;

/// The issue the curated registry says owns the skip under test.
const ISSUE: u32 = 422;

/// The plan the compliant reason names, and a body that discusses `ISSUE`.
const PLAN: &str = "docs/plans/corpus-assertion.md";
const PLAN_BODY: &str = "Tracked by #422, blocked on #166. How the skip ends: stream the scan.";

/// A spec id `docs/specs` declares, and one it does not.
const DECLARED_ID: &str = "CORPUS-PIN";
const UNDECLARED_ID: &str = "NOT-A-REAL-SPEC-ID";

/// A reason that satisfies every clause of the policy.
const COMPLIANT: &str = "[SKIP-TOO-LARGE-FOR-CI] GH #422 [CORPUS-PIN] \
     docs/plans/corpus-assertion.md — clones tokio-rs/tokio at its pinned commit and scans the \
     whole Rust tree. `make test-corpus` runs it, single-threaded, via `-- --ignored`.";

/// The context the compliant reason is judged against.
fn context() -> PolicyContext {
    PolicyContext::new(&[DECLARED_ID, "CORPUS-RECALL"], &[(PLAN, PLAN_BODY)])
}

/// Judges `reason` as though it were the skip on `corpus_tokio_rust`.
fn judge(reason: &str) -> Vec<Breach> {
    let skip = IgnoredTest {
        file: "crates/deslop/tests/corpus_repos.rs".to_owned(),
        test: "corpus_tokio_rust".to_owned(),
        reason: reason.to_owned(),
    };
    breaches(&skip, ISSUE, &context())
}

/// The compliant reason with one part removed or corrupted — the mutation.
fn mutated(from: &str, to: &str) -> Vec<Breach> {
    assert!(
        COMPLIANT.contains(from),
        "the mutation anchor {from:?} is no longer in the compliant reason, so this case is \
         testing nothing"
    );
    judge(&COMPLIANT.replace(from, to))
}

#[test]
fn the_compliant_reason_is_accepted_so_every_other_case_isolates_one_breach() {
    assert_eq!(
        judge(COMPLIANT),
        Vec::new(),
        "the control reason must satisfy the policy outright"
    );
}

#[test]
fn a_bare_ignore_breaches_and_reports_nothing_else() {
    assert_eq!(
        judge(""),
        vec![Breach::NoReason],
        "a reasonless skip is one breach, not nine: reporting the rest would bury the only \
         thing the author has to fix"
    );
}

#[test]
fn a_reason_claiming_no_category_or_two_categories_breaches() {
    assert_eq!(
        mutated("[SKIP-TOO-LARGE-FOR-CI] ", ""),
        vec![Breach::Categories(Vec::new())],
        "a skip that claims no category has not said why it is allowed"
    );
    assert_eq!(
        mutated(
            "[SKIP-TOO-LARGE-FOR-CI]",
            "[SKIP-TOO-LARGE-FOR-CI] [SKIP-UNFINISHED]"
        ),
        vec![Breach::Categories(vec![
            "[SKIP-UNFINISHED]".to_owned(),
            "[SKIP-TOO-LARGE-FOR-CI]".to_owned(),
        ])],
        "two categories is two different stories about the same skip"
    );
}

#[test]
fn an_excuse_that_is_not_an_allowed_category_breaches() {
    assert_eq!(
        mutated("[SKIP-TOO-LARGE-FOR-CI]", "[SKIP-BREAKING-CI]"),
        vec![Breach::Categories(Vec::new())],
        "\"it was breaking CI\" must not be expressible: inventing a tag has to fail, or the \
         two allowed categories are a suggestion"
    );
}

#[test]
fn citing_the_wrong_issue_breaches_even_though_an_issue_is_cited() {
    assert_eq!(
        mutated("GH #422", "GH #999"),
        vec![Breach::IssueNotCited(ISSUE)],
        "the issue in the reason must be the issue the registry says owns the skip, or the \
         citation points somewhere nobody is tracking"
    );
    assert_eq!(
        mutated("GH #422", "issue 422"),
        vec![Breach::IssueNotCited(ISSUE)],
        "the strict `GH #<n>` form is what makes a citation findable; prose naming the number \
         some other way does not count"
    );
}

#[test]
fn naming_no_plan_or_a_plan_that_is_not_in_the_tree_breaches() {
    assert_eq!(
        mutated("docs/plans/corpus-assertion.md — ", ""),
        vec![Breach::NoPlan],
        "a skip without a plan is a feature abandoned in place"
    );
    let moved = "docs/plans/deleted-plan.md";
    assert_eq!(
        mutated(PLAN, moved),
        vec![Breach::PlanMissing(moved.to_owned())],
        "citing a plan that was renamed or deleted must fail, or the citation decays silently"
    );
}

#[test]
fn a_plan_that_never_mentions_the_issue_breaches() {
    let skip = IgnoredTest {
        file: "crates/deslop/tests/corpus_repos.rs".to_owned(),
        test: "corpus_tokio_rust".to_owned(),
        reason: COMPLIANT.to_owned(),
    };
    let silent = PolicyContext::new(
        &[DECLARED_ID],
        &[(PLAN, "A plan about something else entirely.")],
    );
    assert_eq!(
        breaches(&skip, ISSUE, &silent),
        vec![Breach::PlanSilentOnIssue(ISSUE)],
        "pointing at a plan that never discusses the issue is a citation with nothing behind \
         it — the plan has to say how the skip ends"
    );
}

#[test]
fn citing_no_spec_id_or_an_undeclared_one_breaches() {
    assert_eq!(
        mutated("[CORPUS-PIN] ", ""),
        vec![Breach::NoSpecId],
        "without a spec id nothing connects the skipped behaviour to the specification"
    );
    assert_eq!(
        mutated(DECLARED_ID, UNDECLARED_ID),
        vec![Breach::UndeclaredSpecId(UNDECLARED_ID.to_owned())],
        "a spec id no specification declares is a cross-reference to nowhere"
    );
}

#[test]
fn the_category_tag_alone_does_not_satisfy_the_spec_id_clause() {
    assert_eq!(
        mutated("[CORPUS-PIN] ", ""),
        vec![Breach::NoSpecId],
        "`[SKIP-TOO-LARGE-FOR-CI]` is bracketed and upper-case like a spec id. It must not \
         count as one, or every skip satisfies the clause by restating its own category"
    );
}

#[test]
fn dropping_the_run_instruction_breaches() {
    assert_eq!(
        mutated("via `-- --ignored`.", "and that is that."),
        vec![Breach::NoRunInstruction],
        "the assertions have to stay reachable to whoever picks the issue up"
    );
}

#[test]
fn several_breaches_are_reported_together_rather_than_one_at_a_time() {
    let stripped = judge("[SKIP-UNFINISHED] nothing else at all");
    assert_eq!(
        stripped,
        vec![
            Breach::IssueNotCited(ISSUE),
            Breach::NoPlan,
            Breach::NoSpecId,
            Breach::NoRunInstruction,
        ],
        "one run must report every missing part, so fixing a skip is one edit and not four \
         rounds of the gate"
    );
    for breach in stripped {
        assert!(
            breach.explain().len() > 30,
            "{breach:?} must explain itself: a verdict the author cannot act on is a verdict \
             they will work around"
        );
    }
}

#[test]
fn an_unregistered_skip_and_a_stale_registry_entry_are_reported_separately() {
    let registered = ("a.rs".to_owned(), "registered".to_owned());
    let fresh = ("a.rs".to_owned(), "unregistered".to_owned());
    let (unregistered, stale) = registry_diff(
        &[registered.clone(), fresh.clone()],
        &[registered.clone(), ("b.rs".to_owned(), "gone".to_owned())],
    );
    assert_eq!(
        unregistered,
        vec![fresh],
        "a skip nobody registered is a test that stopped running without anyone deciding it \
         could"
    );
    assert_eq!(
        stale,
        vec![("b.rs".to_owned(), "gone".to_owned())],
        "a registry entry whose `#[ignore]` is gone must fail too: a skip that outlives its \
         defect reads as coverage nobody has"
    );
    assert_eq!(
        registry_diff(&[registered.clone()], &[registered]),
        (Vec::new(), Vec::new()),
        "a registry that matches the tree reports neither direction"
    );
}

#[test]
fn the_reason_scanners_read_tokens_and_not_whatever_is_nearby() {
    assert_eq!(
        bracketed_ids("[CORPUS-PIN] [lowercase] [With Space] [] [PIPELINE-DETERMINISM]"),
        vec!["CORPUS-PIN", "PIPELINE-DETERMINISM"],
        "only the upper-case hyphenated shape is a spec id; prose in brackets is not"
    );
    assert_eq!(
        issue_mentions("GH #422 and #166, but ## heading and #not-a-number"),
        vec![422, 166],
        "issue numbers are digits after a hash, and a markdown heading is neither"
    );
    assert_eq!(
        plan_paths("see docs/plans/a.md and docs/plans/b.md, not docs/specs/c.md"),
        vec!["docs/plans/a.md", "docs/plans/b.md"],
        "a plan path is under docs/plans; a spec path is not a plan"
    );
}
