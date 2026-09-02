//! [PAIR-COMPARE-CLI] Endpoint parsing, both directions.

use anyhow::Result;

use super::{parse_comparison, parse_endpoint};

/// A wire-form path with its byte range.
const WELL_FORMED: &str = "tokio/src/io/stdout.rs:10:420";

/// The same shape a Windows caller pastes, drive letter and all.
const WINDOWS_ABSOLUTE: &str = "C:/repo/src/lib.rs:0:96";

#[test]
fn a_well_formed_endpoint_parses_into_its_three_fields() -> Result<()> {
    let endpoint = parse_endpoint(WELL_FORMED)?;
    assert_eq!(endpoint.path.to_string_lossy(), "tokio/src/io/stdout.rs");
    assert_eq!(endpoint.start_byte, 10);
    assert_eq!(endpoint.end_byte, 420);
    Ok(())
}

#[test]
fn a_colon_in_the_path_survives_because_offsets_are_read_from_the_right() -> Result<()> {
    let endpoint = parse_endpoint(WINDOWS_ABSOLUTE)?;
    assert_eq!(
        endpoint.path.to_string_lossy(),
        "C:/repo/src/lib.rs",
        "splitting from the left would eat the drive letter and name a file that does not exist"
    );
    assert_eq!(endpoint.end_byte, 96);
    Ok(())
}

#[test]
fn an_empty_range_is_refused_rather_than_measured() {
    let error = parse_endpoint("src/lib.rs:40:40")
        .err()
        .map_or_else(String::new, |failure| format!("{failure}"));
    assert!(
        error.contains("covers no bytes"),
        "an endpoint covering nothing must be refused, not compared: {error}"
    );
}

#[test]
fn a_non_numeric_offset_names_the_field_it_could_not_read() {
    let error = parse_endpoint("src/lib.rs:start:40")
        .err()
        .map_or_else(String::new, |failure| format!("{failure}"));
    assert!(
        error.contains("start byte"),
        "the error must name which offset failed: {error}"
    );
}

#[test]
fn a_missing_offset_is_refused() {
    let error = parse_endpoint("src/lib.rs:40")
        .err()
        .map_or_else(String::new, |failure| format!("{failure}"));
    assert!(
        error.contains("<start_byte>"),
        "the error must state the shape expected: {error}"
    );
}

#[test]
fn a_comparison_needs_exactly_two_endpoints() {
    let one = [WELL_FORMED.to_owned()];
    let error = parse_comparison(&one)
        .err()
        .map_or_else(String::new, |failure| format!("{failure}"));
    assert!(
        error.contains("exactly 2 endpoints"),
        "one endpoint is not a pair, and a cluster id is not valid input: {error}"
    );
}

#[test]
fn two_endpoints_parse_into_a_left_and_a_right() -> Result<()> {
    let both = [WELL_FORMED.to_owned(), WINDOWS_ABSOLUTE.to_owned()];
    let (left, right) = parse_comparison(&both)?;
    assert_eq!(left.start_byte, 10);
    assert_eq!(right.end_byte, 96);
    Ok(())
}
