//! [TEST-SELECTION] Unit coverage for the feature-liveness scan.
//!
//! Every case is a source string parsed through the same tree-sitter
//! path the workspace scan uses, so the extraction is exercised rather
//! than the file walk.

use anyhow::Result;

use super::feature_liveness_pins_in;

/// The file name every case is attributed to.
const FILE: &str = "crates/example/tests/example.rs";
/// The feature the fixtures pin.
const PROFILING: &str = "profiling";
/// A second feature, for the multi-pin case.
const LIVE: &str = "live";

/// One pin, as the contract test reads it.
fn pin(feature: &str) -> (String, String) {
    (FILE.to_owned(), feature.to_owned())
}

#[test]
fn an_unconditional_test_asserting_a_feature_is_a_pin() -> Result<()> {
    let source = r#"
#[test]
fn the_matrix_enables_profiling() {
    assert!(cfg!(feature = "profiling"), "off");
}
"#;
    assert_eq!(
        feature_liveness_pins_in(source, FILE)?,
        vec![pin(PROFILING)],
        "an unconditional test asserting `cfg!(feature = ..)` is exactly \
         what proves the required command enables that feature"
    );
    Ok(())
}

#[test]
fn a_cfg_gated_test_is_never_a_pin() -> Result<()> {
    let source = r#"
#[cfg(feature = "profiling")]
#[test]
fn only_compiled_when_the_feature_is_on() {
    assert!(cfg!(feature = "profiling"), "off");
}
"#;
    assert_eq!(
        feature_liveness_pins_in(source, FILE)?,
        Vec::new(),
        "a pin compiled only under the configuration it checks can never \
         fail, so it proves nothing and must not be counted"
    );
    Ok(())
}

#[test]
fn a_plain_function_is_never_a_pin() -> Result<()> {
    let source = r#"
fn helper() {
    assert!(cfg!(feature = "profiling"), "off");
}
"#;
    assert_eq!(
        feature_liveness_pins_in(source, FILE)?,
        Vec::new(),
        "only a `#[test]` runs in the gate; a helper nobody calls asserts \
         nothing"
    );
    Ok(())
}

#[test]
fn doc_comments_between_the_attribute_and_the_function_do_not_hide_the_test() -> Result<()> {
    let source = r#"
#[test]
/// A comment sits between the attribute and the function.
fn the_matrix_enables_profiling() {
    assert!(cfg!(feature = "profiling"), "off");
}
"#;
    assert_eq!(
        feature_liveness_pins_in(source, FILE)?,
        vec![pin(PROFILING)],
        "tree-sitter makes attributes siblings of the item, so a comment \
         between them must not end the attribute run"
    );
    Ok(())
}

#[test]
fn every_feature_in_one_test_is_pinned_and_non_feature_predicates_are_not() -> Result<()> {
    let source = r#"
#[test]
fn the_matrix_enables_both() {
    assert!(cfg!(feature = "profiling"), "off");
    assert!(cfg!(unix), "not unix");
    assert!(cfg!(target_os = "linux") || cfg!(feature = "live"), "off");
}
"#;
    assert_eq!(
        feature_liveness_pins_in(source, FILE)?,
        vec![pin(PROFILING), pin(LIVE)],
        "each `feature = \"..\"` operand is its own pin, and `unix` / \
         `target_os` name no cargo feature so neither may be reported as one"
    );
    Ok(())
}
