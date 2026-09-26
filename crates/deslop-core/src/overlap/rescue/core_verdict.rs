//! Canonical rescue core verdict and bounded trace ([FUSED-SHARED-SUBTREE-CORE]).

use std::hash::BuildHasher;

use super::RescueContext;
use crate::{
    content::{ContentMeasurer, PairScope},
    fingerprint::Fingerprint,
    overlap::{core::CoreVerdict, judge_core, OverlapMeasurer},
    pair::crosses_files,
};

/// The aligned core is the code the two endpoints provably share. A pair
/// lacking a copied core cannot be rescued by widening its wrapper.
pub(super) fn core_verdict<S: BuildHasher, L: BuildHasher>(
    left: &Fingerprint,
    right: &Fingerprint,
    context: &RescueContext<'_, S, L>,
    measurer: &mut OverlapMeasurer<'_>,
    contents: &mut ContentMeasurer,
) -> CoreVerdict {
    let core = measurer.aligned_core(left, right);
    let scope = core_scope(left, right, context);
    let verdict = judge_core(&core, context.core_floor, || {
        contents
            .pair(
                (left, right),
                &context.tree_index,
                context.sources,
                context.languages,
            )
            .core(&core, context.sources, scope)
    });
    log_core_verdict(left, right, &core, verdict);
    verdict
}

/// The same-file declaration context of the shared code.
fn core_scope<S: BuildHasher, L: BuildHasher>(
    left: &Fingerprint,
    right: &Fingerprint,
    context: &RescueContext<'_, S, L>,
) -> PairScope {
    PairScope {
        same_file: !crosses_files(left, right),
        interior: context.scopes.enclosing(left).is_some()
            && context.scopes.enclosing(right).is_some(),
        core: true,
    }
}

/// Records one core verdict so a surprising rescue or refusal is
/// traceable without source text ([PRINCIPLES-LOGGING]).
fn log_core_verdict(
    left: &Fingerprint,
    right: &Fingerprint,
    core: &[(Fingerprint, Fingerprint)],
    verdict: CoreVerdict,
) {
    if !tracing::enabled!(tracing::Level::TRACE) {
        return;
    }
    let evidence = verdict.evidence;
    let spans = format_spans(core);
    tracing::trace!(
        left_file = ?left.file_id,
        left_start = left.byte_range.start,
        left_end = left.byte_range.end,
        right_file = ?right.file_id,
        right_start = right.byte_range.start,
        right_end = right.byte_range.end,
        spans,
        core_nodes = verdict.nodes,
        measured = evidence.measured,
        contradiction = ?evidence.contradiction,
        agreement = evidence.agreement,
        rename = evidence.rename_consistency,
        consistent_rename = evidence.consistent_rename,
        copy = verdict.copy,
        "rescue core verdict",
    );
}

/// Byte offsets of proved shared spans, with no file text or paths.
fn format_spans(core: &[(Fingerprint, Fingerprint)]) -> String {
    core.iter()
        .map(|(span, partner)| {
            format!(
                "{}..{}={}..{}",
                span.byte_range.start,
                span.byte_range.end,
                partner.byte_range.start,
                partner.byte_range.end
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}
