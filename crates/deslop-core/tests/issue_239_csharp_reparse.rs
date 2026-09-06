//! Regression coverage for GH #239: the `VSCode` panel hung on
//! "Analysing…" forever because `render_report`'s C#-only
//! byte-equivalence fallback re-parsed the *entire source file* once per
//! cluster member, bypassing the shared per-render parse cache
//! ([CLONE-NOISE-REPARSE-CACHE]). On a large C# corpus (5k+ clusters)
//! that pinned the LSP for minutes-to-hours before it could answer
//! `initialize`, so the editor spinner never resolved.
//!
//! The test renders the same two Type-2 C# files twice: once as a
//! two-member cluster (one method per file) and once as a cluster whose
//! members cover every method in both files. Parse work must scale with
//! the number of *files*, not the number of *members*, so the many-member
//! render must stay within a fixed multiple of the two-member render. The
//! bound is a ratio (with an absolute floor) rather than a wall-clock
//! budget so it self-calibrates to the host: the per-member re-parse
//! overshoots it by an order of magnitude while the cached path clears it
//! several times over on debug, release, and coverage builds alike.

mod common;

use std::{fmt::Write as _, time::Instant};

use anyhow::{anyhow, Result};
use common::ReportFixture;
use deslop_core::{ast::ByteRange, cluster::Cluster, pair::PairScore};

/// Methods generated per C# file; the stress cluster carries two files'
/// worth of members. High enough that a per-member full-file re-parse
/// dwarfs the two-parse baseline by an order of magnitude.
const METHODS_PER_FILE: usize = 192;

/// Render repetitions per measurement; the minimum is compared so a
/// one-off scheduler hiccup cannot flip the verdict.
const RENDER_RUNS: usize = 3;

/// Maximum allowed `stress / baseline` render-time ratio. The fixed
/// (parse-once-per-file) path stays under ~8x; the per-member re-parse
/// regression lands at ~`METHODS_PER_FILE`x.
const BUDGET_RATIO: u128 = 20;

/// Absolute floor for the stress budget so a sub-millisecond baseline on
/// a fast host cannot manufacture an impossible bound.
const BUDGET_FLOOR_MS: u128 = 250;

#[test]
fn csharp_equivalence_fallback_scales_with_files_not_members_issue_239() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut fixture = ReportFixture::new(tmp.path(), "csharp");
    let (alpha_source, alpha_ranges) = csharp_worker_source("Alpha", METHODS_PER_FILE)?;
    let (beta_source, beta_ranges) = csharp_worker_source("Beta", METHODS_PER_FILE)?;
    let alpha = fixture.file("AlphaWorker.cs", &alpha_source);
    let beta = fixture.file("BetaWorker.cs", &beta_source);

    let first_alpha_range = first_range(&alpha_ranges)?;
    let first_beta_range = first_range(&beta_ranges)?;
    let baseline = cluster_over(
        "issue-239-baseline",
        &[(alpha, first_alpha_range), (beta, first_beta_range)],
    );
    let stress_ranges: Vec<(deslop_core::state::FileId, ByteRange)> = alpha_ranges
        .iter()
        .map(|range| (alpha, *range))
        .chain(beta_ranges.iter().map(|range| (beta, *range)))
        .collect();
    let stress = cluster_over("issue-239-stress", &stress_ranges);

    // Sanity: the stress cluster must surface as a visible renamed
    // (Type-2) cluster with every member intact — the timing bound below
    // is only meaningful if the C# equivalence path actually ran on a
    // cluster the report keeps.
    let report = fixture.render(std::slice::from_ref(&stress));
    let rendered = report
        .clusters
        .iter()
        .find(|cluster| cluster.id == "issue-239-stress")
        .ok_or_else(|| anyhow!("stress cluster must stay visible in the ranked report"))?;
    assert_eq!(
        rendered.bucket, "nearly_identical",
        "renamed Type-2 members must not classify as byte-identical"
    );
    assert_eq!(
        rendered.occurrences.len(),
        METHODS_PER_FILE * 2,
        "every member must render as an occurrence"
    );

    let baseline_ms = min_render_millis(&fixture, std::slice::from_ref(&baseline));
    let stress_ms = min_render_millis(&fixture, std::slice::from_ref(&stress));
    let budget_ms = (baseline_ms * BUDGET_RATIO).max(BUDGET_FLOOR_MS);
    assert!(
        stress_ms <= budget_ms,
        "GH #239 regression: rendering one C# cluster with {members} members took \
         {stress_ms}ms against a {baseline_ms}ms two-member baseline (budget \
         {budget_ms}ms = max({BUDGET_RATIO}x baseline, {BUDGET_FLOOR_MS}ms floor)). \
         The byte-equivalence fallback is re-parsing the full file per member \
         instead of parsing each file once per render.",
        members = METHODS_PER_FILE * 2,
    );
    Ok(())
}

/// Builds one synthetic C# class of `methods` near-identical methods and
/// returns the source plus each method's byte range. Identifiers embed
/// `class_tag` and the method index, so counterpart methods across two
/// generated files share AST shape but never share raw bytes — the
/// renamed-clone (Type-2) shape that forces `report_bucket_kind` through
/// the C# method-equivalence fallback for every member.
fn csharp_worker_source(class_tag: &str, methods: usize) -> Result<(String, Vec<ByteRange>)> {
    let mut source = format!(
        "using System;\n\nnamespace Deslop.Issue239\n{{\n    public class {class_tag}Worker\n    {{\n"
    );
    let mut ranges = Vec::with_capacity(methods);
    for index in 0..methods {
        let start = source.len();
        write!(
            source,
            "        public int Compute{class_tag}{index}(int seed{class_tag})\n\
             {{\n\
             var total{class_tag} = seed{class_tag} + {index};\n\
             for (var step{class_tag} = 0; step{class_tag} < seed{class_tag}; step{class_tag}++)\n\
             {{\n\
             if (step{class_tag} % 3 == 0)\n\
             {{\n\
             total{class_tag} += step{class_tag} * {index};\n\
             }}\n\
             else\n\
             {{\n\
             total{class_tag} -= step{class_tag};\n\
             }}\n\
             }}\n\
             return total{class_tag};\n\
             }}\n"
        )?;
        ranges.push(ByteRange {
            start,
            end: source.len(),
        });
    }
    source.push_str("    }\n}\n");
    Ok((source, ranges))
}

/// Returns the first generated method range, failing loudly when the
/// generator produced none.
fn first_range(ranges: &[ByteRange]) -> Result<ByteRange> {
    ranges
        .first()
        .copied()
        .ok_or_else(|| anyhow!("generator must produce at least one method range"))
}

/// Fabricates one exact-structural cluster over `members` byte ranges.
/// `token_jaccard` sits below the proven-identical floor so
/// classification routes through the byte-equivalence check under test.
fn cluster_over(id: &str, members: &[(deslop_core::state::FileId, ByteRange)]) -> Cluster {
    Cluster {
        id: id.to_owned(),
        members: members
            .iter()
            .enumerate()
            .map(|(index, (file_id, range))| {
                ReportFixture::fingerprint_at(*file_id, *range, 120, index)
            })
            .collect(),
        weight: 10_000.0,
        signals: PairScore {
            structural: 1.0,
            token_jaccard: 0.97,
            embedding_cos: 0.0,
        },
        content: deslop_core::content::ContentEvidence::unmeasured(),
    }
}

/// Renders `clusters` `RENDER_RUNS` times and returns the fastest run in
/// milliseconds, asserting each render actually produced the cluster.
fn min_render_millis(fixture: &ReportFixture, clusters: &[Cluster]) -> u128 {
    (0..RENDER_RUNS)
        .map(|_| {
            let started = Instant::now();
            let report = fixture.render(clusters);
            assert_eq!(
                report.clusters.len(),
                1,
                "timed render must keep its cluster visible"
            );
            started.elapsed().as_millis()
        })
        .min()
        .unwrap_or(u128::MAX)
}
