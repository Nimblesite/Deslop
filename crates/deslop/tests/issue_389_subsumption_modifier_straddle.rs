//! gh #389 — one physical duplication, published exactly once
//! ([PIPELINE-CLUSTER-SUBSUME], [FUSED-SHARED-SUBTREE]).
//!
//! `incremental-multilang` at `--min-nodes 8` used to carry **two**
//! `identical` clusters over the same C# file pair: the 44-node authored
//! method clone starting at `static` (`LedgerAlpha.cs` bytes 180–537),
//! and a 13-node sibling window over the method's signature line
//! starting at `public` (bytes 173–236). The two views disagreed about
//! whether the leading visibility modifier belonged to the method, so
//! per-occurrence containment failed by the 7 bytes of `public` and
//! [PIPELINE-CLUSTER-SUBSUME] published both — violating, on this input,
//! the spec's own stated motivation: publishing both shows the reader
//! one duplicate twice and double-counts it in `clusters_total` and in
//! the duplication metric.
//!
//! The issue named two candidate causes and asked for them to be
//! separated. The measurement settles it: the survivor is
//! `LedgerAlpha.cs` bytes **173–537**, which starts at `public`. The
//! method-declaration fingerprint's range now *includes* the leading
//! modifier, so both views describe the same span convention, ordinary
//! containment holds, and the election collapses the pair with no
//! straddle tolerance added to the predicate. That is the range-convention
//! answer, and this suite is what stops it regressing to the other one —
//! a subsumption predicate loosened to bare intersection is exactly what
//! [PIPELINE-CLUSTER-SUBSUME] rejects.
//!
//! Held at `--min-nodes 8` because that is the floor the issue
//! reproduces at. `incremental_multilang_golden.rs` runs the same corpus
//! at 20 — deliberately above 13 — so that suite could never see this
//! edge; the expectation table both suites read is the same one, so a
//! fixture edit that moves a span cannot be absorbed here while the
//! golden quietly disagrees.

use serde_json::Value;

use crate::common::{multilang::*, *};

/// The floor gh #389 reproduces at. The 13-node signature window only
/// exists below `MULTILANG_MIN_NODES`, so a suite that never scans this
/// low cannot pin the escape.
const MIN_NODES: u32 = 8;

/// Renders the authored corpus into a throwaway scan root. The
/// checked-in fixture is never scanned in place — a store-backed run
/// writes `.deslop/cache` into its scan root ([OUTPUT-DIR]).
fn render() -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_multilang(&scan_root)?;
    run_report(&scan_root, MIN_NODES)
}

/// Every visible cluster holding an occurrence in either of a case's two
/// files. The count is the assertion: two clusters over one pair *is*
/// the defect, whatever each one is labelled.
fn clusters_over_pair<'a>(report: &'a Value, case: &LangCase) -> Vec<&'a Value> {
    clusters(report)
        .iter()
        .filter(|cluster| {
            occurrence_files(cluster)
                .iter()
                .any(|file| case.files().contains(&file.as_str()))
        })
        .collect()
}

/// The occurrence rendered for one file of a pair.
fn occurrence_for<'a>(cluster: &'a Value, file: &str) -> Result<&'a Value> {
    occurrences(cluster)
        .iter()
        .find(|occurrence| occurrence_path(occurrence).is_ok_and(|path| path.ends_with(file)))
        .ok_or_else(|| anyhow::anyhow!("cluster published no occurrence in {file}"))
}

/// The rendered `(start_line, end_line, start_byte, end_byte)` of one
/// occurrence — every field a reader clicks or an agent slices.
fn span_of(occurrence: &Value) -> Result<OccurrenceSpan> {
    let number = |name: &str| -> Result<u64> {
        occurrence
            .get(name)
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("occurrence has no numeric `{name}`: {occurrence}"))
    };
    Ok((
        number("start_line")?,
        number("end_line")?,
        number("start_byte")?,
        number("end_byte")?,
    ))
}

/// One language: exactly one published cluster over the pair, byte-proven,
/// at the span the shared expectation table records.
fn assert_published_once(report: &Value, case: &LangCase) -> Result<()> {
    let language = case.language;
    let over_pair = clusters_over_pair(report, case);
    assert_eq!(
        over_pair.len(),
        1,
        "{language}: one physical duplication must be published once, not {count} times — \
         two views of one clone double-count it in `clusters_total` and in the duplication \
         metric (gh #389). Published: {published:#?}",
        count = over_pair.len(),
        published = over_pair
            .iter()
            .map(|cluster| (
                cluster_id(cluster),
                cluster_bucket(cluster),
                cluster_size(cluster)
            ))
            .collect::<Vec<_>>(),
    );
    let [cluster] = over_pair.as_slice() else {
        anyhow::bail!(
            "{language}: expected exactly one cluster over {:?}",
            case.files()
        );
    };
    assert_eq!(
        cluster_bucket(cluster),
        "identical",
        "{language}: the authored pair is byte-identical — {dump}",
        dump = cluster_id(cluster)
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "{language}: the surviving cluster keeps both occurrences; a subsumption that \
         eats one of them is a false negative — {id}",
        id = cluster_id(cluster)
    );
    assert_eq!(
        cluster_id(cluster),
        case.cluster_id,
        "{language}: cluster ids travel into editor state and MCP lookups \
         ([PIPELINE-DETERMINISM])"
    );
    for (file, expected) in case.spans() {
        let occurrence = occurrence_for(cluster, file)?;
        assert_eq!(
            span_of(occurrence)?,
            expected,
            "{language}: {file} must be published at the span both views agree on; \
             a range convention that excludes the leading modifier is what broke \
             containment by 7 bytes (gh #389)"
        );
    }
    Ok(())
}

// [PIPELINE-CLUSTER-SUBSUME] The C# modifier straddle, and the same
// contract in the five languages beside it: at the floor the escape
// lives at, every authored pair is published exactly once.
#[test]
fn one_duplication_per_language_is_published_exactly_once() -> Result<()> {
    let report = render()?;
    for case in MULTILANG_CASES {
        assert_published_once(&report, case)?;
    }
    assert_eq!(
        clusters_hidden(&report),
        0,
        "no group may be suppressed to reach the single-publication count — \
         hiding the duplicate view would satisfy the count while still \
         double-counting it in the metrics: {report:#}"
    );
    Ok(())
}

// [PIPELINE-CLUSTER-SUBSUME] The straddle itself, stated in bytes: the
// published C# range starts at the `public` modifier, which is the fact
// that makes containment hold. Asserting only the collapsed count would
// pass just as well if the *signature* view had won and the report had
// shrunk to one 13-node line — the opposite defect, and a false negative
// over the 15 lines of method body.
#[test]
fn the_csharp_range_starts_at_the_modifier_and_covers_the_whole_method() -> Result<()> {
    let report = render()?;
    let case = MULTILANG_CASES
        .iter()
        .find(|case| case.language == "csharp")
        .ok_or_else(|| anyhow::anyhow!("the fixture no longer stages a C# pair"))?;
    let cluster = expect_cluster_spanning(&report, &case.files())?;

    let source = std::fs::read_to_string(multilang_corpus().join(case.alpha))?;
    let (start_line, end_line, start_byte, end_byte) = case.alpha_span;
    let published = source
        .get(usize::try_from(start_byte)?..usize::try_from(end_byte)?)
        .ok_or_else(|| anyhow::anyhow!("published range falls outside {}", case.alpha))?;

    assert!(
        published.starts_with("public static long ReconcileEntries"),
        "the published range must open on the visibility modifier — the 7 bytes of \
         `public` are the whole of the straddle (gh #389): {published:?}"
    );
    assert!(
        published.trim_end().ends_with('}'),
        "the published range must close on the method's own brace, not on its \
         signature line: {published:?}"
    );
    assert_eq!(
        published.lines().count(),
        usize::try_from(end_line.saturating_sub(start_line).saturating_add(1))?,
        "the rendered line span and the rendered byte span must describe the same \
         region: {published:?}"
    );
    assert_eq!(
        occurrence_text(&multilang_corpus(), occurrence_for(cluster, case.beta)?)?,
        published,
        "both occurrences are byte-identical; the cluster claims `identical` and must \
         be able to prove it from the ranges it published"
    );
    Ok(())
}
