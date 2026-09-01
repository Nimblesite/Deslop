//! [OUTPUT-DIR] Unit coverage for the reported-path wire form.
//!
//! gh #439 is a Windows-only defect: on a platform whose separator is
//! already `/` the renderer looked correct, so every assertion written
//! against the running platform passed while Windows reports carried a
//! form no consumer could match. These cases supply the platform
//! separator explicitly, so the contract is pinned on every platform
//! rather than only the one the suite happens to run on.

use std::path::Path;

use super::{rewrite_separators, wire_path, WIRE_PATH_SEPARATOR};

/// The separator Windows joins with, and the one no reported path may carry.
const WINDOWS_SEPARATOR: char = '\\';

/// A curated manifest path, in the form every consumer names a file.
const WIRE_FORM: &str = "tokio/src/io/stdout.rs";

/// The same path as a Windows report rendered it before gh #439.
const NATIVE_FORM: &str = "tokio\\src\\io\\stdout.rs";

/// The rewrite a Windows report needs: every separator becomes the wire
/// separator, so a rendered path equals the curated manifest path it is
/// compared against.
#[test]
fn a_windows_path_is_rewritten_into_wire_form() {
    assert_eq!(
        rewrite_separators(NATIVE_FORM, WINDOWS_SEPARATOR),
        WIRE_FORM,
        "a report path joined with `{WINDOWS_SEPARATOR}` must render as `{WIRE_FORM}`, or the \
         corpus manifests, the VSIX links and the MCP `path_contains` filter all miss it on the \
         separator alone"
    );
}

/// A path already in wire form is returned unchanged, so the rewrite is
/// idempotent and a POSIX report is byte-identical before and after.
#[test]
fn a_wire_form_path_is_left_alone() {
    assert_eq!(
        rewrite_separators(WIRE_FORM, WIRE_PATH_SEPARATOR),
        WIRE_FORM,
        "rewriting a path that is already wire form must change nothing"
    );
}

/// Losslessness. A backslash is a legal character in a POSIX file name,
/// so on a platform that separates with `/` it is data and must survive.
/// Rewriting it there would rename the user's file in every report.
#[test]
fn a_posix_file_name_containing_a_backslash_survives() {
    let posix_name = "src/weird\\name.rs";
    assert_eq!(
        rewrite_separators(posix_name, WIRE_PATH_SEPARATOR),
        posix_name,
        "`{WINDOWS_SEPARATOR}` is a legal POSIX file-name character; rewriting it would name a \
         file that does not exist"
    );
}

/// The public entry point agrees with the platform it runs on: whatever
/// this platform separates with, the result carries only the wire form.
#[test]
fn the_public_entry_point_renders_this_platform_in_wire_form() {
    let rendered = wire_path(&Path::new("core").join("handlers").join("alpha.rs"));
    let text = rendered.to_string_lossy().into_owned();
    assert_eq!(
        text, "core/handlers/alpha.rs",
        "a nested path must render as `core/handlers/alpha.rs` on every platform; got `{text}`"
    );
}
