//! Parity false-positive-filter E2E tests for JavaScript / TypeScript
//! ([CLONE-NOISE-SIGNATURE-ONLY], [PIPELINE-BOILERPLATE-FILTER]).
//!
//! The cluster-level noise filters that protect C#, Rust, Python, and Dart
//! from shape-only false positives must protect JS/TS too. These pin two
//! cases: a signature-only match between unrelated function bodies is
//! suppressed (#154), and the re-export-barrel suppression never erases a
//! real `export default` value that merely normalises to a literal.

use anyhow::Result;

use crate::common::signals::has_verbatim_pair;
use crate::common::*;

#[test]
fn typescript_signature_only_match_with_divergent_bodies_is_suppressed() -> Result<()> {
    let report = run_report(&fixture("ts-signature-only-noise"), 6)?;
    // Both functions share the typed signature `(_: Context, _: Options):
    // Outcome` — which normalises identically — but their bodies are
    // unrelated. Without #154 this fuses to a top-ranked false positive;
    // with it, the signature-only family is detected and hidden.
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(2),
        "both signature-only files must be analysed: {report:#}"
    );
    assert!(
        clusters(&report).is_empty(),
        "an unrelated-body signature match must not surface as a clone: {report:#}"
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the signature-only family must be detected and hidden, proving #154 fired: {report:#}"
    );
    Ok(())
}

#[test]
fn typescript_export_default_template_logic_still_clusters() -> Result<()> {
    let report = run_report(&fixture("ts-export-default-logic"), 12)?;
    // Each file `export default`s a template literal that embeds duplicated
    // reduce/ternary/member-expression logic. After normalisation an
    // untagged default export reduces to `export_statement -> __literal__`,
    // the same shape as an `export * from` barrel — so the barrel filter
    // must NOT suppress it. The duplicated embedded logic still clusters.
    let clone = expect_cluster_spanning(&report, &["reportA.ts", "reportB.ts"])?;
    assert!(
        !has_verbatim_pair(&fixture("ts-export-default-logic"), clone)?,
        "the duplicated template-literal logic must still be detected: {report:#}"
    );
    Ok(())
}

#[test]
fn typescript_reexport_barrels_are_suppressed_but_real_exports_survive() -> Result<()> {
    let report = run_report(&fixture("ts-reexport-barrel"), 3)?;
    // `export { X } from "./m"` barrel files are pure module scaffolding — the
    // JS/TS analogue of Dart's `library_export` — so they must not cluster.
    assert!(
        cluster_spanning(&report, &["index_a.ts", "index_b.ts"]).is_none(),
        "re-export barrels must be suppressed, not reported as a clone: {report:#}"
    );
    // A real function exported just below a barrel still surfaces, proving the
    // suppression is scoped to the barrel and never swallows real logic.
    let real = expect_cluster_spanning(&report, &["real_a.ts", "real_b.ts"])?;
    assert!(
        !has_verbatim_pair(&fixture("ts-reexport-barrel"), real)?,
        "the real export pair must be admitted (a near-miss, byte-distinct): \
         {real:#}"
    );
    Ok(())
}
