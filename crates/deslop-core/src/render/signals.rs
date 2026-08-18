//! One rendering of the fused signal triple and the content evidence
//! behind it, shared by every surface ([FUSION-CONTENT-GATE], #344).
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

/// The three deterministic axes plus the fused confidence, in report
/// order.
fn confidence_pairs(signals: ReportSignals) -> [(&'static str, f64); 4] {
    [
        ("structural", signals.structural),
        ("jaccard", signals.token_jaccard),
        ("embedding", signals.embedding_cos),
        ("fused", signals.fused),
    ]
}

/// The measured content evidence the gate scored the shape against.
fn evidence_pairs(signals: ReportSignals) -> [(&'static str, f64); 3] {
    [
        ("agreement", signals.agreement),
        ("rename", signals.rename_consistency),
        ("literal", signals.literal_fraction),
    ]
}

/// The three deterministic axes plus the fused confidence, as
/// `name=value` pairs.
#[must_use]
pub fn confidence_summary(signals: ReportSignals) -> String {
    render_pairs(&confidence_pairs(signals), &MARKDOWN_STYLE)
}

/// The measured content evidence the gate scored the shape against.
#[must_use]
pub fn evidence_summary(signals: ReportSignals) -> String {
    render_pairs(&evidence_pairs(signals), &MARKDOWN_STYLE)
}

/// The whole confidence explanation — the fused triple *and* the
/// measured content evidence — on one markup-free line.
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
/// ([`crate::buckets::lacks_content_support`], [FUSION-CONTENT-GATE]).
///
/// Worded once, here, because more than one surface refuses on this
/// evidence and a user comparing an LSP refusal against the report must
/// see the same numbers said the same way. The trailing explanation is
/// [`plain_explanation`], so the refusal carries the whole confidence
/// story rather than the two fields that convicted the cluster.
#[must_use]
pub fn unvouched_content_reason(signals: ReportSignals) -> String {
    format!(
        "the shapes match but the measured content does not vouch for them — \
         support {support:.2} is below the {floor:.2} content floor: {explanation}",
        support = crate::buckets::content_support(signals.agreement, signals.rename_consistency),
        floor = crate::buckets::CONTENT_SUPPORT_FLOOR,
        explanation = plain_explanation(signals),
    )
}

/// Column header for the fixed-width per-group signal table.
pub const TABLE_HEADER: &str =
    "group  structural  token_jaccard  embedding_cos  fused  agreement  rename  literal\n";

/// One fixed-width row of [`TABLE_HEADER`].
#[must_use]
pub fn table_row(id: &str, signals: ReportSignals) -> String {
    format!(
        "{id:<6}  {s:>10.2}  {j:>13.2}  {e:>13.2}  {f:>5.2}  {a:>9.2}  {r:>6.2}  {l:>7.2}",
        s = signals.structural,
        j = signals.token_jaccard,
        e = signals.embedding_cos,
        f = signals.fused,
        a = signals.agreement,
        r = signals.rename_consistency,
        l = signals.literal_fraction,
    )
}
