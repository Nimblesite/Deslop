//! [FUSED-CONTENT-GATE] / [RANK-STRUCTURAL-ONLY] — a rename is only
//! *proven* by evidence the rename itself did not supply.
//!
//! `InventoryApi` and `CatalogApi` are the Dart sibling-method idiom:
//! seven `Future<List<String>>` accessors that share a shape and differ
//! in exactly one authored thing, the endpoint each one calls
//! (`stock-locations`, `display-banners`, ...). Nobody copied anything;
//! the shape is what the HTTP client mandates.
//!
//! The report published them as `nearly_identical`, with
//! `pair_rename_consistency = 1.000`, ranked **above** a byte-identical
//! clone in the same corpus — an explicit duplication claim about
//! seven methods that call seven different endpoints.
//!
//! # The mechanism
//!
//! The selected window is lines 12-15 of each method, and the endpoint
//! literal is on line 11. Every position the window keeps is a local
//! whose name was substituted wholesale (`stockRows` -> `bannerRows`),
//! and the one position that would have contradicted the substitution
//! is on the line the window excludes.
//!
//! `pair_rename_consistency` then measures a literal population of
//! *size zero* and scores it `1.0` — "every literal is preserved" is
//! the same number as "there are no literals" — so the whole proof
//! rests on the identifier substitution, and the anchor mass that is
//! supposed to price the coincidence is drawn from that same
//! substitution. The claim is its own evidence.
//!
//! `'/catalog/$uid/settings/{endpoint}'` is what makes the window land
//! there: with the interpolation removed the selected window covers the
//! literal, the family measures `rename_consistency = 0.000` and is
//! correctly published as `structural_only`. Nothing about the authored
//! duplication changed between those two runs — only which bytes the
//! window happened to enclose.
//!
//! # The contract
//!
//! Not "never cluster them" — seven methods of one shape are worth a
//! reader's attention. The contract is that the tool may not make its
//! strongest claim on a window it selected for containing no
//! contradicting evidence, and may not report a rename as *proven* when
//! no anchor outside the substitution corroborates it.

use serde_json::Value;

use crate::common::{signals::*, *};

/// The fixture: two sibling-method API classes plus an unrelated
/// byte-identical pair.
const FIXTURE: &str = "dart-rename-without-anchors";

/// Node floor. 30 is the floor `rank_structural_only_policy` pins the
/// same geometry at, so the two suites cannot disagree about which
/// windows are candidates.
const MIN_NODES: u32 = 30;

/// Every file the fixture holds. Asserted per run so a family that
/// published nothing can only ever mean it was analysed and excluded.
const FIXTURE_FILE_COUNT: u64 = 5;

/// The two sibling-method classes. Nothing in them is copied.
const FAMILY_FILES: [&str; 2] = ["inventory_api.dart", "catalog_api.dart"];

/// The unrelated byte-identical pair: the false-negative control.
const CONTROL_FILES: [&str; 2] = ["control_alpha.dart", "control_beta.dart"];

/// The line, in every method of both classes, carrying the endpoint
/// literal — the one authored byte-difference between the siblings, and
/// the evidence the promoted window excludes.
///
/// The methods repeat on a fixed nine-line stride, so a window that
/// covers a method's authored content covers exactly one of these.
const ENDPOINT_LITERAL_LINES: [u64; 4] = [11, 20, 29, 38];

/// Renders the fixture.
fn render() -> Result<Value> {
    run_report(&fixture(FIXTURE), MIN_NODES)
}

/// Every visible cluster as `id [mass] files`.
fn published(report: &Value) -> Vec<String> {
    clusters(report)
        .iter()
        .map(|cluster| {
            format!(
                "{id} [mass={mass}] {files:?}",
                id = cluster_id(cluster),
                mass = field(cluster, "mass").as_u64().unwrap_or(0),
                files = occurrence_files(cluster),
            )
        })
        .collect()
}

/// One occurrence's `(path, start_line, end_line)`.
fn occurrence_spans(cluster: &Value) -> Vec<(String, u64, u64)> {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |values| {
            values
                .iter()
                .filter_map(|occurrence| {
                    Some((
                        occurrence.get("path")?.as_str()?.to_owned(),
                        occurrence.get("start_line")?.as_u64()?,
                        occurrence.get("end_line")?.as_u64()?,
                    ))
                })
                .collect()
        })
}

// The user-visible half: seven methods calling seven different
// endpoints are not duplicated content, and the tool may not say they
// are. The byte-identical control is judged in the same run, so a
// detector that had simply stopped producing candidates fails here
// too.
#[test]
fn sibling_accessors_never_claim_duplication() -> Result<()> {
    let report = render()?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FIXTURE_FILE_COUNT),
        "every fixture file must reach the pipeline before any verdict about \
         the family means anything: {published:#?}",
        published = published(&report),
    );
    let scan_root = fixture(FIXTURE);
    let control = expect_cluster_spanning(&report, &CONTROL_FILES)?;
    assert_structural_only_contract(control, "rename-needs-an-anchor control");
    assert_no_pair_surface_on_cluster(control, "rename-needs-an-anchor control");
    assert!(
        has_verbatim_pair(&scan_root, control)?,
        "the byte-identical control must still be published as duplication in \
         this very run: {published:#?}",
        published = published(&report),
    );
    let Some(family) = cluster_spanning(&report, &FAMILY_FILES) else {
        return Ok(());
    };
    assert_no_pair_surface_on_cluster(family, "rename-needs-an-anchor family");
    Ok(())
}

// The mechanism, asserted where it happens: a rename is *proven* only
// by evidence the substitution did not itself supply. Saturated
// `rename_consistency` over a window holding no literal is the
// measurement reading its own input back.
#[test]
fn a_rename_is_never_proven_without_an_anchor_outside_it() -> Result<()> {
    let report = render()?;
    let Some(family) = cluster_spanning(&report, &FAMILY_FILES) else {
        return Ok(());
    };
    assert_no_pair_surface_on_cluster(family, "rename-needs-an-anchor family");
    Ok(())
}

// The window may not be chosen for what it leaves out. Each accessor's
// authored difference is its endpoint literal; a published window over
// these methods has to contain it.
#[test]
fn a_published_accessor_window_contains_the_endpoint_it_calls() -> Result<()> {
    let report = render()?;
    let Some(family) = cluster_spanning(&report, &FAMILY_FILES) else {
        return Ok(());
    };
    for (path, start_line, end_line) in occurrence_spans(family) {
        assert!(
            ENDPOINT_LITERAL_LINES
                .iter()
                .any(|line| (start_line..=end_line).contains(line)),
            "{path} L{start_line}-{end_line} is a window carved out of an accessor \
             that excludes the endpoint literal on one of lines \
             {ENDPOINT_LITERAL_LINES:?} — the one authored byte that separates \
             these methods. A duplication claim measured over the evidence-free \
             remainder of a body is a claim about the scaffolding, not the code: \
             {published:#?}",
            published = published(&report),
        );
    }
    Ok(())
}

// Neither shape-only family may outrank a real clone.
#[test]
fn the_real_clone_outranks_the_accessor_family() -> Result<()> {
    let report = render()?;
    let control = expect_cluster_spanning(&report, &CONTROL_FILES)?;
    assert_eq!(
        cluster_id(clusters(&report).first().unwrap_or(&Value::Null)),
        cluster_id(control),
        "the one real duplication in this corpus must rank first: {published:#?}",
        published = published(&report),
    );
    Ok(())
}
