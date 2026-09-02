//! End-to-end signature/detection tests for the Dart language plug-in
//! ([LANG-CAND-DART], [PIPELINE-LANG-TRAIT]). Mirrors the Python suite in
//! [`python_signatures`]: it drives the `deslop` binary as a black box
//! against Dart fixtures and asserts on the rendered JSON report.
//!
//! These prove the full pipeline works for Dart end to end:
//!   - Type-2 renamed clones reach `structural = 1.0` AND
//!     `token_jaccard = 1.0` (identical k-gram sets after Dart
//!     normalisation collapse identifiers/literals).
//!   - A whole-function near-miss still produces a genuine cross-file
//!     cluster with `structural = 1.0` on the shared sub-structures —
//!     proving the Dart structural fingerprint path detects Type-3
//!     near-misses across files, while the signature-only sibling match
//!     is correctly suppressed ([CLONE-NOISE-SIGNATURE-ONLY], #154).
//!   - `token_jaccard` is bit-identical across process restarts
//!     (deterministic signatures).
//!   - False-positive regression guards proving the re-parse filter
//!     subsystem is fully wired for Dart — Dart previously bypassed it
//!     entirely because `grammar_for` had no Dart arm, so every member
//!     re-parse returned `None` and no filter could fire. These guards
//!     prove each filter now fires on Dart's real CST: generated
//!     `*.g.dart`/`*.freezed.dart` self-duplication is hidden (#95) while
//!     hand-written clones still surface, `export`/`import` barrels are
//!     not flagged (#96/#150/#155), signature-only structural matches are
//!     suppressed (#154), polymorphic method overrides sharing one name
//!     are suppressed (#69), and calls varying only in string-literal
//!     arguments are suppressed (#70) — each paired with a genuine clone
//!     that must still surface, proving the suppression stays targeted.

use anyhow::Result;

use crate::common::signals::{
    assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
};
use crate::common::*;

/// Drives the `deslop` binary over the named fixture at `min_nodes` and
/// returns the parsed JSON report, asserting the process exited cleanly.
fn run_cli(fixture_name: &str, min_nodes: u32) -> Result<serde_json::Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let min_nodes = min_nodes.to_string();
    let _assertion = deslop_cmd(&fixture(fixture_name), &output)?
        .args(["--min-nodes", min_nodes.as_str()])
        .assert()
        .success();
    load_json(&output.with_extension("json"))
}

// [FUSED-SIGNALS-THREE-LAYER] Type-2 Dart clones (identical after
// normalisation, every identifier renamed) must produce both
// `structural = 1.0` and `token_jaccard = 1.0` — the structural pass
// proves the Merkle hashes match and the MinHash pass proves identical
// k-gram sets map to identical signatures. On the mass-only wire the
// same fact is proven from the corpus bytes: the occurrences slice to
// identical source text ([PIPELINE-CLUSTER-CLOSURE]).
#[test]
fn dart_type2_clone_is_byte_identical_and_spans_both_files() -> Result<()> {
    let scan_root = fixture("dart-small");
    let report = run_cli("dart-small", 10)?;
    let clusters = clusters(&report);
    assert!(
        !clusters.is_empty(),
        "dart-small must produce at least one cluster",
    );
    let top = clusters
        .first()
        .ok_or_else(|| anyhow::anyhow!("dart-small must produce at least one cluster"))?;
    // A Type-2 clone is a rename: its occurrences differ in raw bytes
    // (`compute(input)` against `run(limit)`), so the wire proves it by
    // admission plus the byte truth that it is NOT a verbatim copy
    // ([PIPELINE-CLUSTER-CLOSURE]).
    assert_structural_only_contract(top, "Dart Type-2 clone");
    assert!(
        !has_verbatim_pair(&scan_root, top)?,
        "the Type-2 Dart clone is a rename — its occurrences must differ in \
         raw bytes, never be byte-identical: {top:#}",
    );
    let files = cluster_file_set(top);
    assert!(
        files.contains("alpha.dart") && files.contains("beta.dart"),
        "the Type-2 cluster must span both alpha.dart and beta.dart, got {files:?}",
    );
    Ok(())
}

// [FUSED-SIGNALS-THREE-LAYER] Two Dart functions sharing control-flow
// sub-structure (`if (_ < _) { return _; }`, `for (...) { _ = _ + _; }`)
// but differing in body length are a genuine Type-3 near-miss. The shared
// subtrees must surface as a cross-file cluster with `structural = 1.0`,
// proving the Dart structural fingerprint path detects near-misses across
// files.
//
// delta.dart: accumulate() runs `running + step` AND `running + 2` per
// iteration. epsilon.dart: aggregate() runs only `accumulator + cursor`.
// The signature-only sibling match — the two `int f(int)` headers, whose
// bodies differ — is a known false positive ([CLONE-NOISE-SIGNATURE-ONLY],
// #154) and is correctly suppressed; it must NOT be what carries the
// cluster, so we require substantive shape evidence, not a header match.
//
// The bound is two-sided ([FUSED-SHARED-SUBTREE], gh #408). It once
// required `structural == 1.0`, which only a byte-identical *fragment*
// nested inside the near-miss can satisfy — and reporting that fragment
// instead of the enclosing method is the recall hole #408 describes. A
// one-statement Type-3 near-miss cannot be Merkle-exact by
// construction, so exactness would mean the whole-method clone was
// missed. `dart-type3`'s enclosing pair measures 0.877; the
// signature-only header match it must not be measures far below the
// admission floor.
#[test]
fn dart_near_miss_produces_genuine_cross_file_structural_cluster() -> Result<()> {
    let scan_root = fixture("dart-type3");
    let report = run_cli("dart-type3", 8)?;
    let cluster = expect_cluster_spanning(&report, &["delta.dart", "epsilon.dart"])?;
    // Admission is the near-miss proof on the mass-only wire: the
    // enclosing pair cleared the shared-subtree bar or it would not be a
    // cluster. The byte-level truth is that the view is a near-miss, not
    // a verbatim copy — the whole point of gh #408 is that the enclosing
    // method differs by the extra statement.
    assert!(
        !has_verbatim_pair(&scan_root, cluster)?,
        "the reported view must be the enclosing near-miss (bytes differ by the \
         extra statement), not a Merkle-exact fragment nested inside it (gh #408): \
         {cluster:#}",
    );
    assert_structural_only_contract(cluster, "dart-type3");
    assert_no_pair_surface_on_cluster(cluster, "dart-type3");
    let occurrence_count = occurrences(cluster).len();
    assert!(
        occurrence_count >= 2,
        "a clone cluster must have at least two occurrences, got {occurrence_count}",
    );
    Ok(())
}

// Zero-false-positive guard: two structurally unrelated Dart functions
// (`tally()` map-building loop vs `describe()` if-cascade) must never
// share a cluster. Every cluster's occurrences must come from a single
// file — a human reading the report must not be told they are duplicates.
#[test]
fn dissimilar_dart_functions_never_form_a_cross_file_cluster() -> Result<()> {
    let report = run_cli("dart-dissimilar-functions", 8)?;
    for cluster in clusters(&report) {
        let files = cluster_file_set(cluster);
        assert!(
            files.len() <= 1,
            "dissimilar Dart functions must not cluster across files; got files {files:?}",
        );
    }
    Ok(())
}

// [PIPELINE-DETERMINISM] Two CLI runs over the same Dart corpus must
// produce identical cluster ids, spans and occurrence counts — proves
// the fingerprint and render paths are deterministic across process
// restarts. The `token_jaccard` bit-pin is gone with the signals
// surface; cluster identity (id + occurrence spans) is what consumers
// actually hold across sessions ([PIPELINE-DETERMINISM]).
#[test]
fn dart_report_is_deterministic_across_runs() -> Result<()> {
    let run1 = run_cli("dart-small", 10)?;
    let run2 = run_cli("dart-small", 10)?;
    let jaccards1: Vec<(String, u64, u64)> = clusters(&run1)
        .iter()
        .map(|cluster| {
            (
                cluster_id(cluster).to_owned(),
                cluster_size(cluster),
                field(cluster, "mass").as_u64().unwrap_or(0),
            )
        })
        .collect();
    let jaccards2: Vec<(String, u64, u64)> = clusters(&run2)
        .iter()
        .map(|cluster| {
            (
                cluster_id(cluster).to_owned(),
                cluster_size(cluster),
                field(cluster, "mass").as_u64().unwrap_or(0),
            )
        })
        .collect();
    assert!(
        !jaccards1.is_empty(),
        "dart-small must produce at least one cluster",
    );
    assert_eq!(
        jaccards1, jaccards2,
        "cluster identity (id, occurrence count, mass) must be identical across runs \
         on the same Dart corpus",
    );
    Ok(())
}

/// True when any cluster in the report spans both named files. Hidden
/// clusters are dropped before serialisation, so every cluster here is
/// one a human is actually shown.
fn any_cluster_spans(report: &serde_json::Value, left: &str, right: &str) -> bool {
    cluster_spanning(report, &[left, right]).is_some()
}

// [EXCLUSION-CONFIG] #95 — Dart code generators (`*.g.dart`,
// `*.freezed.dart`, …) emit near-identical serialisation blocks that
// self-duplicate across every annotated type. They must be hidden from the
// ranked report (still analysed, never surfaced). The suppression must stay
// targeted: `dart-generated-files` pairs two identical `.g.dart` files with
// two identical hand-written parsers — only the generated pair may vanish.
#[test]
fn dart_generated_files_are_hidden_but_handwritten_clones_surface() -> Result<()> {
    let report = run_cli("dart-generated-files", 10)?;
    assert!(
        !any_cluster_spans(&report, "serializers.g.dart", "models.g.dart"),
        "generated `.g.dart` files must not surface as a ranked duplicate cluster",
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the generated-file cluster must be actively hidden, not merely absent",
    );
    assert!(
        any_cluster_spans(&report, "parser_alpha.dart", "parser_beta.dart"),
        "hand-written duplicates must still surface even when generated files are hidden",
    );
    Ok(())
}

// [PIPELINE-BOILERPLATE-FILTER] #96 / #150 / #155 — Dart `export`/`import`
// barrel files are top-level scaffolding, never duplicate logic. Two
// export-only barrels share the identical directive shape but must never be
// reported as a clone of one another.
#[test]
fn dart_export_barrels_are_not_flagged_as_duplicates() -> Result<()> {
    let report = run_cli("dart-export-barrel", 8)?;
    assert!(
        !any_cluster_spans(&report, "widgets.dart", "models.dart"),
        "Dart export barrels are import scaffolding and must not cluster as duplicates",
    );
    Ok(())
}

// [CLONE-NOISE-SIGNATURE-ONLY] #154 — after identifier/literal
// normalisation two functions with the same parameter shape collapse to the
// same signature even when their bodies are unrelated. Such a signature-only
// match (bodies differ in raw bytes) is a false positive and must be
// suppressed. `dart-signature-only` shares the header
// `int computeScore(Map<String, int>, List<String>)` across two functions
// with entirely different bodies.
#[test]
fn dart_signature_only_match_with_differing_bodies_is_suppressed() -> Result<()> {
    let report = run_cli("dart-signature-only", 8)?;
    assert!(
        !any_cluster_spans(&report, "alpha.dart", "beta.dart"),
        "a signature-only structural match with differing bodies must be suppressed for Dart",
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the signature-only match must be actively suppressed (#154), not merely absent",
    );
    Ok(())
}

// [EXCLUSION-CONFIG] #95 — generators that emit no stable file suffix
// (ffigen/jnigen name FFI output `*_bindings.dart`) are still recognised by
// the machine-generated banner in the file head and hidden. Both fixture
// files are byte-identical generated code carrying `AUTO GENERATED FILE,
// DO NOT EDIT.` but no `.g.dart` suffix, so only the header check can hide
// them — exactly the dart-lang/http FFI-binding case from the real-repo sweep.
#[test]
fn dart_generated_header_files_are_hidden_without_a_suffix() -> Result<()> {
    let report = run_cli("dart-generated-header", 8)?;
    assert!(
        !any_cluster_spans(
            &report,
            "native_alpha_bindings.dart",
            "native_beta_bindings.dart"
        ),
        "banner-marked generated files (no `.g.dart` suffix) must still be hidden",
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the banner-marked generated clone must be actively hidden, not merely absent",
    );
    Ok(())
}

// #69 — the same method name implemented across several concrete
// subclasses of one abstract `Shape` clusters on its identical
// signature/shape after Type-2 normalisation, but each override has a
// genuinely different body. That is the polymorphic-implementation
// pattern, not extractable duplication. The three `measure` bodies share
// AST *shape* (only identifiers/literals differ), so the signature-only
// filter (#154) deliberately does NOT fire here — only the polymorphic
// filter (#69) can suppress this, and only once `enclosing_function_name`
// resolves Dart's nested `signature → function_signature → name`. A
// genuinely copy-pasted top-level `loadSettings` (byte-identical bodies)
// must still surface, proving the suppression is targeted.
#[test]
fn dart_polymorphic_override_signatures_are_suppressed() -> Result<()> {
    let report = run_cli("dart-issue-69-polymorphic", 8)?;
    let measure_pairs = [
        ("circle.dart", "square.dart"),
        ("circle.dart", "triangle.dart"),
        ("square.dart", "triangle.dart"),
    ];
    for (left, right) in measure_pairs {
        assert!(
            !any_cluster_spans(&report, left, right),
            "polymorphic `measure` overrides ({left} / {right}) must not surface as \
             duplication (#69)",
        );
    }
    assert!(
        clusters_hidden(&report) >= 1,
        "the polymorphic-signature cluster must be actively hidden (#69), not merely absent",
    );
    assert!(
        any_cluster_spans(&report, "repo_alpha.dart", "repo_beta.dart"),
        "a genuine byte-identical clone must still surface even when polymorphic \
         overrides are suppressed",
    );
    Ok(())
}

// #70 — `recordEvent("user_login", {...}, "evt-001")` and its sibling
// differ only in their string-literal arguments. The call-shape clusters
// across files after normalisation, but varying test data is not
// duplication. The enclosing functions are differently named (so #69
// cannot fire) and the matched range is a call body, not a signature (so
// #154 cannot fire), so only the literal-variation call filter — which
// re-parses via the now-wired Dart grammar and reads the `call_expression`
// `function`/`arguments` fields — can suppress it. A byte-identical
// `summarize` clone must still surface.
#[test]
fn dart_literal_variation_calls_are_suppressed() -> Result<()> {
    let report = run_cli("dart-issue-70-test-data", 8)?;
    assert!(
        !any_cluster_spans(&report, "events_alpha.dart", "events_beta.dart"),
        "calls varying only in string-literal arguments must not surface (#70)",
    );
    // The mass-only wire decides the family at admission: its members
    // carry near-zero authored-content agreement (every literal differs),
    // so the content gate rejects them below the floor
    // ([FUSED-CONTENT-GATE]) and the literal-variation filter never sees
    // them. The liveness proof is the byte-identical control below: the
    // same run admits and publishes it, so the family's absence is an
    // admission decision, never a scan that stopped looking.
    assert!(
        field(&report, "files_analysed")
            .as_u64()
            .is_some_and(|files| files >= 2),
        "the event files must be parsed: {report:#}"
    );
    assert!(
        any_cluster_spans(&report, "summary_alpha.dart", "summary_beta.dart"),
        "a genuine byte-identical clone must still surface alongside the suppressed \
         literal-variation cluster",
    );
    Ok(())
}
