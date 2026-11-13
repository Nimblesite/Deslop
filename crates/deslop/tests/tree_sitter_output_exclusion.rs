//! [CONFIG-EXCLUDE-TREE-SITTER] What `tree-sitter generate` writes is not
//! source, and the hand-written files beside it still are.
//!
//! Osprey ships two grammars. Their generated `src/parser.c` (164k lines,
//! no banner) supplied 46,965 of the repository's 76,538 duplicated lines,
//! and the runtime headers the generator copies to `src/tree_sitter/`
//! reported as 100% duplicates of each other (gh #621).
//!
//! Black-box: two grammars with identical generated parsers and headers,
//! each beside an identical hand-written `scanner.c`; and two projects whose
//! hand-written `src/parser.c` and `src/tree_sitter/parser.h` carry the same
//! names with no `grammar.json` to mark them as generated.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::common::*;

const FIXTURE: &str = "c-tree-sitter-grammars";
/// Low enough that every seeded pair would be reported if analysed.
const MIN_NODES: u32 = 12;
const GRAMMARS: [&str; 2] = ["tree-sitter-alpha", "tree-sitter-beta"];
const HAND_WRITTEN_PROJECTS: [&str; 2] = ["calc", "ledger"];
const GENERATED_PARSER: &str = "src/parser.c";
/// The runtime headers the generator copies beside each grammar. The
/// fixture's three differ only in names, so they report as one group.
const RUNTIME_HEADERS: [&str; 3] = [
    "src/tree_sitter/parser.h",
    "src/tree_sitter/array.h",
    "src/tree_sitter/alloc.h",
];
const RENAMED_HEADER_COPIES: usize = RUNTIME_HEADERS.len() * GRAMMARS.len();
/// Generated per grammar: the parser and its runtime headers.
const GENERATED_SOURCES: [&str; 4] = [
    GENERATED_PARSER,
    RUNTIME_HEADERS[0],
    RUNTIME_HEADERS[1],
    RUNTIME_HEADERS[2],
];
const SCANNER: &str = "src/scanner.c";
/// Hand-written per project, under the generator's own file names.
const HAND_WRITTEN_SOURCES: [&str; 2] = ["src/parser.c", "src/tree_sitter/parser.h"];
/// Two scanners, plus a parser and a header in each hand-written project.
const FILES_ANALYSED: u64 = 6;
const PAIR: usize = 2;

/// Every occurrence path in the report, with forward slashes.
fn reported_paths(report: &Value) -> Vec<String> {
    clusters(report)
        .iter()
        .flat_map(occurrence_paths)
        .map(|path| path.replace('\\', "/"))
        .collect()
}

/// Asserts `relative` under both `parents` shares one cluster of `members`.
fn assert_reported(report: &Value, parents: [&str; 2], relative: &str, members: usize) {
    let expected = parents.map(|parent| format!("{parent}/{relative}"));
    let cluster = clusters(report).iter().find(|cluster| {
        let paths: Vec<String> = occurrence_paths(cluster)
            .iter()
            .map(|path| path.replace('\\', "/"))
            .collect();
        expected
            .iter()
            .all(|want| paths.iter().any(|path| path.ends_with(want)))
    });
    let members_found = cluster.map(|cluster| occurrences(cluster).len());
    assert_eq!(
        members_found,
        Some(members),
        "{relative} is copied across {parents:?} and must be one group: {:?}",
        reported_paths(report)
    );
}

/// Generated parsers and headers leave the analysis; the scanners and the
/// hand-written look-alikes stay, each reported as the copy it is.
#[test]
fn generated_grammar_output_is_excluded_and_hand_written_neighbours_are_not() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FILES_ANALYSED)
    );
    let paths = reported_paths(&report);
    for grammar in GRAMMARS {
        for generated in GENERATED_SOURCES {
            let leaked = format!("{grammar}/{generated}");
            assert!(
                !paths.iter().any(|path| path.ends_with(&leaked)),
                "{leaked} is generator output and must not be reported: {paths:?}"
            );
        }
    }
    assert_reported(&report, GRAMMARS, SCANNER, PAIR);
    for hand_written in HAND_WRITTEN_SOURCES {
        assert_reported(&report, HAND_WRITTEN_PROJECTS, hand_written, PAIR);
    }
    Ok(())
}

/// Copies the named fixture files under each parent into `scan_root`.
fn copy_sources(scan_root: &Path, parents: [&str; 2], sources: &[&str]) -> Result<()> {
    for parent in parents {
        for source in sources {
            let target = scan_root.join(parent).join(source);
            let folder = target.parent().context("fixture file has a folder")?;
            std::fs::create_dir_all(folder)?;
            let _bytes = std::fs::copy(fixture(FIXTURE).join(parent).join(source), target)?;
        }
    }
    Ok(())
}

/// Without the `grammar.json` it was generated from, a `src/parser.c` is
/// just a C file: the layout no longer says generator, so nothing is dropped.
#[test]
fn a_parser_without_its_grammar_is_analysed() -> Result<()> {
    let (_tmp, scan_root) = temp_scan_dir("src")?;
    copy_sources(&scan_root, GRAMMARS, &GENERATED_SOURCES)?;
    copy_sources(&scan_root, GRAMMARS, &[SCANNER])?;
    let report = run_report(&scan_root, MIN_NODES)?;
    let per_grammar = GENERATED_SOURCES.len() + [SCANNER].len();
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some((GRAMMARS.len() * per_grammar) as u64)
    );
    for source in [GENERATED_PARSER, SCANNER] {
        assert_reported(&report, GRAMMARS, source, PAIR);
    }
    for header in RUNTIME_HEADERS {
        assert_reported(&report, GRAMMARS, header, RENAMED_HEADER_COPIES);
    }
    Ok(())
}
