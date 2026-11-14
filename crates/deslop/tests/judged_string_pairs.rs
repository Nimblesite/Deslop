//! [FUSED-CONTENT-GATE-LITERAL-BOUND] [CORPUS-REGISTER] Real duplicates
//! whose text is mostly strings stay reported, in every language and every
//! way the same files can reach a scan.
//!
//! Every pair here is one the corpus register judges CLEARLY IN, excerpted
//! verbatim from the repository it was judged in (see
//! `fixtures/judged-string-pairs/register.json`, `judged_at`):
//!
//! * C#, `BenchmarkDotNet`: analyzer and code-fix tests holding the program
//!   under test in a raw string, neighbours differing by one call or name.
//! * Rust, serde: `NonZero*` deserialisation tests whose bodies are error
//!   strings, and `Serialize*` traits whose bodies are doc comments.
//! * Python, `FastAPI` and Flask: constructors and methods carried by long
//!   `Doc("…")` texts and docstrings.
//! * TypeScript, Vite: transform snapshots in template literals, HMR log
//!   expectations, minified-code fixtures.
//! * JavaScript, Express and date-fns: HTTP header and body expectations,
//!   playground call runs.
//! * PHP, Symfony Console and Monolog: assertion messages and formatted
//!   block expectations.
//! * Dart, http: `Set-Cookie` header parsing and charset expectations.
//! * Go, Viper: a YAML document in a raw string beside its decoded map.
//!
//! The negatives are the judged unrelated strings of gh #626 (a usage text
//! and an embedded test program) and gh #581 (two different SQL queries).

use std::fs;

use anyhow::Result;
use serde_json::{json, Value};

use deslop_test_support::corpus_score::{CLEARLY_IN, CLEARLY_OUT};

use crate::common::{
    clusters, field, fixture,
    judged_pairs::{
        all_repos, answering_spans, assert_every_judgment_holds, copy_tree, files_under, register,
        register_for, scan_and_score, seed_repos, with_list, JUDGED_FIXTURE, LANGUAGES,
    },
    temp_scan_dir,
};

/// Every judged pair in the fixture register.
const JUDGED_PAIRS: usize = 20;
/// Source files in the fixture tree, licences and register excluded.
const SOURCE_FILES: u64 = 21;
/// Directory names of the judged-unrelated negatives inside a mixed scan.
const OSPREY_DIR: &str = "osprey";
const DATAPROVIDER_DIR: &str = "dataprovider";
const OSPREY_FIXTURE: &str = "rust-escaped-string-literals";
const DATAPROVIDER_FIXTURE: &str = "csharp-sql-command-setup";
/// Prefix that reverses the repositories' discovery order.
const REORDER_PREFIX: &str = "z";
/// The gh #626 and gh #581 judgments below.
const JUDGED_UNRELATED: usize = 4;
const CRLF: &str = "\r\n";
const LF: &str = "\n";

/// The gh #626 and gh #581 judgments: unrelated strings are not copies.
fn judged_unrelated_strings() -> Value {
    let out =
        |left: &str, right: &str, why: &str| json!({ "why": why, "occurrences": [left, right] });
    json!([
        out(
            "osprey/usage.rs:1-18",
            "osprey/run_output.rs:6-8",
            "A CLI usage text and an embedded test program share escapes, not text (gh #626).",
        ),
        out(
            "osprey/usage.rs:1-18",
            "osprey/navigation.rs:1-16",
            "A CLI usage text and navigation strings share escapes, not text (gh #626).",
        ),
        out(
            "osprey/run_output.rs:6-8",
            "osprey/navigation.rs:1-16",
            "An embedded test program and navigation strings share escapes, not text (gh #626).",
        ),
        out(
            "dataprovider/PostgresSchemaInspector.cs:10-32",
            "dataprovider/PostgresSchemaInspector.cs:39-64",
            "A column query and an index query behind the same setup are different SQL (gh #581).",
        ),
    ])
}

/// Prefix every licence file in the fixture tree carries.
const LICENCE_PREFIX: &str = "LICENSE";

/// Whether a fixture file is source code, not a licence.
fn is_source(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| !name.starts_with(LICENCE_PREFIX))
}

/// All 20 judged pairs, scanned together as one tree, each answered by a
/// cluster that shows most of its judged lines.
#[test]
fn every_judged_string_pair_is_reported_in_one_scan() -> Result<()> {
    let register = register()?;
    let (_tmp, root) = temp_scan_dir(JUDGED_FIXTURE)?;
    seed_repos(&root, &all_repos())?;
    let sources = files_under(&root)?
        .into_iter()
        .filter(|path| is_source(path))
        .count();
    assert_eq!(u64::try_from(sources)?, SOURCE_FILES);

    let (report, score) = scan_and_score(&root, &register)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(SOURCE_FILES)
    );
    assert_eq!(score.clearly_in_total, JUDGED_PAIRS);
    assert_eq!(score.clearly_in_found, JUDGED_PAIRS);
    assert_every_judgment_holds(&score, "the whole tree");
    assert_eq!(answering_spans(&report, &score)?.len(), JUDGED_PAIRS);
    Ok(())
}

/// Each language's pairs alone, and each repository alone: no judgment
/// may depend on unrelated code elsewhere in the scan.
#[test]
fn each_language_and_each_repository_keeps_its_judged_pairs() -> Result<()> {
    let register = register()?;
    let mut judged = 0;
    for (language, repos) in LANGUAGES {
        let (_tmp, root) = temp_scan_dir(language)?;
        seed_repos(&root, repos)?;
        let narrowed = register_for(&register, repos);
        let (_report, score) = scan_and_score(&root, &narrowed)?;
        assert_every_judgment_holds(&score, language);
        judged += score.judged;
        for repo in repos.iter().filter(|_| repos.len() > 1) {
            let (_tmp, alone) = temp_scan_dir(repo)?;
            seed_repos(&alone, &[repo])?;
            let (_report, score) = scan_and_score(&alone, &register_for(&register, &[repo]))?;
            assert_every_judgment_holds(&score, &format!("{language}: {repo} alone"));
        }
    }
    assert_eq!(
        judged, JUDGED_PAIRS,
        "every judged pair belongs to one language"
    );
    Ok(())
}

/// The judged copies and the judged unrelated strings in one scan: the
/// copies are reported and none of the unrelated strings is paired.
#[test]
fn judged_copies_and_judged_unrelated_strings_scanned_together() -> Result<()> {
    let register = with_list(&register()?, CLEARLY_OUT, judged_unrelated_strings());
    let (_tmp, root) = temp_scan_dir(JUDGED_FIXTURE)?;
    seed_repos(&root, &all_repos())?;
    copy_tree(&fixture(OSPREY_FIXTURE), &root.join(OSPREY_DIR))?;
    copy_tree(&fixture(DATAPROVIDER_FIXTURE), &root.join(DATAPROVIDER_DIR))?;

    let (_report, score) = scan_and_score(&root, &register)?;
    assert_eq!(score.clearly_in_found, JUDGED_PAIRS);
    assert_eq!(score.clearly_out_total, JUDGED_UNRELATED);
    assert_eq!(score.clearly_out_absent, JUDGED_UNRELATED);
    assert_every_judgment_holds(&score, "copies beside unrelated strings");

    // Each negative alone, so no copy elsewhere is what keeps it apart.
    for (dir, source) in [
        (OSPREY_DIR, OSPREY_FIXTURE),
        (DATAPROVIDER_DIR, DATAPROVIDER_FIXTURE),
    ] {
        let (_tmp, alone) = temp_scan_dir(dir)?;
        copy_tree(&fixture(source), &alone.join(dir))?;
        let only = with_list(&register_for(&register, &[dir]), CLEARLY_IN, json!([]));
        let (report, score) = scan_and_score(&alone, &only)?;
        assert_every_judgment_holds(&score, dir);
        assert!(
            clusters(&report).is_empty(),
            "{dir}: unrelated strings form no cluster at all: {:#}",
            field(&report, "clusters")
        );
    }
    Ok(())
}

/// Windows line endings change no judgment and no reported extent.
#[test]
fn judged_string_pairs_survive_crlf_line_endings() -> Result<()> {
    let register = register()?;
    let (_tmp, lf_root) = temp_scan_dir(JUDGED_FIXTURE)?;
    seed_repos(&lf_root, &all_repos())?;
    let (lf_report, lf_score) = scan_and_score(&lf_root, &register)?;
    assert_every_judgment_holds(&lf_score, "LF");

    let (_tmp, crlf_root) = temp_scan_dir(JUDGED_FIXTURE)?;
    seed_repos(&crlf_root, &all_repos())?;
    for path in files_under(&crlf_root)?
        .into_iter()
        .filter(|path| is_source(path))
    {
        let text = fs::read_to_string(&path)?;
        assert!(
            !text.contains(CRLF),
            "{} is LF in the fixture",
            path.display()
        );
        fs::write(&path, text.replace(LF, CRLF))?;
    }
    let (crlf_report, crlf_score) = scan_and_score(&crlf_root, &register)?;
    assert_every_judgment_holds(&crlf_score, "CRLF");
    assert_eq!(
        answering_spans(&crlf_report, &crlf_score)?,
        answering_spans(&lf_report, &lf_score)?,
        "line endings move no reported extent"
    );
    Ok(())
}

/// Reversing the repositories' discovery order changes no judgment and no
/// reported extent: a pair is never found only because of where it sorts.
#[test]
fn judged_string_pairs_hold_in_reversed_discovery_order() -> Result<()> {
    let register = register()?;
    let (_tmp, root) = temp_scan_dir(JUDGED_FIXTURE)?;
    seed_repos(&root, &all_repos())?;
    let (report, score) = scan_and_score(&root, &register)?;

    let (_tmp, reordered) = temp_scan_dir(JUDGED_FIXTURE)?;
    let repos = all_repos();
    let renamed = |index: usize, repo: &str| {
        format!(
            "{}{repo}",
            REORDER_PREFIX.repeat(repos.len().saturating_sub(index))
        )
    };
    for (index, repo) in repos.iter().enumerate() {
        copy_tree(
            &fixture(JUDGED_FIXTURE).join(repo),
            &reordered.join(renamed(index, repo)),
        )?;
    }
    let rename_range = |range: &Value| -> Value {
        let text = range.as_str().unwrap_or_default();
        repos
            .iter()
            .enumerate()
            .find(|(_, repo)| text.starts_with(&format!("{repo}/")))
            .map_or_else(
                || range.clone(),
                |(index, repo)| Value::String(text.replacen(repo, &renamed(index, repo), 1)),
            )
    };
    let moved_entries: Vec<Value> = field(&register, CLEARLY_IN)
        .as_array()
        .into_iter()
        .flatten()
        .map(|entry| {
            let ranges: Vec<Value> = field(entry, "occurrences")
                .as_array()
                .into_iter()
                .flatten()
                .map(rename_range)
                .collect();
            with_list(entry, "occurrences", Value::Array(ranges))
        })
        .collect();
    let moved = with_list(&register, CLEARLY_IN, Value::Array(moved_entries));
    let (moved_report, moved_score) = scan_and_score(&reordered, &moved)?;
    assert_every_judgment_holds(&moved_score, "reversed discovery order");
    assert_eq!(
        answering_spans(&moved_report, &moved_score)?,
        answering_spans(&report, &score)?,
        "discovery order moves no reported extent"
    );
    Ok(())
}
