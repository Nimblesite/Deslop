//! A stand-in for a shipped Deslop binary. [DEPLOY-VSIX-PACKAGE]
//!
//! The verifier proofs need executables that report a *chosen* name and
//! version, so that a verifier's rejection has something real to reject. A
//! `#!/bin/sh` script cannot be one of those on Windows: there is no shebang,
//! and `CreateProcess` will not start a text file — every proof that staged
//! one got a null exit status and empty output, and then compared that empty
//! output against an expectation and passed.
//!
//! So the fixture is a real program. It is compiled once per test run and
//! copied for each fixture, with that fixture's two answers appended after the
//! marker below — which is why it reads its own image rather than taking the
//! answers from a file beside it: a second file staged next to a binary would
//! show up inside the package fixtures as an undeclared entry, and the
//! verifiers are right to reject those.
//!
//! Built by `scripts/lib/fake-binary.mjs`; it is not part of the workspace.

use std::error::Error;
use std::fs;

/// Separates the compiled image from the answers appended to a copy of it.
const MARKER: &[u8] = b"\n@@DESLOP-FAKE-BINARY-PAYLOAD@@\n";

/// Prints the JSON answer for `--version --json` and the plain one otherwise,
/// matching the two shapes `[DEPLOY-BINARY-VERSION]` requires of a real binary.
fn main() -> Result<(), Box<dyn Error>> {
    let image = fs::read(std::env::current_exe()?)?;
    let start = last_offset(&image, MARKER).ok_or("this copy carries no appended answers")?;
    let answers = String::from_utf8(image[start + MARKER.len()..].to_vec())?;
    let mut lines = answers.lines();
    let plain = lines.next().ok_or("the appended answers have no plain version line")?;
    let json = lines.next().ok_or("the appended answers have no JSON version line")?;
    let wants_json = std::env::args().any(|argument| argument == "--json");
    println!("{}", if wants_json { json } else { plain });
    Ok(())
}

/// Offset of the LAST occurrence of `needle`. The marker is also a string
/// constant inside the compiled image, so the first occurrence is this
/// program's own code and the appended copy is always the final one.
fn last_offset(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).rposition(|window| window == needle)
}
