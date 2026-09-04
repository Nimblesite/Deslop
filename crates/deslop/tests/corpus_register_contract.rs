//! [CORPUS-REGISTER] Contract over the clone registers in `corpus/register/`.
//!
//! A register is independent ground truth: pairs of code regions a judge
//! classified CLEARLY IN or CLEARLY OUT while isolated from this codebase, so
//! the verdict cannot be the engine's own opinion of itself read back. The
//! judging protocol is `.agents/skills/judge-clone-pairs`; the spec section is
//! `docs/specs/corpus.md` [CORPUS-REGISTER].
//!
//! An entry that names one range, or carries no reason, or was never diffed
//! against the pinned source asserts nothing while looking like an assertion —
//! exactly the failure mode `corpus_manifest_contract` was written for on the
//! recall lists. These tests make that state fail loudly.
//!
//! [TEST-SELECTION-SKIP] They read JSON only, need no clone on disk, and carry
//! no `#[ignore]`: they run in `make test`, which is the pipeline a hollowed-out
//! register would otherwise pass unnoticed.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use deslop_test_support::{corpus::repo_root, read_json};
use serde_json::Value;

/// Where the judged registers live, one per target repository.
const REGISTER_DIR: &str = "corpus/register";
/// The gate config shares that directory but is not a register. Naming it here
/// keeps the directory scan from reading configuration as ground truth.
const THRESHOLDS_STEM: &str = "score-thresholds";
/// The two judged verdicts, and the recorded-but-asserting-nothing third.
const CLEARLY_IN: &str = "clearly_in";
const CLEARLY_OUT: &str = "clearly_out";
const NOT_CLEAR: &str = "not_clear";
/// Prose that must stand in for an empty `clearly_out` list, so nobody reads
/// emptiness as evidence that precision is good.
const CLEARLY_OUT_STATUS: &str = "clearly_out_status";
/// A pairing claim needs two regions; one range describes no pair at all.
const MINIMUM_RANGES: usize = 2;
/// The registers judged so far. A register that vanishes must fail here.
const EXPECTED_REGISTERS: usize = 4;
/// Fields every judged entry carries.
const WHY: &str = "why";
const VERIFIED: &str = "verified";
const OCCURRENCES: &str = "occurrences";
/// The commit the judge read, without which a range names nothing stable.
const SHA: &str = "sha";
/// A judgement stated in fewer characters than this is not a judgement.
const MINIMUM_PROSE: usize = 40;
/// The protocol every register cites, as paths this test resolves on disk. A
/// register that points at a moved or deleted protocol documents nothing.
const PROTOCOL: &str = "protocol";
const PROTOCOL_PATH_FIELDS: [&str; 3] = ["spec", "judging_skill", "preparer_skill"];
const SPEC_FIELD: &str = "spec";
const SPEC_SECTION_FIELD: &str = "spec_section";
const JUDGING_SKILL_FIELD: &str = "judging_skill";
/// [CORPUS-SCORE] The gate every scored repository is held to.
const THRESHOLDS: &str = "corpus/register/score-thresholds.json";
const DEFAULTS: &str = "defaults";
const REPOS: &str = "repos";
/// A ratio threshold, refused by the gate contract below: see
/// [`every_gate_allowance_is_an_absolute_count_so_a_growing_register_cannot_loosen_it`].
const MINIMUM_SCORE: &str = "minimum_score_percent";
const MAXIMUM_FALSE_NEGATIVES: &str = "maximum_false_negatives";
const MAXIMUM_FALSE_POSITIVES: &str = "maximum_false_positives";
/// The strict default: a judged repository answers every judged pair
/// correctly, so it is allowed zero false negatives and zero false positives.
/// Loosening this instead of recording a tracked exception would silently drop
/// the gate on every repository at once.
const PERFECT_SCORE: u64 = 0;

/// The judging protocol is only independent ground truth while the judge cannot
/// learn what produced the reports, so the protocol itself must never name this
/// project. See [CORPUS-REGISTER].
const PROJECT_NAME: &str = "deslop";

/// Every register, with its file stem.
fn registers() -> Result<Vec<(String, Value)>> {
    let directory = repo_root().join(REGISTER_DIR);
    let mut found = Vec::new();
    for entry in fs::read_dir(&directory).context("corpus/register must be readable")? {
        let path = entry?.path();
        let name = stem(&path);
        if path.extension().is_some_and(|ext| ext == "json") && name != THRESHOLDS_STEM {
            found.push((name, read_json(&path)?));
        }
    }
    found.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(found)
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_owned()
}

/// The entries of one verdict list, empty when the key is absent.
fn entries<'a>(register: &'a Value, verdict: &str) -> &'a [Value] {
    register
        .get(verdict)
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
}

/// A string field, blank when absent — a missing field and an empty one are
/// the same failure.
fn text(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// Splits `path:start-end`, returning the two line numbers.
fn line_bounds(range: &str) -> Option<(u32, u32)> {
    let (_, span) = range.rsplit_once(':')?;
    let (start, end) = span.split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?))
}

#[test]
fn every_register_pins_a_commit_and_at_least_one_judged_pair() -> Result<()> {
    let found = registers()?;
    assert!(
        found.len() >= EXPECTED_REGISTERS,
        "the register set lost files: only {:?} remain",
        found.iter().map(|(name, _)| name).collect::<Vec<_>>()
    );
    for (name, register) in &found {
        assert_eq!(
            text(register, SHA).len(),
            40,
            "{name}: a register must pin the full commit its ranges were read at, \
             or every line number in it names something unknown"
        );
        assert!(
            !entries(register, CLEARLY_IN).is_empty(),
            "{name}: no CLEARLY IN entries — a register that judges nothing asserts nothing"
        );
    }
    Ok(())
}

#[test]
fn every_judged_entry_names_a_pair_it_actually_read() -> Result<()> {
    for (name, register) in registers()? {
        for verdict in [CLEARLY_IN, CLEARLY_OUT] {
            for (index, entry) in entries(&register, verdict).iter().enumerate() {
                let label = format!("{name} {verdict}[{index}]");
                assert!(
                    text(entry, WHY).len() >= MINIMUM_PROSE,
                    "{label}: `why` must state the judgement in plain English"
                );
                assert!(
                    text(entry, VERIFIED).len() >= MINIMUM_PROSE,
                    "{label}: `verified` must record the diff that was run and what it returned"
                );
                let ranges = entries(entry, OCCURRENCES);
                assert!(
                    ranges.len() >= MINIMUM_RANGES,
                    "{label}: names {} range(s); a pairing claim needs at least {MINIMUM_RANGES}",
                    ranges.len()
                );
            }
        }
    }
    Ok(())
}

/// Holds one range to the `path:start-end` shape, and to naming real lines.
fn assert_range(label: &str, range: &Value) {
    let text = range.as_str().unwrap_or_default();
    let bounds = line_bounds(text);
    assert!(
        bounds.is_some(),
        "{label}: range is not `path:start-end`: {text}"
    );
    let (start, end) = bounds.unwrap_or_default();
    assert!(
        start >= 1 && end >= start,
        "{label}: range is empty or inverted: {text}"
    );
}

#[test]
fn every_range_is_a_well_formed_non_empty_span() -> Result<()> {
    for (name, register) in registers()? {
        for verdict in [CLEARLY_IN, CLEARLY_OUT, NOT_CLEAR] {
            for entry in entries(&register, verdict) {
                for range in entries(entry, OCCURRENCES) {
                    assert_range(&format!("{name} {verdict}"), range);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn an_empty_clearly_out_list_states_why_it_is_empty() -> Result<()> {
    for (name, register) in registers()? {
        if entries(&register, CLEARLY_OUT).is_empty() {
            assert!(
                text(&register, CLEARLY_OUT_STATUS).len() >= MINIMUM_PROSE,
                "{name}: `clearly_out` is empty and `{CLEARLY_OUT_STATUS}` does not say so — \
                 an empty list must never read as evidence that precision is good"
            );
        }
    }
    Ok(())
}

#[test]
fn every_register_cites_a_protocol_that_exists_on_disk() -> Result<()> {
    let root = repo_root();
    for (name, register) in registers()? {
        let protocol = register.get(PROTOCOL).cloned().unwrap_or(Value::Null);
        for field in PROTOCOL_PATH_FIELDS {
            let cited = text(&protocol, field);
            assert!(
                !cited.is_empty(),
                "{name}: `{PROTOCOL}.{field}` is missing — a register must say which \
                 protocol produced it, or nobody can re-judge it the same way"
            );
            assert!(
                root.join(&cited).exists(),
                "{name}: `{PROTOCOL}.{field}` points at `{cited}`, which does not exist"
            );
        }
        let spec = fs::read_to_string(root.join(text(&protocol, SPEC_FIELD)))?;
        let section = text(&protocol, SPEC_SECTION_FIELD);
        assert!(
            spec.contains(&section),
            "{name}: the spec no longer carries section `{section}`"
        );
    }
    Ok(())
}

#[test]
fn the_judging_protocol_never_names_this_project() -> Result<()> {
    let root = repo_root();
    for (name, register) in registers()? {
        let cited = text(
            &register.get(PROTOCOL).cloned().unwrap_or(Value::Null),
            JUDGING_SKILL_FIELD,
        );
        let protocol = fs::read_to_string(root.join(&cited))?.to_lowercase();
        assert!(
            !protocol.contains(PROJECT_NAME),
            "{name}: `{cited}` names this project. A judge who learns what produced the \
             reports is reading the engine's opinion of itself back as ground truth, and \
             every verdict from that pass is void"
        );
    }
    Ok(())
}

/// A count field, or zero when absent.
fn count(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or_default()
}

#[test]
fn the_score_gate_is_strict_by_default_and_every_exception_is_a_tracked_repository() -> Result<()> {
    let root = repo_root();
    let config: Value = read_json(&root.join(THRESHOLDS))?;
    let defaults = config.get(DEFAULTS).cloned().unwrap_or(Value::Null);
    for allowance in [MAXIMUM_FALSE_NEGATIVES, MAXIMUM_FALSE_POSITIVES] {
        assert_eq!(
            defaults.get(allowance).and_then(Value::as_u64),
            Some(PERFECT_SCORE),
            "the default gate must demand a perfect score, and `{allowance}` of zero is what a \
             perfect score is in units that do not re-scale with the register — an exception \
             belongs under `{REPOS}`, with the reason, not in the default that covers every \
             repository at once"
        );
    }

    let repos = config.get(REPOS).and_then(Value::as_object);
    for (name, entry) in repos.into_iter().flatten() {
        let register = root
            .join(REGISTER_DIR)
            .join(format!("{}.json", name.to_lowercase()));
        assert!(
            register.exists(),
            "`{REPOS}.{name}` gates a repository with no register at {}",
            register.display()
        );
        assert!(
            text(entry, WHY).len() >= MINIMUM_PROSE,
            "`{REPOS}.{name}` relaxes the gate without saying which defect it is tracking — \
             an entry here is an admission that a bug shipped, and must read like one"
        );
    }
    Ok(())
}

#[test]
fn no_gated_allowance_exceeds_what_its_register_actually_judges() -> Result<()> {
    let root = repo_root();
    let config: Value = read_json(&root.join(THRESHOLDS))?;
    let repos = config.get(REPOS).and_then(Value::as_object);
    for (name, entry) in repos.into_iter().flatten() {
        let register: Value = read_json(
            &root
                .join(REGISTER_DIR)
                .join(format!("{}.json", name.to_lowercase())),
        )?;
        for (allowance, verdict) in [
            (MAXIMUM_FALSE_NEGATIVES, CLEARLY_IN),
            (MAXIMUM_FALSE_POSITIVES, CLEARLY_OUT),
        ] {
            let judged = entries(&register, verdict).len() as u64;
            assert!(
                count(entry, allowance) <= judged,
                "`{REPOS}.{name}.{allowance}` allows more defects than the register has \
                 {verdict} entries ({judged}) — a gate nothing can breach is not a gate"
            );
        }
    }
    Ok(())
}

#[test]
fn every_gate_allowance_is_an_absolute_count_so_a_growing_register_cannot_loosen_it() -> Result<()>
{
    let root = repo_root();
    let config: Value = read_json(&root.join(THRESHOLDS))?;
    let sections = [
        (DEFAULTS, config.get(DEFAULTS).cloned().unwrap_or(Value::Null)),
        (REPOS, config.get(REPOS).cloned().unwrap_or(Value::Null)),
    ];
    for (section, value) in sections {
        let entries: Vec<(String, Value)> = value.as_object().map_or_else(
            || vec![(section.to_owned(), value.clone())],
            |fields| {
                if section == REPOS {
                    fields
                        .iter()
                        .map(|(name, entry)| (name.clone(), entry.clone()))
                        .collect()
                } else {
                    vec![(section.to_owned(), value.clone())]
                }
            },
        );
        for (name, entry) in entries {
            assert!(
                entry.get(MINIMUM_SCORE).is_none(),
                "`{section}.{name}` gates on `{MINIMUM_SCORE}`, a ratio against the register. \
                 A score is `100 * (judged - false negatives - false positives) / judged`, so a \
                 percentage threshold is a defect allowance divided by the register size: the \
                 same number permits more defects every time the register grows, loosening the \
                 gate with nobody editing it. State the allowance in \
                 `{MAXIMUM_FALSE_NEGATIVES}` and `{MAXIMUM_FALSE_POSITIVES}`, which mean the \
                 same thing at every register size."
            );
        }
    }
    Ok(())
}
