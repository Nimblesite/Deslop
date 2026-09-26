//! [PAIR-COMPARE-CLI] Endpoint parsing, both directions.

use anyhow::Result;

use super::{parse_comparison, parse_endpoint, ENDPOINTS_PER_COMPARISON};

/// A wire-form path with its byte range.
const WELL_FORMED: &str = "tokio/src/io/stdout.rs:10:420";
const WELL_FORMED_PATH: &str = "tokio/src/io/stdout.rs";
const WELL_FORMED_START: usize = 10;
const WELL_FORMED_END: usize = 420;

/// The same shape a Windows caller pastes, drive letter and all.
const WINDOWS_ABSOLUTE: &str = "C:/repo/src/lib.rs:0:96";
const WINDOWS_ABSOLUTE_PATH: &str = "C:/repo/src/lib.rs";
const WINDOWS_ABSOLUTE_END: usize = 96;

/// Renders the error a parse produced, or nothing when it succeeded.
fn error_text<T>(outcome: Result<T>) -> String {
    outcome
        .err()
        .map_or_else(String::new, |failure| format!("{failure}"))
}

#[test]
fn a_well_formed_endpoint_parses_into_its_three_fields() -> Result<()> {
    let endpoint = parse_endpoint(WELL_FORMED)?;
    assert_eq!(endpoint.path.to_string_lossy(), WELL_FORMED_PATH);
    assert_eq!(endpoint.start_byte, WELL_FORMED_START);
    assert_eq!(endpoint.end_byte, WELL_FORMED_END);
    Ok(())
}

#[test]
fn a_colon_in_the_path_survives_because_offsets_are_read_from_the_right() -> Result<()> {
    let endpoint = parse_endpoint(WINDOWS_ABSOLUTE)?;
    assert_eq!(
        endpoint.path.to_string_lossy(),
        WINDOWS_ABSOLUTE_PATH,
        "splitting from the left would eat the drive letter and name a file that does not exist"
    );
    assert_eq!(endpoint.end_byte, WINDOWS_ABSOLUTE_END);
    Ok(())
}

#[test]
fn an_empty_range_is_refused_rather_than_measured() {
    let error = error_text(parse_endpoint("src/lib.rs:40:40"));
    assert!(
        error.contains("covers no bytes"),
        "an endpoint covering nothing must be refused, not compared: {error}"
    );
}

#[test]
fn an_inverted_range_is_refused_rather_than_measured() {
    let error = error_text(parse_endpoint("src/lib.rs:41:40"));
    assert!(
        error.contains("covers no bytes"),
        "an endpoint whose end precedes its start must be refused, not compared: {error}"
    );
}

#[test]
fn a_non_numeric_offset_names_the_field_it_could_not_read() {
    let error = error_text(parse_endpoint("src/lib.rs:start:40"));
    assert!(
        error.contains("start byte"),
        "the error must name which offset failed: {error}"
    );
    let error = error_text(parse_endpoint("src/lib.rs:40:end"));
    assert!(
        error.contains("end byte"),
        "the error must name which offset failed: {error}"
    );
}

#[test]
fn a_missing_offset_is_refused() {
    let error = error_text(parse_endpoint("src/lib.rs:40"));
    assert!(
        error.contains("<start_byte>"),
        "the error must state the shape expected: {error}"
    );
}

#[test]
fn an_empty_path_is_refused() {
    let error = error_text(parse_endpoint(":10:40"));
    assert!(
        error.contains("<path>"),
        "an endpoint naming no file must be refused: {error}"
    );
}

#[test]
fn a_comparison_needs_exactly_two_endpoints() {
    let one = [WELL_FORMED.to_owned()];
    let error = error_text(parse_comparison(&one));
    assert!(
        error.contains(&format!("exactly {ENDPOINTS_PER_COMPARISON} endpoints")),
        "one endpoint is not a pair, and a cluster id is not valid input: {error}"
    );
    let three = [
        WELL_FORMED.to_owned(),
        WINDOWS_ABSOLUTE.to_owned(),
        WELL_FORMED.to_owned(),
    ];
    let error = error_text(parse_comparison(&three));
    assert!(
        error.contains("got 3"),
        "three endpoints are not a pair either, and the count must be named: {error}"
    );
}

#[test]
fn two_endpoints_parse_into_a_left_and_a_right() -> Result<()> {
    let both = [WELL_FORMED.to_owned(), WINDOWS_ABSOLUTE.to_owned()];
    let (left, right) = parse_comparison(&both)?;
    assert_eq!(left.start_byte, WELL_FORMED_START);
    assert_eq!(left.path.to_string_lossy(), WELL_FORMED_PATH);
    assert_eq!(right.end_byte, WINDOWS_ABSOLUTE_END);
    assert_eq!(right.path.to_string_lossy(), WINDOWS_ABSOLUTE_PATH);
    Ok(())
}
