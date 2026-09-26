//! [CONFIG-EXCLUDE-BUILTIN] A build artifact is not source, and Deslop must
//! not read one as if it were.
//!
//! A bundled or minified file is compiler output: everything on one line,
//! whitespace stripped, identifiers shortened. Nobody edits it and nobody can
//! act on a duplicate found inside it. Left in the analysis it costs far more
//! than its size suggests — `gohugoio/hugo` keeps three such files in ordinary
//! source folders, and they account for 84% of the memory a scan of that
//! repository uses (6,160 MB against 987 MB without them) while putting a
//! build artifact at rank 10 of the report (gh #539).
//!
//! Path rules cannot catch these on their own: hugo's sit outside
//! `node_modules/` and `vendor/`, and a bundler is free to name its output
//! `app-3f2a.js`. The shape of the file is what gives it away, so the guard
//! that has to hold is the one keyed on line length.
//!
//! Black-box: run the CLI over a fixture whose two first-party files carry one
//! genuine clone, beside one artifact of the shape a bundler emits.

use anyhow::Result;
use serde_json::Value;

use crate::common::*;

/// The fixture: two hand-written files that genuinely duplicate each other,
/// and one build artifact.
const FIXTURE: &str = "javascript-minified-bundle";
/// Low enough that the first-party pair is reported at all.
const MIN_NODES: u32 = 8;
/// The files a human wrote. `ledger.js` is large — 53 KB, larger than either
/// artifact — and entirely readable, so it pins the opposite failure: a guard
/// that excluded on size alone would drop real source and turn this into a
/// false negative.
const FIRST_PARTY: [&str; 3] = ["orders.js", "invoices.js", "ledger.js"];
/// The readable file that must survive the guard, and the longest line in it.
const LARGE_READABLE: &str = "ledger.js";
const READABLE_LONGEST_LINE: usize = 69;

/// The two build artifacts, one per rule, so neither rule can be broken while
/// the other covers for it.
///
/// `vendor.min.js` is named as an artifact and is deliberately *below* the
/// size floor the shape test applies, so only the name can catch it.
/// `app-3f2a9c.js` carries no suffix any list knows — the name a content-hash
/// bundler emits — so only its shape can. A single fixture carrying both
/// signals would pass with either rule dead.
const ARTIFACT_BY_NAME: &str = "vendor.min.js";
const ARTIFACT_BY_SHAPE: &str = "app-3f2a9c.js";
const ARTIFACTS: [&str; 2] = [ARTIFACT_BY_NAME, ARTIFACT_BY_SHAPE];
/// What the shape-caught artifact looks like: one line, seventeen thousand
/// characters of it. Nothing hand-written approaches this.
const SHAPE_ARTIFACT_LONGEST_LINE: usize = 17_101;
/// The size below which the shape test does not read a file. `vendor.min.js`
/// sits under it on purpose.
const SHAPE_SIZE_FLOOR: usize = 16_384;

/// Every occurrence path in the report, separator-normalised so the assertion
/// holds on a Windows runner too.
fn occurrence_paths(report: &Value) -> Vec<String> {
    clusters(report)
        .iter()
        .flat_map(|cluster| {
            cluster
                .get("occurrences")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|occurrence| {
            occurrence
                .get("file")
                .or_else(|| occurrence.get("path"))
                .and_then(Value::as_str)
                .map(|path| path.replace('\\', "/"))
        })
        .collect()
}

#[test]
fn a_large_readable_file_is_still_analysed() -> Result<()> {
    let scan_root = fixture(FIXTURE);
    let readable = std::fs::read_to_string(scan_root.join(LARGE_READABLE))?;
    for artifact in ARTIFACTS {
        let bytes = std::fs::read_to_string(scan_root.join(artifact))?.len();
        assert!(
            readable.len() > bytes,
            "the readable file must be larger than {artifact}, or this asserts nothing \
             about size being the wrong signal: {} bytes against {bytes}",
            readable.len(),
        );
    }
    assert_eq!(
        readable.lines().map(str::len).max(),
        Some(READABLE_LONGEST_LINE),
        "every line in {LARGE_READABLE} must stay human-sized",
    );

    let report = run_report(&scan_root, MIN_NODES)?;
    let analysed = report
        .get("metrics")
        .and_then(|metrics| metrics.get("per_file"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let named: Vec<String> = analysed
        .iter()
        .filter_map(|file| file.get("path").and_then(Value::as_str))
        .map(|path| path.replace('\\', "/"))
        .collect();
    assert!(
        named.iter().any(|path| path.ends_with(LARGE_READABLE)),
        "{LARGE_READABLE} is 53 KB of ordinary source and must be analysed. Excluding on \
         size would be a false negative — the whole file would vanish from the report \
         with nothing to say it had. Analysed: {named:?}",
    );
    Ok(())
}

#[test]
fn a_minified_bundle_is_not_analysed_as_source() -> Result<()> {
    let scan_root = fixture(FIXTURE);
    let by_shape = std::fs::read_to_string(scan_root.join(ARTIFACT_BY_SHAPE))?;
    assert_eq!(
        by_shape.lines().map(str::len).max(),
        Some(SHAPE_ARTIFACT_LONGEST_LINE),
        "{ARTIFACT_BY_SHAPE} stopped being a minified artifact, so this test no longer \
         asserts anything about one",
    );
    let by_name = std::fs::read_to_string(scan_root.join(ARTIFACT_BY_NAME))?;
    assert!(
        by_name.len() < SHAPE_SIZE_FLOOR,
        "{ARTIFACT_BY_NAME} must stay under the {SHAPE_SIZE_FLOOR}-byte floor the shape \
         test applies, or the name rule is no longer the only thing that can catch it and \
         this test stops proving the name rule works. Measured {} bytes",
        by_name.len(),
    );

    let report = run_report(&scan_root, MIN_NODES)?;

    assert_eq!(
        report.get("files_analysed").and_then(Value::as_u64),
        Some(FIRST_PARTY.len() as u64),
        "only the {} hand-written files may be analysed; {ARTIFACTS:?} are build output, \
         and parsing one line of {SHAPE_ARTIFACT_LONGEST_LINE} characters is where the \
         memory goes. report={report}",
        FIRST_PARTY.len(),
    );

    let reported = clusters(&report);
    assert!(
        !reported.is_empty(),
        "the first-party pair is a genuine clone and must still be reported — an empty \
         report would satisfy the artifact guard below without proving anything. \
         report={report}",
    );

    let paths = occurrence_paths(&report);
    for artifact in ARTIFACTS {
        assert!(
            !paths.iter().any(|path| path.ends_with(artifact)),
            "no occurrence may sit inside {artifact}: it is build output, so a duplicate \
             found in it is one nobody can act on. Occurrences reported: {paths:?}",
        );
    }
    for expected in ["orders.js", "invoices.js"] {
        assert!(
            paths.iter().any(|path| path.ends_with(expected)),
            "the clone between orders.js and invoices.js must still be reported; {expected} \
             is absent from {paths:?}",
        );
    }
    Ok(())
}
