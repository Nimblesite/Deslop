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
use crate::common::{golden::*, incremental::*, multilang::*, *};

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
