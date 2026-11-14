//! [FUSED-CONTENT-GATE-LITERAL-BOUND] [FUSED-PAIR-SIGNALS] Explicit
//! comparison admits every judged string-heavy copy, in either order.
//!
//! The pairs are the CLEARLY IN entries of
//! `fixtures/judged-string-pairs/register.json` (see `judged_string_pairs.rs`
//! for where each was judged). Discovery may answer a pair through a wider
//! cluster; explicit comparison names the two occurrences that answer the
//! judged sides exactly, so the literal-bound rule meets each pair head
//! on: an embedded program, a docstring or a YAML document that is most
//! of its endpoint, beside a near-copy of itself. Refusing one of them is a false negative the
//! comparison names.
//!
//! The negatives are gh #626's unrelated escaped strings and gh #581's two
//! SQL queries, refused by name in either order.

use std::path::Path;

use anyhow::{Context, Result};
use deslop_core::report::PairEndpoint;
use deslop_test_support::corpus_score::{Range, ScoredEntry};
use serde_json::Value;

use crate::common::{
    clusters, field, fixture,
    judged_pairs::{all_repos, register, scan_and_score, seed_repos, JUDGED_FIXTURE, MIN_NODES},
    occurrence_line_span, occurrences,
    signals::{compare_endpoints, compare_pair},
    temp_scan_dir,
};

/// Every judged pair in the fixture register.
const JUDGED_PAIRS: usize = 20;
const LITERAL_BOUND_REFUSAL: &str =
    "rejected: one string literal carries an endpoint and the texts differ";
const OSPREY_FIXTURE: &str = "rust-escaped-string-literals";
const DATAPROVIDER_FIXTURE: &str = "csharp-sql-command-setup";
/// gh #626: the usage constant's type and value, and the `let osp = "…"`
/// statement, as byte spans (the extents `rust_escaped_string_literals.rs`
/// compares).
const USAGE_TEXT: (&str, usize, usize) = ("usage.rs", 24, 1545);
const EMBEDDED_PROGRAM: (&str, usize, usize) = ("run_output.rs", 270, 566);
/// gh #581: each query from its assignment to its second parameter.
const COLUMN_QUERY_LINES: (&str, u32, u32) = ("PostgresSchemaInspector.cs", 10, 32);
const INDEX_QUERY_LINES: (&str, u32, u32) = ("PostgresSchemaInspector.cs", 39, 64);

/// The byte endpoint of a judged line range: the first non-blank byte of
/// its first line to the last non-blank byte of its last line.
fn endpoint(root: &Path, path: &str, start: u32, end: u32) -> Result<PairEndpoint> {
    let file = root.join(path);
    let text = std::fs::read_to_string(&file)?;
    let line_start = |line: u32| -> Result<usize> {
        let skip = usize::try_from(line.saturating_sub(1))?;
        Ok(text.split_inclusive('\n').take(skip).map(str::len).sum())
    };
    let from = line_start(start)?;
    let to = line_start(end.saturating_add(1))?;
    let slice = text
        .get(from..to)
        .context("judged lines lie inside the file")?;
    let leading = slice.len().saturating_sub(slice.trim_start().len());
    let trailing = slice.len().saturating_sub(slice.trim_end().len());
    Ok(PairEndpoint {
        path: file,
        start_byte: from.saturating_add(leading),
        end_byte: to.saturating_sub(trailing),
    })
}

/// The reported occurrence that answers one judged side: the same file,
/// overlapping the judged lines.
fn answering_occurrence<'a>(cluster: &'a Value, side: &Range) -> Result<&'a Value> {
    occurrences(cluster)
        .iter()
        .find(|occurrence| {
            let (start, end) = occurrence_line_span(occurrence);
            field(occurrence, "path").as_str() == Some(side.path.as_str())
                && start <= u64::from(side.end)
                && u64::from(side.start) <= end
        })
        .with_context(|| format!("no occurrence answers {side:?}: {cluster:#}"))
}

/// The two reported occurrences that answer one judged entry, in register
/// order: the exact extents a report renders, each comparable by
/// [PAIR-COMPARE-CLI].
fn answering_pair<'a>(report: &'a Value, entry: &ScoredEntry) -> Result<[&'a Value; 2]> {
    let id = entry
        .cluster
        .as_deref()
        .with_context(|| format!("judged pair is not reported: {:?}", entry.occurrences))?;
    let cluster = clusters(report)
        .iter()
        .find(|cluster| field(cluster, "id").as_str() == Some(id))
        .with_context(|| format!("cluster {id} is not in the report"))?;
    let sides: Vec<Range> = entry
        .occurrences
        .iter()
        .map(|range| Range::parse(range))
        .collect::<Result<_>>()?;
    let [left, right] = sides.as_slice() else {
        anyhow::bail!("a judged pair names two ranges: {:?}", entry.occurrences);
    };
    Ok([
        answering_occurrence(cluster, left)?,
        answering_occurrence(cluster, right)?,
    ])
}

/// Every judged string-heavy copy is admitted by explicit comparison of
/// the occurrences that report it, in both orders, with the same
/// classification and explanation each way.
#[test]
fn explicit_comparison_admits_every_judged_string_copy_both_ways() -> Result<()> {
    let (_tmp, root) = temp_scan_dir(JUDGED_FIXTURE)?;
    seed_repos(&root, &all_repos())?;
    let (report, score) = scan_and_score(&root, &register()?)?;
    assert_eq!(score.entries.len(), JUDGED_PAIRS);

    let mut refused = Vec::new();
    for entry in &score.entries {
        let [left, right] = answering_pair(&report, entry)?;
        let forward = compare_pair(&root, MIN_NODES, left, right)?.evidence;
        let backward = compare_pair(&root, MIN_NODES, right, left)?.evidence;
        let judged = &entry.occurrences;
        assert_eq!(forward.admitted, backward.admitted, "{judged:?}");
        assert_eq!(
            forward.classification, backward.classification,
            "{judged:?}"
        );
        assert_eq!(forward.explanation, backward.explanation, "{judged:?}");
        if !forward.admitted {
            refused.push(format!("{judged:?}: {}", forward.explanation));
            continue;
        }
        assert!(
            forward.classification.is_some(),
            "an admitted copy is classified: {judged:?}"
        );
        assert_ne!(forward.explanation, LITERAL_BOUND_REFUSAL, "{judged:?}");
        assert!(
            forward.agreement > 0.0,
            "a judged copy agrees with itself somewhere: {judged:?}"
        );
    }
    assert!(
        refused.is_empty(),
        "judged CLEARLY IN pairs refused by explicit comparison:\n{}",
        refused.join("\n")
    );
    Ok(())
}

/// Unrelated strings the issues judged apart are refused by the literal
/// rule, by name, in either order.
#[test]
fn explicit_comparison_refuses_judged_unrelated_strings_both_ways() -> Result<()> {
    let bytes = |root: &Path, (path, start_byte, end_byte): (&str, usize, usize)| PairEndpoint {
        path: root.join(path),
        start_byte,
        end_byte,
    };
    let osprey = fixture(OSPREY_FIXTURE);
    let sql = fixture(DATAPROVIDER_FIXTURE);
    for (source, root, left, right) in [
        (
            OSPREY_FIXTURE,
            &osprey,
            bytes(&osprey, USAGE_TEXT),
            bytes(&osprey, EMBEDDED_PROGRAM),
        ),
        (
            DATAPROVIDER_FIXTURE,
            &sql,
            endpoint(
                &sql,
                COLUMN_QUERY_LINES.0,
                COLUMN_QUERY_LINES.1,
                COLUMN_QUERY_LINES.2,
            )?,
            endpoint(
                &sql,
                INDEX_QUERY_LINES.0,
                INDEX_QUERY_LINES.1,
                INDEX_QUERY_LINES.2,
            )?,
        ),
    ] {
        for (first, second) in [(left.clone(), right.clone()), (right, left)] {
            let evidence = compare_endpoints(root, MIN_NODES, first, second)?.evidence;
            assert!(!evidence.admitted, "{source}: {evidence:#?}");
            assert_eq!(evidence.classification, None, "{source}");
            assert_eq!(evidence.explanation, LITERAL_BOUND_REFUSAL, "{source}");
        }
    }
    Ok(())
}
