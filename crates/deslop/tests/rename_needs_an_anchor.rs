//! [FUSED-CONTENT-GATE] / [FUSED-CONTENT-GATE-INTERIOR] /
//! [FUSED-SHARED-SUBTREE-ECHO] / [FUSED-CANDIDATE-BUCKET-STAR] /
//! [PIPELINE-CLUSTER-EXACT-SCOPE-SCRAPS] / [RANK-MASS-SUM] — a rename is
//! only *proven* by evidence the rename itself did not supply, and it is
//! judged on the whole method it lives in.
//!
//! `InventoryApi` and `CatalogApi` are the Dart sibling-method idiom:
//! seven `Future<List<String>>` accessors that share a shape and differ
//! in exactly one authored thing, the endpoint each one calls
//! (`stock-locations`, `display-banners`, ...). The family is a real
//! Type-2 duplicate — one method written seven times under a
//! consistent rename, every copy calling the same `/catalog/$uid/settings/`
//! prefix — and the report says so at the extent the author wrote it:
//! seven whole methods.
//!
//! # The mechanism this file guards
//!
//! The endpoint literal is on line 11 of each method. A window carved
//! from lines 12-15 keeps every position a wholesale substitution
//! touched (`stockRows` -> `bannerRows`) and leaves out the one position
//! that could contradict it. Over such a window `rename_consistency`
//! measured a literal population of *size zero* and scored it `1.0`, so
//! the whole proof rested on the substitution being consistent with
//! itself, and the family published as literal-free fragments ranked on
//! evidence they had excluded.
//!
//! [FUSED-CONTENT-GATE-INTERIOR] closes that: a literal-free window
//! strictly inside a function anchors a rename on identity identifiers
//! and affirming literals only. [FUSED-SHARED-SUBTREE-ECHO] keeps the
//! class shell and the method body from welding into the family as
//! wider or narrower views of the same method. [FUSED-CANDIDATE-BUCKET-STAR]
//! gives the second and third accessors of the first file a cross-file
//! candidate instead of only the within-file pair the promote floor
//! refuses, and [PIPELINE-CLUSTER-EXACT-SCOPE-SCRAPS] keeps the two
//! field declarations above the first accessor from widening it into a
//! "fields plus method" window. What remains is the authored view,
//! judged with the endpoint inside it.
//!
//! # The contract
//!
//! The family publishes as seven whole methods, each containing the
//! endpoint it calls; it carries no pair claim; and [RANK-MASS-SUM]
//! orders it above the byte-identical control on duplicated mass alone.

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

/// The two sibling-method classes.
const FAMILY_FILES: [&str; 2] = ["inventory_api.dart", "catalog_api.dart"];

/// The unrelated byte-identical pair: the false-negative control.
const CONTROL_FILES: [&str; 2] = ["control_alpha.dart", "control_beta.dart"];

/// The line, in every method of both classes, carrying the endpoint
/// literal — the one authored byte-difference between the siblings.
///
/// The methods repeat on a fixed nine-line stride, so a window that
/// covers a method's authored content covers exactly one of these.
const ENDPOINT_LITERAL_LINES: [u64; 4] = [11, 20, 29, 38];

/// The seven whole accessor methods, `(file, first line, last line)`:
/// the extent [FUSED-CONTENT-GATE] judges and the report publishes.
const FAMILY_METHOD_SPANS: [(&str, u64, u64); 7] = [
    ("catalog_api.dart", 9, 16),
    ("catalog_api.dart", 18, 25),
    ("catalog_api.dart", 27, 34),
    ("inventory_api.dart", 9, 16),
    ("inventory_api.dart", 18, 25),
    ("inventory_api.dart", 27, 34),
    ("inventory_api.dart", 36, 43),
];

/// Normalised nodes in one whole accessor method — the class member
/// the author wrote — so the family's mass is this times six copies.
const FAMILY_METHOD_NODES: u64 = 102;

/// Duplicated copies in the family: seven occurrences, one canonical
/// ([RANK-MASS-SUM] counts `visible − 1`).
const FAMILY_DUPLICATED_COPIES: u64 = 6;

/// The class prefix both files share — constructor, the two fields, and
/// the first three accessors — published as its own Merkle-equal
/// clone: it names no file the family does not, but `InventoryApi`'s
/// fourth accessor lies outside it, so [PIPELINE-CLUSTER-SUBSUME] keeps
/// both views.
const CLASS_PREFIX_SPANS: [(&str, u64, u64); 2] =
    [("catalog_api.dart", 4, 34), ("inventory_api.dart", 4, 34)];

/// Every cluster the fixture publishes, in [RANK-MASS-SUM] order: the
/// family, the class prefix, the control.
const PUBLISHED_CLUSTER_COUNT: usize = 3;

/// Rank of the family: the most duplicated mass in the corpus.
const FAMILY_RANK: u64 = 1;

/// Rank of the shared class prefix.
const CLASS_PREFIX_RANK: u64 = 2;

/// Rank of the byte-identical control: two copies of one function
/// carry less mass than six copies of another.
const CONTROL_RANK: u64 = 3;

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

/// One occurrence's `(path, start_line, end_line)`, sorted so the list
/// compares against the authored spans regardless of report order.
fn occurrence_spans(cluster: &Value) -> Vec<(String, u64, u64)> {
    let mut spans: Vec<(String, u64, u64)> = cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |values| {
            values
                .iter()
                .filter_map(|occurrence| {
                    Some((
                        occurrence
                            .get("path")?
                            .as_str()?
                            .rsplit('/')
                            .next()?
                            .to_owned(),
                        occurrence.get("start_line")?.as_u64()?,
                        occurrence.get("end_line")?.as_u64()?,
                    ))
                })
                .collect()
        });
    spans.sort();
    spans
}

/// The authored spans as owned tuples, sorted like [`occurrence_spans`].
fn expected_spans(spans: &[(&str, u64, u64)]) -> Vec<(String, u64, u64)> {
    let mut expected: Vec<(String, u64, u64)> = spans
        .iter()
        .map(|(path, start, end)| ((*path).to_owned(), *start, *end))
        .collect();
    expected.sort();
    expected
}

/// The `rank` the report stamped on a cluster.
fn rank_of(cluster: &Value) -> Option<u64> {
    field(cluster, "rank").as_u64()
}

// The user-visible half: the family is duplication, and the report says
// so without a pair claim on the cluster. The byte-identical control is
// judged in the same run, so a detector that had simply stopped
// producing candidates fails here too.
#[test]
fn sibling_accessors_publish_as_one_family_without_a_pair_claim() -> Result<()> {
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
    let family = expect_cluster_spanning(&report, &FAMILY_FILES)?;
    assert_no_pair_surface_on_cluster(family, "rename-needs-an-anchor family");
    assert_structural_only_contract(family, "rename-needs-an-anchor family");
    assert_eq!(
        occurrence_spans(family).len(),
        FAMILY_METHOD_SPANS.len(),
        "the family is seven sibling methods: {published:#?}",
        published = published(&report),
    );
    Ok(())
}

// The mechanism, asserted where it happens: a rename is *proven* only
// by evidence the substitution did not itself supply. The family
// therefore publishes at the extent that holds that evidence — the
// whole method, endpoint included — and never as the literal-free
// window inside it or the class shell around it.
#[test]
fn a_rename_is_never_proven_without_an_anchor_outside_it() -> Result<()> {
    let report = render()?;
    let family = expect_cluster_spanning(&report, &FAMILY_FILES)?;
    assert_eq!(
        occurrence_spans(family),
        expected_spans(&FAMILY_METHOD_SPANS),
        "[FUSED-CONTENT-GATE-INTERIOR] the family publishes as the seven whole \
         accessors, each judged with its endpoint inside the window: {published:#?}",
        published = published(&report),
    );
    assert_eq!(
        field(family, "canonical_node_count").as_u64(),
        Some(FAMILY_METHOD_NODES),
        "the canonical occurrence is one whole accessor method: {family:#}",
    );
    Ok(())
}

// The window may not be chosen for what it leaves out. Each accessor's
// authored difference is its endpoint literal; a published window over
// these methods has to contain it.
#[test]
fn a_published_accessor_window_contains_the_endpoint_it_calls() -> Result<()> {
    let report = render()?;
    let family = expect_cluster_spanning(&report, &FAMILY_FILES)?;
    let spans = occurrence_spans(family);
    assert_eq!(
        spans.len(),
        FAMILY_METHOD_SPANS.len(),
        "every accessor is an occurrence: {published:#?}",
        published = published(&report),
    );
    for (path, start_line, end_line) in spans {
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

// [RANK-MASS-SUM] Mass alone orders the report. Six duplicated copies of
// a whole accessor out-weigh one duplicated copy of the control, however
// byte-exact the control is; the shared class prefix sits between them.
#[test]
fn the_accessor_family_outranks_the_real_clone_by_mass() -> Result<()> {
    let report = render()?;
    let ranked = clusters(&report);
    assert_eq!(
        ranked.len(),
        PUBLISHED_CLUSTER_COUNT,
        "the family, the class prefix, and the control — nothing else: {published:#?}",
        published = published(&report),
    );
    let family = expect_cluster_spanning(&report, &FAMILY_FILES)?;
    let control = expect_cluster_spanning(&report, &CONTROL_FILES)?;
    assert_eq!(
        rank_of(family),
        Some(FAMILY_RANK),
        "family rank: {family:#}"
    );
    assert_eq!(
        rank_of(control),
        Some(CONTROL_RANK),
        "control rank: {control:#}"
    );
    let family_mass = field(family, "mass").as_u64().unwrap_or(0);
    let control_mass = field(control, "mass").as_u64().unwrap_or(0);
    assert_eq!(
        family_mass,
        FAMILY_METHOD_NODES * FAMILY_DUPLICATED_COPIES,
        "family mass is canonical nodes × six duplicated copies: {family:#}",
    );
    assert!(
        family_mass > control_mass,
        "[RANK-MASS-SUM] the family ({family_mass}) out-weighs the control \
         ({control_mass}): {published:#?}",
        published = published(&report),
    );
    let class_prefix = ranked
        .iter()
        .find(|cluster| rank_of(cluster) == Some(CLASS_PREFIX_RANK))
        .ok_or_else(|| anyhow::anyhow!("no cluster at rank {CLASS_PREFIX_RANK}"))?;
    assert_eq!(
        occurrence_spans(class_prefix),
        expected_spans(&CLASS_PREFIX_SPANS),
        "the shared class prefix is the second-heaviest duplication: {published:#?}",
        published = published(&report),
    );
    let class_prefix_mass = field(class_prefix, "mass").as_u64().unwrap_or(0);
    assert!(
        family_mass > class_prefix_mass && class_prefix_mass > control_mass,
        "masses descend with rank ({family_mass} > {class_prefix_mass} > {control_mass}): \
         {published:#?}",
        published = published(&report),
    );
    Ok(())
}
