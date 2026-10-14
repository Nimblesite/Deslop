//! [EXIT-CODES-KINDS] Human verdicts agree with a kind-only CLI failure.

use super::*;

#[test]
fn kind_only_failure_is_visible_in_text_and_colors_the_html_banner() -> Result<()> {
    let (_tmp, output, mut command) = kind_limit_run(&[&format!(
        "{KIND_LIMIT_IDENTICAL}{IDENTICAL_KIND_BEACHING_LIMIT}"
    )])?;
    let _assertion = command
        .args([MIN_NODES_FLAG, TOTAL_INCLUDE_MIN_NODES, NO_COLOR_FLAG])
        .assert()
        .code(THRESHOLD_BREACH_EXIT_CODE);
    let json = read_json_report(&output.json)?;
    assert_kind_threshold(&json, "identical", CONFIG_SOURCE, true);
    assert_threshold(&json, NO_SOURCE, false);
    let text = fs::read_to_string(&output.txt)?;
    assert_contains(
        &text,
        "Identical code threshold:",
        "a kind-only gate is visible in text",
    );
    let html = fs::read_to_string(&output.html)?;
    assert_contains(
        &html,
        "metrics-banner metrics-banner--breached",
        "HTML cannot claim a passing run when a kind gate failed",
    );
    assert_contains(
        &html,
        "Identical code threshold",
        "the HTML verdict names the kind that failed",
    );
    Ok(())
}

#[test]
fn kind_only_failure_colors_html_when_the_total_gate_passes() -> Result<()> {
    let (_tmp, output, mut command) = kind_limit_run(&[&format!(
        "{KIND_LIMIT_IDENTICAL}{IDENTICAL_KIND_BEACHING_LIMIT}"
    )])?;
    let _assertion = command
        .args([
            MIN_NODES_FLAG,
            TOTAL_INCLUDE_MIN_NODES,
            NO_COLOR_FLAG,
            FAIL_OVER_FLAG,
            PERMISSIVE_THRESHOLD,
        ])
        .assert()
        .code(THRESHOLD_BREACH_EXIT_CODE);
    let json = read_json_report(&output.json)?;
    assert_kind_threshold(&json, "identical", CONFIG_SOURCE, true);
    assert_threshold(&json, CLI_SOURCE, false);
    assert_contains(
        &fs::read_to_string(&output.html)?,
        "metrics-banner metrics-banner--breached",
        "a passing total cannot hide a failed kind gate",
    );
    Ok(())
}
