//! [TEST-SELECTION-SKIP] Both directions of the `#[ignore]` scan.
//!
//! The gate this feeds decides which tests are allowed not to run, so it has
//! to be exact in both directions at once:
//!
//! * a *mention* of `ignore` — in a comment, a doc comment, or a string
//!   literal — is not an attribute, and text matching cannot tell them apart;
//! * an attribute wrapped across lines, preceded by other attributes, or
//!   separated from its function by a doc comment is still that function's
//!   skip, and must be attributed to it by name.
//!
//! Every reason is compared to the value the *compiler* sees, not to the
//! literal as written, because the policy gate matches tokens inside it.

use anyhow::{anyhow, Result};

use super::{ignored_tests_in, IgnoredTest};

/// Stand-in path for the parsed fragments, so failures name a source.
const FILE: &str = "crates/example/tests/example.rs";

/// Scans one fragment, failing the test rather than the assertion when the
/// fragment cannot be parsed at all.
fn scan(source: &str) -> Result<Vec<IgnoredTest>> {
    ignored_tests_in(source, FILE)
}

/// The single skip a fragment declares.
fn only(source: &str) -> Result<IgnoredTest> {
    let found = scan(source)?;
    assert_eq!(found.len(), 1, "expected exactly one skip in:\n{source}");
    found
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no skip found in:\n{source}"))
}

/// The message of the error `source` must produce. Returns an error of its own
/// when the scan succeeded, so "it did not fail" fails the test by the same
/// route as a wrong message rather than by unwrapping.
fn scan_error(source: &str) -> Result<String> {
    match scan(source) {
        Ok(found) => Err(anyhow!("expected an error; the scan returned {found:?}")),
        Err(error) => Ok(error.to_string()),
    }
}

#[test]
fn an_ignored_test_is_reported_with_its_function_name_file_and_reason() -> Result<()> {
    let found = only(
        r#"
#[test]
#[ignore = "GH #422: too large for the runner."]
fn corpus_tokio_rust() {}
"#,
    )?;
    assert_eq!(found.test, "corpus_tokio_rust");
    assert_eq!(found.file, FILE);
    assert_eq!(found.reason, "GH #422: too large for the runner.");
    Ok(())
}

#[test]
fn attribute_order_and_interleaved_doc_comments_do_not_lose_the_owning_function() -> Result<()> {
    let ignore_first = only(
        r#"
#[ignore = "first"]
#[test]
fn skip_declared_before_the_test_attribute() {}
"#,
    )?;
    assert_eq!(ignore_first.test, "skip_declared_before_the_test_attribute");

    let commented = only(
        r#"
#[test]
#[ignore = "second"]
// A note between the attribute and the function it decorates.
/// A doc comment in the same position.
fn skip_separated_by_comments() {}
"#,
    )?;
    assert_eq!(commented.test, "skip_separated_by_comments");
    assert_eq!(commented.reason, "second");
    Ok(())
}

#[test]
fn a_reason_wrapped_across_lines_reads_as_one_sentence_without_its_indentation() -> Result<()> {
    let found = only(
        r#"
#[test]
#[ignore = "[SKIP-UNFINISHED] GH #369 — the fixture loses its second \
            correlated signal, so the refresh has no stable cluster. \
            Run with `-- --ignored`."]
fn wrapped() {}
"#,
    )?;
    assert_eq!(
        found.reason,
        "[SKIP-UNFINISHED] GH #369 — the fixture loses its second correlated signal, so the \
         refresh has no stable cluster. Run with `-- --ignored`."
    );
    assert!(
        !found.reason.contains("  "),
        "continuation indentation leaked into the reason: {:?}",
        found.reason
    );
    Ok(())
}

#[test]
fn quoted_and_newline_escapes_resolve_to_the_characters_they_stand_for() -> Result<()> {
    let found = only(
        r#"
#[test]
#[ignore = "says \"blocked\"\nand then stops"]
fn escaped() {}
"#,
    )?;
    assert_eq!(found.reason, "says \"blocked\"\nand then stops");
    Ok(())
}

#[test]
fn a_bare_ignore_is_reported_with_an_empty_reason_rather_than_passing_unseen() -> Result<()> {
    let found = only(
        r"
#[test]
#[ignore]
fn undocumented_skip() {}
",
    )?;
    assert_eq!(found.test, "undocumented_skip");
    assert_eq!(
        found.reason, "",
        "a bare `#[ignore]` must reach the policy gate carrying nothing, so the gate rejects it"
    );
    Ok(())
}

#[test]
fn the_word_ignore_outside_an_attribute_is_never_a_skip() -> Result<()> {
    let found = scan(
        r##"
//! Module docs that discuss `#[ignore]` and why we do not use it.

/// Doc comment mentioning #[ignore = "not real"].
#[test]
fn mentions_ignore_in_prose() {
    // #[ignore = "commented out"]
    let advice = "#[ignore = \"in a string literal\"]";
    assert!(!advice.is_empty());
}

fn ignore(value: u32) -> u32 {
    value
}
"##,
    )?;
    assert_eq!(
        found,
        Vec::new(),
        "text matching finds five `ignore`s here and the AST finds none"
    );
    Ok(())
}

#[test]
fn every_skip_in_a_file_is_reported_not_only_the_first() -> Result<()> {
    let found = scan(
        r#"
#[test]
#[ignore = "one"]
fn first() {}

#[test]
fn not_skipped() {}

#[test]
#[ignore = "two"]
fn second() {}
"#,
    )?;
    let named: Vec<(&str, &str)> = found
        .iter()
        .map(|skip| (skip.test.as_str(), skip.reason.as_str()))
        .collect();
    assert_eq!(named, vec![("first", "one"), ("second", "two")]);
    Ok(())
}

#[test]
fn a_conditional_ignore_is_an_error_rather_than_a_skip_the_gate_cannot_see() -> Result<()> {
    let message = scan_error(
        r#"
#[test]
#[cfg_attr(target_os = "windows", ignore)]
fn conditionally_skipped() {}
"#,
    )?;
    assert!(
        message.contains(FILE) && message.contains("cfg_attr"),
        "`#[cfg_attr(.., ignore)]` must be rejected by name: {message}"
    );
    Ok(())
}

#[test]
fn an_unconditional_cfg_attr_that_never_mentions_ignore_is_left_alone() -> Result<()> {
    let found = scan(
        r"
#[cfg_attr(test, derive(Debug))]
struct Reported;

#[test]
fn runs() {}
",
    )?;
    assert_eq!(found, Vec::new());
    Ok(())
}

#[test]
fn an_ignore_that_decorates_something_other_than_a_function_is_an_error() -> Result<()> {
    let message = scan_error(
        r#"
#[ignore = "on a module"]
mod grouped {}
"#,
    )?;
    assert!(
        message.contains("mod_item"),
        "an `#[ignore]` on a non-function must name what it decorated rather than being \
         attributed to a later test: {message}"
    );
    Ok(())
}
