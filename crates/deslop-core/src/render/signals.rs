//! One rendering of the elected pair's signal axes and the content
//! evidence behind it, shared by every surface ([FUSED-CONTENT-GATE],
//! #344).
//!
//! `structural`, `token_jaccard` and `embedding_cos` say *how much* the
//! members matched; they never say *why* a cluster routed where it did.
//! A corroborated Type-2 rename and an anchor-poor scaffolding family
//! render the identical triple — only the measured content evidence
//! separates them ([TECH-PMATCH-BAKER]). Until #344 that evidence existed
//! only in a `debug` log, so no reader and no black-box test could see
//! the gate's input.
//!
//! Every surface formats it here rather than restating the field list,
//! so the HTML footer, the Markdown report, the CLI text report and the
//! LSP diagnostics and code lenses can never drift into describing the
//! same numbers differently. Surfaces differ only in punctuation, which
//! is what [`PairStyle`] carries; the field list and the two-decimal
//! precision are defined once.
//!
//! There is no cluster-level `fused` to render ([FUSED-SCOPE]): `fused`
//! is the pair admission score, decided pair by pair at admission
//! ([FUSED-STRATEGY-BOUNDED-MAX]) and never carried on the report. The
//! report shows the elected pair's measured axes and its content
//! evidence, with the bucket as the engine's verdict.

use crate::report::ReportSignals;

/// How one surface punctuates a `label`/`value` pair and joins pairs.
struct PairStyle {
    /// Text between the label and the value.
    separator: &'static str,
    /// Text after the value — Markdown closes its code span here.
    terminator: &'static str,
    /// Text between consecutive pairs.
    joiner: &'static str,
}

/// Markdown surfaces put every value in a code span.
const MARKDOWN_STYLE: PairStyle = PairStyle {
    separator: "=`",
    terminator: "`",
    joiner: " ",
};

/// Surfaces the client renders verbatim — LSP diagnostic messages and
/// code lens titles — carry no markup, so a backtick would show up as a
/// literal character in the Problems panel and in the lens.
const PLAIN_STYLE: PairStyle = PairStyle {
    separator: " ",
    terminator: "",
    joiner: " · ",
};

/// Renders `pairs` to two decimal places in `style`.
fn render_pairs(pairs: &[(&str, f64)], style: &PairStyle) -> String {
    pairs
        .iter()
        .map(|(label, value)| {
            format!(
                "{label}{separator}{value:.2}{terminator}",
                separator = style.separator,
                terminator = style.terminator,
            )
        })
        .collect::<Vec<_>>()
        .join(style.joiner)
}

/// The three deterministic axes, in report order
/// ([FUSED-CLUSTER-SIGNALS]).
fn confidence_pairs(signals: ReportSignals) -> [(&'static str, f64); 3] {
    [
        ("structural", signals.structural),
        ("jaccard", signals.token_jaccard),
        ("embedding", signals.embedding_cos),
    ]
}

/// The measured content evidence the gate scored the shape against.
fn evidence_pairs(signals: ReportSignals) -> [(&'static str, f64); 3] {
    [
        ("agreement", signals.pair_agreement),
        ("rename", signals.pair_rename_consistency),
        ("literal", signals.literal_fraction),
    ]
}

/// The three deterministic axes, as `name=value` pairs.
#[must_use]
pub fn confidence_summary(signals: ReportSignals) -> String {
    render_pairs(&confidence_pairs(signals), &MARKDOWN_STYLE)
}

/// The measured content evidence the gate scored the shape against.
#[must_use]
pub fn evidence_summary(signals: ReportSignals) -> String {
    render_pairs(&evidence_pairs(signals), &MARKDOWN_STYLE)
}

/// The whole signal explanation — the elected pair's three axes *and*
/// the measured content evidence — on one markup-free line.
///
/// For surfaces the LSP client renders verbatim: the diagnostic message
/// ([LSP-DIAGNOSTICS]) and the code lens title ([LSP-CODE-LENS]). Both
/// are single-line plain text, so they take the whole explanation at
/// once rather than the two Markdown halves.
#[must_use]
pub fn plain_explanation(signals: ReportSignals) -> String {
    let confidence = confidence_pairs(signals);
    let evidence = evidence_pairs(signals);
    let pairs: Vec<(&str, f64)> = confidence.iter().chain(evidence.iter()).copied().collect();
    render_pairs(&pairs, &PLAIN_STYLE)
}

/// Why a decision surface refused to act on a cluster whose shape
/// saturates but whose measured content evidence does not vouch for it
/// ([`crate::buckets::lacks_content_support`], [FUSED-CONTENT-GATE]).
///
/// Worded once, here, because more than one surface refuses on this
/// evidence and a user comparing an LSP refusal against the report must
/// see the same numbers said the same way. The trailing explanation is
/// [`plain_explanation`], so the refusal carries the whole signal story
/// rather than the two fields that convicted the cluster.
#[must_use]
pub fn unvouched_content_reason(signals: ReportSignals) -> String {
    format!(
        "the shapes match but the measured content does not vouch for them — \
         support {support:.2} is below the {floor:.2} content floor: {explanation}",
        support = crate::buckets::content_support(signals.pair_agreement, signals.pair_rename_consistency),
        floor = crate::buckets::CONTENT_SUPPORT_FLOOR,
        explanation = plain_explanation(signals),
    )
}

/// Column header for the fixed-width per-group signal table.
pub const TABLE_HEADER: &str =
    "group  structural  token_jaccard  embedding_cos  agreement  rename  literal\n";

/// One fixed-width row of [`TABLE_HEADER`].
#[must_use]
pub fn table_row(id: &str, signals: ReportSignals) -> String {
    format!(
        "{id:<6}  {s:>10.2}  {j:>13.2}  {e:>13.2}  {a:>9.2}  {r:>6.2}  {l:>7.2}",
        s = signals.structural,
        j = signals.token_jaccard,
        e = signals.embedding_cos,
        a = signals.pair_agreement,
        r = signals.pair_rename_consistency,
        l = signals.literal_fraction,
    )
}

/// Half of the last digit a surface prints ([FUSED-CONTENT-GATE]).
/// Two values that render as the same string must never be described
/// as different, so every comparison the verdict makes is taken at the
/// precision the reader actually sees.
const RENDERED_EPSILON: f64 = 0.005;

/// One signal value at the two-decimal precision every surface renders.
fn format_signal(value: f64) -> String {
    format!("{value:.2}")
}

/// Plain-English reading of the elected pair's signal axes against the
/// measured content evidence ([FUSED-CONTENT-GATE-VERDICT], #344,
/// gh #460), for readers who have the numbers in front of them and no
/// way to tell a renamed copy from boilerplate.
///
/// Grounded only in the figures a surface already renders; it explains
/// the gap between the shape match and the content evidence, and never
/// re-derives the engine's bucket ([CLONE-BUCKETS-ROUTING] — a consumer
/// reads the engine's label and never manufactures one). Computed once,
/// here, and carried on the wire as `evidence_verdict`, so the VS Code
/// panel, the `JetBrains` panel and any future surface quote the same
/// sentence rather than each growing its own verdict engine.
///
/// **Which reading applies turns on whether the gate ran at all.**
/// [FUSED-CONTENT-GATE] is scoped to shape-saturating clusters:
/// `buckets::routing::route_shape_identical` returns before the gate
/// whenever [`crate::buckets::has_saturating_shape_evidence`] is false.
/// Below saturation the content evidence was measured but could not be
/// weighed — the two locations are not aligned position for position —
/// so [`unweighed_verdict`] names it as observation rather than support
/// (gh #460: `agreement = 0.31`, `rename_consistency = 0.00` — the
/// strongest available disproof of the match — must never be published
/// as corroboration of it).
///
/// The gate's own predicate decides, never a saturation test written a
/// second time here, so the sentence and the gate cannot disagree about
/// whether the gate ran. That predicate is a pure function of the
/// rendered triple, which is what lets the render path and the
/// `--from-report` replay path ([`crate::report_restamp`]) reach the
/// same reading from the same cluster.
///
/// **Pinned by**
/// `deslop::content_gate_signal_honesty::a_gate_skipped_cluster_is_not_told_its_content_evidence_agreed`,
/// with
/// `a_gated_cluster_still_reports_the_evidence_that_corroborated_it` as
/// the control that keeps the saturated population's verdict honest.
#[must_use]
pub fn content_evidence_verdict(signals: ReportSignals) -> String {
    let shape = signals.shape_score();
    if signals.embedding_cos > shape && signals.embedding_cos >= RENDERED_EPSILON {
        return semantic_verdict(signals, shape);
    }
    if !crate::buckets::has_saturating_shape_evidence(signals) {
        return unweighed_verdict(signals, shape);
    }
    if crate::buckets::content_support(signals.pair_agreement, signals.pair_rename_consistency)
        >= crate::buckets::CONTENT_SUPPORT_FLOOR
    {
        corroborated_verdict(signals, shape)
    } else {
        boilerplate_verdict(signals, shape)
    }
}

/// The shape never saturated, so [FUSED-CONTENT-GATE] was scoped out
/// and the evidence rests on the shape axes alone
/// ([FUSED-CONTENT-GATE-VERDICT], gh #460).
///
/// The measured figures are still shown — they are the only content
/// evidence there is, and withholding them would leave the reader a
/// shape they cannot interrogate — but named as unused, never dressed
/// up as support. Nor as disproof: below a saturated shape the two
/// locations are not aligned position for position, which is the
/// alignment both content populations assume, so a low reading here is
/// no more reliable against the match than a high one would be for it.
fn unweighed_verdict(signals: ReportSignals, shape: f64) -> String {
    format!(
        "The shapes match at {shape}, and that is the whole of this finding: the \
         content check runs only where the shape match saturates, so it did not run here and \
         nothing the code actually says was weighed against the shape. The content was still \
         measured, and these numbers went unused: the locations share {agreement} of their \
         content and consistent renaming explains {rename} of what differs. Read them as \
         observations — below an exact shape match the two locations are not lined up \
         position for position, so a low reading is no more proof against this match than a \
         high one would be for it.",
        shape = format_signal(shape),
        agreement = format_signal(signals.pair_agreement),
        rename = format_signal(signals.pair_rename_consistency),
    )
}

/// The embedding pass, not the shape, is what produced this finding.
fn semantic_verdict(signals: ReportSignals, shape: f64) -> String {
    format!(
        "The shapes barely match ({shape}) — this finding comes from the \
         embedding model, which read these as the same behavior written two ways. The \
         content evidence measures the code itself, not the behavior: shared content \
         {agreement}, renaming {rename}.",
        shape = format_signal(shape),
        agreement = format_signal(signals.pair_agreement),
        rename = format_signal(signals.pair_rename_consistency),
    )
}

/// The measured content evidence vouches for the shape match. Stated as
/// the measurement, never as a recommendation: the engine owns the
/// verdict.
fn corroborated_verdict(signals: ReportSignals, shape: f64) -> String {
    format!(
        "The shapes match at {shape} and the content evidence vouches for it: the \
         locations share {agreement} of their content and consistent renaming explains \
         {rename} of what differs, so the match clears the {floor:.2} content floor.",
        shape = format_signal(shape),
        agreement = format_signal(signals.pair_agreement),
        rename = format_signal(signals.pair_rename_consistency),
        floor = crate::buckets::CONTENT_SUPPORT_FLOOR,
    )
}

/// Below the content floor — the anchor-poor scaffolding family.
fn boilerplate_verdict(signals: ReportSignals, shape: f64) -> String {
    format!(
        "The shapes match at {shape} but the content behind them does not agree: the \
         locations share only {agreement} of their content and consistent renaming explains \
         {rename} of what differs, so support falls below the {floor:.2} content floor. A \
         matching shape over content that does not agree is what sibling boilerplate looks \
         like — read both locations before extracting anything.",
        shape = format_signal(shape),
        agreement = format_signal(signals.pair_agreement),
        rename = format_signal(signals.pair_rename_consistency),
        floor = crate::buckets::CONTENT_SUPPORT_FLOOR,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rendered signal triple with the content evidence behind it.
    fn signals(
        structural: f64,
        token_jaccard: f64,
        embedding_cos: f64,
        agreement: f64,
        rename_consistency: f64,
        literal_fraction: f64,
    ) -> ReportSignals {
        let mut built = ReportSignals {
            structural,
            token_jaccard,
            shape: 0.0,
            embedding_cos,
            pair_agreement: agreement,
            pair_rename_consistency: rename_consistency,
            literal_fraction,
        };
        built.shape = built.shape_score();
        built
    }

    /// [FUSED-CONTENT-GATE] The shape reading is the stronger of the two
    /// views of one normalised representation — the single reduction the
    /// wire `shape` field carries.
    #[test]
    fn shape_score_is_the_stronger_axis() {
        assert!((signals(1.0, 0.3, 0.0, 0.08, 0.0, 0.91).shape - 1.0).abs() < f64::EPSILON);
        assert!((signals(0.2, 0.3, 0.9, 0.05, 0.0, 0.0).shape - 0.3).abs() < f64::EPSILON);
    }

    /// [FUSED-CONTENT-GATE] The four readings a rendered cluster can
    /// carry, verbatim. Every surface quotes `evidence_verdict`, so this
    /// wording is the wording users and agents read: a corroborated
    /// rename and an anchor-poor scaffolding family render the identical
    /// triple, and only these sentences separate them.
    #[test]
    fn verdict_reads_each_family() {
        let scaffolding = signals(1.0, 1.0, 0.0, 0.08, 0.0, 0.91);
        assert_eq!(
            content_evidence_verdict(scaffolding),
            "The shapes match at 1.00 but the content behind them does not agree: the \
             locations share only 0.08 of their content and consistent renaming explains \
             0.00 of what differs, so support falls below the 0.70 content floor. A \
             matching shape over content that does not agree is what sibling boilerplate \
             looks like — read both locations before extracting anything."
        );

        let proven_rename = signals(1.0, 1.0, 0.0, 0.1, 1.0, 0.0);
        let rename_verdict = content_evidence_verdict(proven_rename);
        assert_eq!(
            rename_verdict,
            "The shapes match at 1.00 and the content evidence vouches for it: the \
             locations share 0.10 of their content and consistent renaming explains \
             1.00 of what differs, so the match clears the 0.70 content floor."
        );
        assert!(
            !rename_verdict.contains("boilerplate"),
            "a corroborated rename must never be described as boilerplate"
        );

        let verbatim = signals(1.0, 1.0, 0.0, 1.0, 1.0, 0.0);
        assert_eq!(
            content_evidence_verdict(verbatim),
            "The shapes match at 1.00 and the content evidence vouches for it: the \
             locations share 1.00 of their content and consistent renaming explains \
             1.00 of what differs, so the match clears the 0.70 content floor."
        );

        let semantic = signals(0.2, 0.3, 0.9, 0.05, 0.0, 0.0);
        assert_eq!(
            content_evidence_verdict(semantic),
            "The shapes barely match (0.30) — this finding comes from the \
             embedding model, which read these as the same behavior written two ways. The \
             content evidence measures the code itself, not the behavior: shared content \
             0.05, renaming 0.00."
        );

        // [FUSED-CONTENT-GATE-VERDICT] gh #460 — the accessor pair's
        // measured triple. Neither axis saturates, so the gate never ran
        // on it and its content evidence never entered the finding.
        let gate_skipped = signals(0.82, 0.73, 0.0, 0.31, 0.0, 0.0);
        let skipped_verdict = content_evidence_verdict(gate_skipped);
        assert_eq!(
            skipped_verdict,
            "The shapes match at 0.82, and that is the whole of this finding: the \
             content check runs only where the shape match saturates, so it did not run here \
             and nothing the code actually says was weighed against the shape. The content \
             was still measured, and these numbers went unused: the locations share 0.31 of \
             their content and consistent renaming explains 0.00 of what differs. Read them \
             as observations — below an exact shape match the two locations are not lined up \
             position for position, so a low reading is no more proof against this match \
             than a high one would be for it."
        );
        assert!(
            !skipped_verdict.contains("vouches for it"),
            "the gate never ran on this cluster, so its evidence cannot be said to have \
             vouched for anything: {skipped_verdict}"
        );
        assert_ne!(
            skipped_verdict,
            content_evidence_verdict(verbatim),
            "a cluster whose evidence was weighed and one whose evidence was skipped must \
             not read the same"
        );

        assert_ne!(
            content_evidence_verdict(scaffolding),
            content_evidence_verdict(proven_rename),
            "one shape reading must not produce one explanation"
        );
    }
}
