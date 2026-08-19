//! Six-language golden for the incremental parse store
//! ([PIPELINE-INCREMENTAL], [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE],
//! [PIPELINE-INCREMENTAL-ANALYSIS-REUSE], [PIPELINE-DETERMINISM]).
//!
//! `tests/fixtures/incremental-multilang/src` holds one authored Type-1
//! clone pair in each of Rust, Python, TypeScript, Dart, C# and Go,
//! scanned as a single mixed corpus. `expected-report.json` is the
//! committed cold rendering of it.
//!
//! The store keys on `(language_id, tool_version, min_nodes,
//! blake3(source))`. A single-language corpus cannot tell a correct key
//! from one that has dropped the language component, and cannot catch a
//! store that serves one language's tree from another language's slot —
//! both of which manufacture false positives and false negatives at
//! once. Six languages sharing one store is the shape that can.
//!
//! Three halves, extending the Phase 0 pattern of `report_golden.rs`
//! from "stable across time" to "stable across every cache state":
//!
//! - **unchanged** — a cold render must equal the committed bytes;
//! - **correct** — the committed golden must independently satisfy the
//!   contract the authored fixture sources imply, so a wrongly-blessed
//!   golden cannot self-certify;
//! - **warm** — a fully-warm store-backed run must reproduce that same
//!   golden, having rebuilt no signature at all.
//!
//! Regenerate with `DESLOP_BLESS=1 cargo test -p deslop --test
//! incremental_multilang_golden`, then review the diff — see
//! `tests/fixtures/incremental-multilang/README.md`.

use serde_json::Value;

mod common;
use crate::common::{golden::*, incremental::*, multilang::*, multilang_warm::*, verdict::*, *};

/// Renders the fixture cold, with the store never consulted, into a
/// throwaway scan root — the checked-in fixture is never scanned in
/// place, so no run can drop a `.deslop/` cache into the fixture tree.
/// Returns the report's raw bytes; the golden pins the serialisation,
/// not merely the decoded document.
fn render_cold_multilang() -> Result<Vec<u8>> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_multilang(&scan_root)?;
    let (bytes, _counters) = run_capturing_bytes(
        &scan_root,
        &tmp.path().join("out"),
        MULTILANG_MIN_NODES,
        Store::Off,
        &["--notext", "--nohtml"],
    )?;
    Ok(bytes)
}

/// The command that regenerates the committed golden.
const BLESS: &str = "`DESLOP_BLESS=1 cargo test -p deslop --test incremental_multilang_golden`";

/// Why a drift here is worth investigating before it is blessed away.
const DRIFT_HINT: &str = "Ranking, spans, ids and metrics are all user-visible, and a drift \
                          that touches only some of the six languages points straight at the \
                          store's language partitioning.";

// [PIPELINE-DETERMINISM] Half one: unchanged. Two cold renders of the
// mixed corpus must agree with each other and with the committed bytes.
#[test]
fn cold_multilang_report_matches_committed_golden_byte_for_byte() -> Result<()> {
    let rendered = String::from_utf8(render_cold_multilang()?)?;
    assert_eq!(
        rendered,
        String::from_utf8(render_cold_multilang()?)?,
        "two cold renders over the same six-language corpus must be bit-identical \
         [PIPELINE-DETERMINISM]"
    );
    assert_matches_golden(&rendered, &multilang_golden_path(), BLESS, DRIFT_HINT)
}

// [PIPELINE-DETERMINISM] Half two: correct. Byte equality only proves
// the tool still agrees with a file the tool wrote. These invariants
// come from the authored fixture sources, so a wrongly-blessed golden —
// one blessed while a language was silently missing, or while the store
// was cross-serving trees — fails here even though its bytes match.
#[test]
fn committed_multilang_golden_satisfies_the_authored_contract() -> Result<()> {
    let golden = load_golden(&multilang_golden_path(), BLESS)?;
    assert_multilang_contract(&golden, "committed golden")?;
    assert_cold_header(&golden);
    assert_every_occurrence_is_a_real_clone(&golden)?;
    assert_one_cluster_per_language(&golden)?;
    assert_every_cluster_is_reported_exactly(&golden)?;
    assert_golden_metrics(&golden)?;
    Ok(())
}

/// Every user-visible field of every cluster, pinned per language from
/// [`MULTILANG_CASES`]: the stable id, the subtree size, the exact
/// occurrence spans, the bucket, the category, and all four signals.
///
/// The looser halves above would survive drifts that matter. A cluster
/// can span the right file pair with a moved span, a re-derived id, a
/// halved node count, or — the audit's regression — a `token_jaccard`
/// that changed while nothing else did. Each of those is a different
/// report for the same source, so each gets its own assertion.
fn assert_every_cluster_is_reported_exactly(golden: &Value) -> Result<()> {
    for case in MULTILANG_CASES {
        let language = case.language;
        let clone = expect_lang_clone(golden, case)?;
        assert_eq!(
            cluster_id(clone),
            case.cluster_id,
            "{language}: cluster ids are stable across runs and sessions \
             ([PIPELINE-DETERMINISM]); a changed id breaks every consumer \
             holding the old one: {clone:#}"
        );
        assert_eq!(
            field(clone, "canonical_node_count").as_u64(),
            Some(case.nodes),
            "{language}: the authored clone is a {} node subtree, and \
             ranking weight is computed from that count: {clone:#}",
            case.nodes
        );
        assert_eq!(
            field(clone, "category").as_str(),
            Some("logic"),
            "{language}: an extractable reconciliation routine is `logic`, \
             never a demoted data table ([RANK-CATEGORY]): {clone:#}"
        );
        assert_occurrence_shape(clone, language);
        assert_exact_spans(clone, case)?;
        assert_pinned_signals(clone, language);
    }
    Ok(())
}

/// The occurrence-set shape: exactly two live copies, nothing truncated,
/// and neither copy hidden. A hidden occurrence would silently shrink
/// the visible cluster size and the ranking weight derived from it.
fn assert_occurrence_shape(clone: &Value, language: &str) {
    assert_eq!(
        (
            cluster_size(clone),
            field(clone, "occurrences_total").as_u64(),
            field(clone, "occurrences_truncated").as_bool(),
        ),
        (2, Some(2), Some(false)),
        "{language}: the authored pair is two untruncated occurrences: {clone:#}"
    );
    for occurrence in occurrences(clone) {
        assert_eq!(
            field(occurrence, "hidden").as_bool(),
            Some(false),
            "{language}: neither authored copy is generated or hidden \
             ([EXCLUSION-CONFIG]): {occurrence:#}"
        );
    }
}

/// The exact `(start_line, end_line, start_byte, end_byte)` of both
/// occurrences, matched by file name so occurrence order cannot mask a
/// swap — the failure mode the audit's blob-swap probe produced, where
/// the two files kept their names and exchanged their spans.
fn assert_exact_spans(clone: &Value, case: &LangCase) -> Result<()> {
    let language = case.language;
    for (file, expected) in case.spans() {
        let occurrence = occurrences(clone)
            .iter()
            .find(|candidate| {
                field(candidate, "path")
                    .as_str()
                    .is_some_and(|path| path.ends_with(file))
            })
            .ok_or_else(|| anyhow::anyhow!("{language}: no occurrence for {file}: {clone:#}"))?;
        let actual = (
            field(occurrence, "start_line").as_u64().unwrap_or_default(),
            field(occurrence, "end_line").as_u64().unwrap_or_default(),
            field(occurrence, "start_byte").as_u64().unwrap_or_default(),
            field(occurrence, "end_byte").as_u64().unwrap_or_default(),
        );
        assert_eq!(
            actual, expected,
            "{language}: {file} must be reported at exactly \
             (start_line, end_line, start_byte, end_byte) {expected:?} — a \
             reader clicks the line and an agent slices the bytes: {occurrence:#}"
        );
    }
    Ok(())
}

/// All four signals, exactly. Embeddings are off and both copies are
/// byte-identical, so every value is determined — there is no band to
/// hide inside ([FUSED-THRESHOLD]).
fn assert_pinned_signals(clone: &Value, language: &str) {
    for (name, expected) in MULTILANG_SIGNALS {
        let actual = signal(clone, name);
        assert!(
            approx(actual, *expected),
            "{language}: signal `{name}` must be {expected}, got {actual}. A \
             signal that moves while the source does not is the corrupted- \
             or misaddressed-blob signature ([PIPELINE-INCREMENTAL-INTEGRITY]): \
             {clone:#}"
        );
    }
}

/// [METRICS-REPO] The reported figures must be transparent and
/// reproducible: the exact totals, *and* the arithmetic that connects
/// them. The per-file rows are re-summed here and the percentage
/// re-divided, so a golden whose header numbers were blessed from a
/// different corpus state fails even though each number looks plausible
/// on its own.
fn assert_golden_metrics(golden: &Value) -> Result<()> {
    assert_eq!(
        (
            metric_field(golden, "analysed_loc").as_u64(),
            metric_field(golden, "duplicated_loc").as_u64(),
            metric_field(golden, "clusters_total").as_u64(),
            metric_field(golden, "duplicated_files").as_u64(),
        ),
        (Some(210), Some(136), Some(6), Some(MULTILANG_FILE_COUNT)),
        "the twelve-file corpus measures 210 analysed / 136 duplicated LOC \
         across 6 clusters, every file duplicated: {golden:#}"
    );
    assert_metric_arithmetic(golden)?;
    assert_eq!(
        field(metric_field(golden, "threshold"), "source").as_str(),
        Some("none"),
        "the fixture opts into no CI gate, so the threshold source is `none` \
         ([EXIT-CODES]): {golden:#}"
    );
    assert_eq!(
        field(metric_field(golden, "threshold"), "breached").as_bool(),
        Some(false),
        "no gate can be breached when none is configured: {golden:#}"
    );
    assert_eq!(
        field(golden, "embedding_provenance"),
        &Value::Null,
        "the golden is rendered with `--embeddings off`, so it declares no \
         embedding provenance: {golden:#}"
    );
    Ok(())
}

/// Re-derives every headline metric from the parts of the report that
/// are independently asserted elsewhere — the per-file rows and the
/// cluster spans — so no figure is taken on the renderer's word.
fn assert_metric_arithmetic(golden: &Value) -> Result<()> {
    let rows = per_file_metrics(golden);
    assert_eq!(
        rows.len() as u64,
        MULTILANG_FILE_COUNT,
        "every analysed file needs its own per-file row: {golden:#}"
    );
    let summed_analysed: u64 = rows
        .iter()
        .map(|row| field(row, "analysed_loc").as_u64().unwrap_or_default())
        .sum();
    let summed_duplicated: u64 = rows
        .iter()
        .map(|row| field(row, "duplicated_loc").as_u64().unwrap_or_default())
        .sum();
    assert_eq!(
        (summed_analysed, summed_duplicated),
        (
            metric_field(golden, "analysed_loc").as_u64().unwrap_or(0),
            metric_field(golden, "duplicated_loc").as_u64().unwrap_or(0),
        ),
        "the repo totals must be the sum of the per-file rows: {golden:#}"
    );
    assert_eq!(
        visible_duplicated_loc(golden),
        summed_duplicated,
        "duplicated LOC must equal the lines the visible cluster spans \
         actually cover — the metric and the clusters are two views of one \
         measurement: {golden:#}"
    );
    let expected_percent = 100.0 * loc_as_f64(summed_duplicated)? / loc_as_f64(summed_analysed)?;
    let reported = metric_field(golden, "duplication_percent")
        .as_f64()
        .unwrap_or(-1.0);
    assert!(
        approx(reported, expected_percent),
        "duplication_percent must be duplicated/analysed × 100 = \
         {expected_percent}, got {reported}: {golden:#}"
    );
    Ok(())
}

/// The fixed-flag scan header: the pinned `min_nodes`, nothing hidden,
/// and a store that was never consulted — the golden is rendered with
/// `--no-incremental`, so both cache counters must be zero. A golden
/// blessed from a warm run would carry non-zero counters and is not a
/// cold baseline at all.
fn assert_cold_header(golden: &Value) {
    assert_eq!(
        field(golden, "min_nodes").as_u64(),
        Some(u64::from(MULTILANG_MIN_NODES)),
        "the golden must be rendered at the pinned subtree floor: {golden}"
    );
    assert_eq!(
        clusters_hidden(golden),
        0,
        "six authored Type-1 clones are real duplication, not hidden noise: {golden}"
    );
    assert_eq!(
        (
            field(field(golden, "cache_stats"), "hits").as_u64(),
            field(field(golden, "cache_stats"), "misses").as_u64(),
        ),
        (Some(0), Some(0)),
        "the cold golden is rendered with the store off, so it consulted nothing: {golden}"
    );
}

/// Every occurrence the golden reports must slice back out of the
/// authored fixture source, and the slices within one cluster must be
/// byte-identical to each other — the definition of the `identical`
/// bucket the contract asserts. This is what makes the golden falsifiable
/// against the corpus rather than against itself: a span that drifted by
/// one byte, or an occurrence pointing into the wrong file, fails here.
fn assert_every_occurrence_is_a_real_clone(golden: &Value) -> Result<()> {
    let corpus = multilang_corpus();
    for case in MULTILANG_CASES {
        let language = case.language;
        let clone = expect_lang_clone(golden, case)?;
        let texts = occurrence_texts(&corpus, clone)?;
        let first = texts
            .first()
            .ok_or_else(|| anyhow::anyhow!("{language}: clone reports no occurrences"))?;
        for (text, path) in texts.iter().zip(occurrence_paths(clone)) {
            assert_eq!(
                text, first,
                "{language}: {path} must slice to the same bytes as its sibling \
                 occurrence — an `identical` cluster whose members differ is a \
                 false positive"
            );
        }
        assert!(
            first.contains("balance"),
            "{language}: the reported span must be the authored reconciliation \
             body, not some incidental subtree; got: {first}"
        );
        assert_eq!(
            texts.len(),
            2,
            "{language}: the authored pair has exactly two copies"
        );
    }
    Ok(())
}

/// Exactly one cluster per language, each language present exactly once,
/// and weights ranked non-increasing. A store that leaked between
/// languages would show up as a duplicate or missing entry in this list.
fn assert_one_cluster_per_language(golden: &Value) -> Result<()> {
    let mut languages = lang_of_every_cluster(golden)?;
    let ranked = languages.clone();
    languages.sort_unstable();
    languages.dedup();
    assert_eq!(
        languages.len(),
        ranked.len(),
        "each language contributes exactly one cluster; ranked order was \
         {ranked:?}: {golden:#}"
    );
    assert_eq!(
        languages.len(),
        MULTILANG_CASES.len(),
        "every authored language must appear: {ranked:?}: {golden:#}"
    );
    let weights: Vec<f64> = clusters(golden)
        .iter()
        .map(|cluster| field(cluster, "weight").as_f64().unwrap_or(-1.0))
        .collect();
    for pair in weights.windows(2) {
        if let [higher, lower] = pair {
            assert!(
                higher >= lower,
                "clusters must be ranked by non-increasing weight, got {weights:?}: {golden:#}"
            );
        }
    }
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] Half three: warm. A
// store-backed pass over the unchanged six-language corpus owes the
// committed cold golden field for field, with `cache_stats` the sole
// permitted difference — and it owes it having rebuilt no signature.
#[test]
fn fully_warm_multilang_run_reproduces_the_committed_golden() -> Result<()> {
    // Seeding a warm corpus *is* the cold-fills / warm-serves /
    // warm-owes-cold contract over twelve byte-distinct files spanning
    // six parsers, and it asserts the recall floor on the warm pass — the
    // same helper the invalidation matrix starts every scenario from, so
    // the two suites cannot disagree about what "warmed" means.
    let corpus = WarmCorpus::warm()?;
    let (cold, warm) = (&corpus.cycle.cold, corpus.baseline());
    assert_multilang_contract(cold, "store-on cold")?;
    let golden = load_golden(&multilang_golden_path(), BLESS)?;
    assert_reports_equal(
        warm,
        &golden,
        "fully-warm six-language pass vs committed golden",
    );
    assert_reports_equal(
        cold,
        &golden,
        "store-on cold six-language pass vs committed golden",
    );
    Ok(())
}
