//! [CORPUS-REPORT-ENTRY] A judged pair is written to the JSON scorecard as the
//! pair and pass or fail, and a failure as the one thing that failed — never
//! the scorer's working.

use anyhow::Result;
use serde_json::{json, Value};

use super::{
    first_entry, pinned_sha, register, report, score_repo, the_pair, CLEARLY_IN, CLEARLY_OUT,
    CLUSTER_ID, FIRST_RANGE, REPO, SECOND_RANGE,
};

/// Outcomes as the record writes them.
const PASS: &str = "pass";
const FAIL: &str = "fail";
/// The field a failure names what failed in.
const FAILED: &str = "failed";
/// What failed, for each way a judged pair can fail.
const NOT_REPORTED: &str = "not reported";
const REPORTED_IN_CLUSTER: &str = "reported in cluster abc123";
const MISTYPED: &str = "reported as nearly_identical, judged Type I";
/// The kind a mistyped pair is reported under, and the type it was held to.
const NEARLY_IDENTICAL: &str = "nearly_identical";
const TYPE_ONE: &str = "Type I";

/// The first entry `judged` scores against `published`, as the JSON writes it.
fn written(judged: &Value, published: &Value) -> Result<Value> {
    let score = score_repo(REPO, judged, published)?;
    Ok(serde_json::to_value(first_entry(&score)?)?)
}

/// The pair, and how it went.
fn expected(verdict: &str, outcome: &str) -> Value {
    json!({ "verdict": verdict, "occurrences": [FIRST_RANGE, SECOND_RANGE], "outcome": outcome })
}

/// A failed pair, and what failed.
fn failed(verdict: &str, what: &str) -> Value {
    let mut entry = expected(verdict, FAIL);
    if let Some(fields) = entry.as_object_mut() {
        let _absent = fields.insert(FAILED.to_owned(), json!(what));
    }
    entry
}

#[test]
fn a_judged_pair_is_written_as_pass_or_fail_and_a_failure_names_what_failed() -> Result<()> {
    let ranges = [FIRST_RANGE, SECOND_RANGE];
    let paired = report(&the_pair());
    let silent = report(&[]);
    let pinned = json!({ "sha": pinned_sha(), CLEARLY_IN: [ {
        "why": "judged", "type": TYPE_ONE, "occurrences": ranges,
    } ] });
    let renamed = json!({ "clusters": [
        { "id": CLUSTER_ID, "kind": NEARLY_IDENTICAL, "occurrences": the_pair() },
    ] });

    let cases = [
        (
            register(CLEARLY_IN, &ranges),
            &paired,
            expected(CLEARLY_IN, PASS),
        ),
        (
            register(CLEARLY_OUT, &ranges),
            &silent,
            expected(CLEARLY_OUT, PASS),
        ),
        (
            register(CLEARLY_IN, &ranges),
            &silent,
            failed(CLEARLY_IN, NOT_REPORTED),
        ),
        (
            register(CLEARLY_OUT, &ranges),
            &paired,
            failed(CLEARLY_OUT, REPORTED_IN_CLUSTER),
        ),
        (pinned, &renamed, failed(CLEARLY_IN, MISTYPED)),
    ];
    for (judged, published, want) in cases {
        assert_eq!(written(&judged, published)?, want, "{judged}");
    }
    Ok(())
}
