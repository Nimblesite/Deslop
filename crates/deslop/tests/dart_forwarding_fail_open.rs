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
//! `dart-forwarding-business-pair` holds `standardTotal` and
//! `premiumTotal`: one allowlisted call each, differing in one string
//! and one integer literal — the exact surface of the meilisearch
//! wrappers. But the call goes to a *sibling helper on the same class*,
//! not to a collaborator the class holds, so parameterising the helper
//! call lifts the pair. A proof that accepts any call, wherever it
//! goes, hides it; forwarding must mean handing the data to
//! collaborator state — a field or parameter receiver. Nothing in the
//! pair is renamed, so its rename evidence is 0.0 and positional
//! agreement (0.73) is its whole case: a same-file pair pays the same
//! support floor as a cross-file pair ([FUSED-CONTENT-GATE]), and the
//! family question is this proof's, not an admission floor's.
//!
//! Every control below is driven four ways — the JSON and text reports
//! at the shared subtree floor, the same scan at a lower and a higher
//! floor, and a scan under a breached `--fail-over` — and each way pins
//! the family count, every occurrence's line range, the hidden count,
//! the file's metrics and the evidence the occurrences must carry.
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

use std::fs;

use anyhow::Result;
use serde_json::Value;

use crate::common::{verdict::*, *};

/// The subtree-size floor every control shares, and the floors either
/// side of it: a finding must not depend on where the floor sits.
const SUBTREE_FLOOR: u32 = 12;
const LOWER_FLOOR: u32 = 8;
const HIGHER_FLOOR: u32 = 20;

/// Every fixture is one file; nothing in any of them is scaffolding.
const ONE_FILE: u64 = 1;
const NOTHING_HIDDEN: u64 = 0;

/// [EXIT-CODES] `--fail-over 0.0` is breached by any duplication: the
/// CLI exits 3, the report lands on disk, and detection is unchanged.
const BREACHED_BY_ANY_DUPLICATION: &str = "0.0";
const THRESHOLD_BREACH_EXIT_CODE: i32 = 3;
const THRESHOLD_SOURCE_CLI: &str = "cli";
const THRESHOLD_PERCENT_ZERO: f64 = 0.0;

/// Every pair here is two methods; the duplicate-route family is five.
const PAIR: u64 = 2;
const ONE_FAMILY: usize = 1;
const DUPLICATE_ROUTE_FAMILY: u64 = 5;

/// The copy-paste bug in `dart-forwarding-duplicate-route`: two of the
/// five wrappers DELETE this one route.
const DUPLICATE_ROUTE: &str = "/indexes/dup/settings";
const DUPLICATE_ROUTE_WRAPPERS: usize = 2;

// What each piece of reported evidence proves, so a missing needle fails
// with the reason it mattered rather than with the string.
const MEMBERS_WHY: &str = "each member below is part of the duplication";
const LITERALS_WHY: &str = "the differing literals are the parameter the lift would take";

/// One control's whole contract: the fixture, the family it must
/// publish, every occurrence's line range in report order, the metrics
/// the file must carry, and the evidence the occurrence text must show.
struct Scenario {
    fixture: &'static str,
    file: &'static str,
    families: usize,
    size: u64,
    lines: &'static [(u64, u64)],
    duplicated_loc: u64,
    analysed_loc: u64,
    duplication_percent: f64,
    /// Small windows the lower floor admits and the noise bank convicts.
    hidden_at_lower_floor: u64,
    members: &'static [&'static str],
    literals: &'static [&'static str],
    why: &'static str,
}

const COMPUTING_PAIR: Scenario = Scenario {
    fixture: "dart-forwarding-fail-open",
    file: "Calc.dart",
    families: ONE_FAMILY,
    size: PAIR,
    lines: &[(12, 14), (16, 18)],
    duplicated_loc: 6,
    analysed_loc: 21,
    duplication_percent: 28.571_428_571_428_57,
    hidden_at_lower_floor: 0,
    members: &["scaledDomestic", "scaledExport"],
    literals: &["* rate + 7", "* factor + 7"],
    why: "two one-statement Dart methods that multiply and add are liftable \
          logic, not API scaffolding. Hiding them proves the forwarding \
          allowlist leaked arithmetic — or that a statement count stood in \
          for the proof.",
};

const BUSINESS_PAIR: Scenario = Scenario {
    fixture: "dart-forwarding-business-pair",
    file: "Pricing.dart",
    families: ONE_FAMILY,
    size: PAIR,
    lines: &[(35, 38), (40, 43)],
    duplicated_loc: 8,
    analysed_loc: 48,
    duplication_percent: 16.666_666_666_666_664,
    hidden_at_lower_floor: 2,
    members: &[
        "standardTotal",
        "premiumTotal",
        "computePrice",
        "roundMoney",
    ],
    literals: &["\"standard\"", "\"premium\"", "100", "250"],
    why: "the bound-result pair calls a sibling helper — parameterisable \
          logic, not API scaffolding. The forwarding proof once hid it \
          because it accepted any call without proving a collaborator \
          receiver, and the content gate once refused it below a same-file \
          floor a two-literal copy cannot reach; both are false negatives.",
};

const AFTER_DELEGATION: Scenario = Scenario {
    fixture: "dart-forwarding-transform-after-delegation",
    file: "Ledger.dart",
    families: ONE_FAMILY,
    size: PAIR,
    lines: &[(36, 39), (41, 44)],
    duplicated_loc: 8,
    analysed_loc: 47,
    duplication_percent: 17.021_276_595_744_68,
    hidden_at_lower_floor: 1,
    members: &[
        "standardTotal",
        "premiumTotal",
        "applyMarkup",
        "client.fetch",
    ],
    literals: &["\"standard\"", "\"premium\"", "100", "250"],
    why: "both bodies hand the same byte-identical request to the injected \
          client and then diverge inside `applyMarkup` — the one call that \
          reaches back into the class is the one that differs, and it is \
          liftable by parameterising it. Hiding the pair proves the \
          forwarding proof stopped at the first delegating call.",
};

const BEFORE_DELEGATION: Scenario = Scenario {
    fixture: "dart-forwarding-transform-before-delegation",
    file: "Billing.dart",
    families: ONE_FAMILY,
    size: PAIR,
    lines: &[(37, 40), (42, 45)],
    duplicated_loc: 8,
    analysed_loc: 48,
    duplication_percent: 16.666_666_666_666_664,
    hidden_at_lower_floor: 0,
    members: &["quarterlyFee", "annualCharge", "normalise", "client.submit"],
    literals: &["\"standard\"", "\"premium\"", "100", "250"],
    why: "the class computes on its own inputs through `normalise` and only \
          then submits the result. No REST wrapper computes before it \
          forwards; hiding this pair proves a trailing delegation excused \
          the computation that preceded it.",
};

const DUPLICATE_ROUTE_WRAPPER_FAMILY: Scenario = Scenario {
    fixture: "dart-forwarding-duplicate-route",
    file: "Api.dart",
    families: ONE_FAMILY,
    size: DUPLICATE_ROUTE_FAMILY,
    lines: &[(22, 24), (26, 28), (30, 32), (34, 36), (38, 40)],
    duplicated_loc: 15,
    analysed_loc: 45,
    duplication_percent: 33.333_333_333_333_33,
    hidden_at_lower_floor: 0,
    members: &[
        "resetAlpha",
        "resetBeta",
        "resetGamma",
        "resetDelta",
        "resetEpsilon",
    ],
    literals: &[
        "/indexes/alpha/settings",
        "/indexes/beta/settings",
        "/indexes/gamma/settings",
    ],
    why: "two of these five wrappers DELETE the same route — one call is \
          dead or misaimed. Hiding the family erases a real finding, and \
          the reported windows cannot show it because the method names \
          differ.",
};

/// Runs `deslop <fixture> --min-nodes <floor> --embeddings off <extra>`
/// expecting `exit_code`, and returns the JSON report beside the text
/// report the same run rendered.
fn scan(
    scenario: &Scenario,
    floor: u32,
    extra: &[&str],
    exit_code: i32,
) -> Result<(Value, String)> {
    let tmp = tempfile::tempdir()?;
    let prefix = tmp.path().join("report");
    let floor = floor.to_string();
    let _assertion = deslop_cmd(&fixture(scenario.fixture), &prefix)?
        .args(["--min-nodes", floor.as_str(), "--embeddings", "off"])
        .args(extra)
        .assert()
        .code(exit_code);
    let report = load_json(&prefix.with_extension("json"))?;
    let text = fs::read_to_string(prefix.with_extension("txt"))?;
    Ok((report, text))
}

/// Every occurrence's `(start_line, end_line)` across the visible
/// clusters, in report order.
fn reported_lines(report: &Value) -> Vec<(u64, u64)> {
    clusters(report)
        .iter()
        .flat_map(occurrences)
        .map(|occurrence| {
            (
                field(occurrence, "start_line").as_u64().unwrap_or(0),
                field(occurrence, "end_line").as_u64().unwrap_or(0),
            )
        })
        .collect()
}

/// The family, its occurrences and its hidden count, exactly.
fn assert_family(scenario: &Scenario, report: &Value, hidden: u64) {
    let why = scenario.why;
    let visible = clusters(report);
    assert_eq!(visible.len(), scenario.families, "{why} report={report:#}");
    for cluster in visible {
        assert_single_file_cluster(cluster, scenario.size, scenario.file);
    }
    assert_eq!(
        reported_lines(report),
        scenario.lines,
        "{why} every occurrence must be reported at its authored extent: {report:#}"
    );
    assert_eq!(
        clusters_hidden(report),
        hidden,
        "{why} hidden count: {report:#}"
    );
    assert_eq!(
        field(report, "files_analysed").as_u64(),
        Some(ONE_FILE),
        "{why}: {report:#}"
    );
}

/// [METRICS-REPO] The file's figures, as the engine computed them.
fn assert_metrics(scenario: &Scenario, report: &Value) -> Result<()> {
    let why = scenario.why;
    assert_eq!(
        metric_field(report, "analysed_loc").as_u64(),
        Some(scenario.analysed_loc),
        "{why}: {report:#}"
    );
    assert_eq!(
        metric_field(report, "duplicated_loc").as_u64(),
        Some(scenario.duplicated_loc),
        "{why}: {report:#}"
    );
    assert_eq!(
        duplicated_loc_for_path(report, scenario.file)?,
        scenario.duplicated_loc,
        "{why}: {report:#}"
    );
    let percent = metric_field(report, "duplication_percent")
        .as_f64()
        .unwrap_or(0.0);
    assert!(
        approx(percent, scenario.duplication_percent),
        "{why} duplication_percent={percent}, expected {}: {report:#}",
        scenario.duplication_percent
    );
    Ok(())
}

/// The evidence the occurrence text must carry, member by member.
fn assert_evidence(scenario: &Scenario, report: &Value) -> Result<Vec<String>> {
    let scan_root = fixture(scenario.fixture);
    let mut texts = Vec::new();
    for cluster in clusters(report) {
        texts.extend(occurrence_texts(&scan_root, cluster)?);
    }
    assert_reported(&texts, scenario.members, MEMBERS_WHY);
    assert_reported(&texts, scenario.literals, LITERALS_WHY);
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

/// [CLI-TEXT] The text renderer prints the same headline figures and
/// every occurrence as `file:start:end`.
fn assert_text_report(scenario: &Scenario, text: &str) {
    let why = scenario.why;
    let headline = format!("{} cluster(s), {NOTHING_HIDDEN} hidden", scenario.families);
    let figures = format!(
        "({} / {} LOC",
        scenario.duplicated_loc, scenario.analysed_loc
    );
    assert!(
        text.contains(&headline),
        "{why} text report headline must read `{headline}`: {text}"
    );
    assert!(
        text.contains(&figures),
        "{why} text report must carry `{figures}`: {text}"
    );
    for (start, end) in scenario.lines {
        let occurrence = format!("{}:{start}:{end}", scenario.file);
        assert!(
            text.contains(&occurrence),
            "{why} text report must list `{occurrence}`: {text}"
        );
    }
}

/// [EXIT-CODES] A breached fail-over exits 3 with the report on disk,
/// the threshold recorded, and detection unchanged.
fn assert_breached_fail_over(scenario: &Scenario) -> Result<()> {
    let (report, text) = scan(
        scenario,
        SUBTREE_FLOOR,
        &["--fail-over", BREACHED_BY_ANY_DUPLICATION],
        THRESHOLD_BREACH_EXIT_CODE,
    )?;
    let threshold = metric_field(&report, "threshold");
    assert_eq!(
        field(threshold, "breached").as_bool(),
        Some(true),
        "{}: {report:#}",
        scenario.why
    );
    assert_eq!(
        field(threshold, "source").as_str(),
        Some(THRESHOLD_SOURCE_CLI),
        "{report:#}"
    );
    assert_eq!(
        field(threshold, "percent").as_f64(),
        Some(THRESHOLD_PERCENT_ZERO),
        "{report:#}"
    );
    assert_family(scenario, &report, NOTHING_HIDDEN);
    assert_metrics(scenario, &report)?;
    assert_text_report(scenario, &text);
    Ok(())
}

/// Drives one control through every interaction and returns the
/// occurrence text of the shared-floor scan.
fn run_control(scenario: &Scenario) -> Result<Vec<String>> {
    let (report, text) = scan(scenario, SUBTREE_FLOOR, &[], 0)?;
    assert_family(scenario, &report, NOTHING_HIDDEN);
    assert_metrics(scenario, &report)?;
    assert_text_report(scenario, &text);
    let texts = assert_evidence(scenario, &report)?;

    let (higher, _) = scan(scenario, HIGHER_FLOOR, &[], 0)?;
    assert_family(scenario, &higher, NOTHING_HIDDEN);
    assert_metrics(scenario, &higher)?;

    let (lower, _) = scan(scenario, LOWER_FLOOR, &[], 0)?;
    assert_family(scenario, &lower, scenario.hidden_at_lower_floor);
    assert_metrics(scenario, &lower)?;

    assert_breached_fail_over(scenario)?;
    Ok(texts)
}

#[test]
fn one_statement_bodies_that_compute_are_not_forwarding() -> Result<()> {
    // The fail-open direction: `Calc.dart` bodies multiply and add, so
    // the forwarding allowlist cannot prove them and the pair publishes.
    let _texts = run_control(&COMPUTING_PAIR)?;
    Ok(())
}

#[test]
fn same_class_helper_calls_are_not_forwarding() -> Result<()> {
    let _texts = run_control(&BUSINESS_PAIR)?;
    Ok(())
}

#[test]
fn a_same_class_call_after_delegation_is_not_forwarding() -> Result<()> {
    let _texts = run_control(&AFTER_DELEGATION)?;
    Ok(())
}

#[test]
fn a_same_class_call_before_delegation_is_not_forwarding() -> Result<()> {
    let _texts = run_control(&BEFORE_DELEGATION)?;
    Ok(())
}

#[test]
fn wrappers_sharing_a_body_keep_the_family_visible() -> Result<()> {
    let texts = run_control(&DUPLICATE_ROUTE_WRAPPER_FAMILY)?;
    let duplicate_route = texts
        .iter()
        .filter(|text| text.contains(DUPLICATE_ROUTE))
        .count();
    assert_eq!(
        duplicate_route, DUPLICATE_ROUTE_WRAPPERS,
        "both same-route wrappers must be among the reported occurrences — \
         they are the reason this family is not noise: {texts:#?}"
    );
    Ok(())
}
