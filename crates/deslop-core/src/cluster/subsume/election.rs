//! Survivor election for [PIPELINE-CLUSTER-SUBSUME].
//!
//! Two views cover the same physical bytes; exactly one may reach the
//! report. [`super::collapse_cross_cluster_overlap`] decides *whether*
//! they are one duplication, and this module decides *which* view is
//! it: file coverage, then measured content credibility, then physical
//! enclosure, then precision. Split out because the two questions have
//! independent failure modes, and every rule here is a scar — each
//! carries the fixture that would break without it.

use crate::{
    buckets::{is_demoted_tier, measured_kind, spans_multiple_files, ClusterKind},
    report::ReportSignals,
};

use super::super::Cluster;
use super::{all_occurrences_paired, covers_every_file, Nesting};

/// Which of two clusters covering one region reaches the report.
pub(super) enum Preference {
    /// The first (proposed) view survives; the other re-describes it.
    First,
    /// The second view overturns the nomination.
    Second,
    /// Neither subsumes the other — both are published.
    Neither,
}

/// Returns `true` when the two clusters are views of one duplicated
/// region: **every** occurrence of each is paired by containment with an
/// occurrence of the other.
///
/// Both directions, not either. One direction alone is satisfied by a
/// wide cluster that merely happens to contain one occurrence of a much
/// larger, differently-scoped cluster: a duplicated pair of generated
/// functions each contain a copy of a one-line statement clone that also
/// appears in a hand-written file, and the one-directional test calls
/// those one duplication. They are two — the statement family names a
/// file the function pair never mentions — and collapsing them replaces
/// "these two functions are identical" with a list of one-line
/// fragments.
pub(super) fn covers_same_region(outer: &Cluster, inner: &Cluster) -> bool {
    all_occurrences_paired(&inner.members, &outer.members)
        && all_occurrences_paired(&outer.members, &inner.members)
}

/// Chooses between two views of one region. `proposed` is the view the
/// caller nominated — the enclosing one where nesting decides, the
/// heavier one where neither set nests.
///
/// File coverage decides first, and it is not a tie-break — it is a
/// false-negative guard. Dropping a view that names a file the survivor
/// does not name erases that file's duplication from the report
/// entirely; no other cluster reports it. So a view survives whenever it
/// is the only one naming some file, however imprecise it is and however
/// deeply it nests, and when each view names a file the other does not,
/// **both** are published. The same guard preserves a `cs + rs + py`
/// view against a `cs`-only rival.
///
/// Only between two views over one file set does precision decide.
pub(super) fn preferred_view(proposed: &Cluster, other: &Cluster, nesting: Nesting) -> Preference {
    match (
        covers_every_file(&proposed.members, &other.members),
        covers_every_file(&other.members, &proposed.members),
    ) {
        (false, false) => Preference::Neither,
        (false, true) => Preference::Second,
        (true, false) => Preference::First,
        (true, true) => precision_preference(proposed, other, nesting),
    }
}

/// Between two views of one region over one file set, measured content
/// credibility decides first (#367, #408): a view the report will
/// demote or hide — `structural_only` or `loosely_similar` under
/// [`measured_kind`] — never deletes a view the report will publish as
/// actionable. The deleted view is the only place the reader would ever
/// see the duplication; replacing a credible whole-method Type-3 clone
/// with a saturated 13-node fragment nested inside it erased the only
/// actionable finding in five languages.
///
/// The reverse arm carries a byte-proof bar: a demoted view yields only
/// to *verbatim-proven* duplication
/// ([`crate::content::ContentEvidence::verbatim_dominated`]) that
/// crosses files. Any
/// sub-window of a demoted surface measures higher agreement than the
/// surface itself — the divergent positions are what the window
/// excludes — so without the bar, the #197 in-file sibling-method
/// family resurfaced as a credible six-line window family the moment
/// its demoted umbrella died (`dart_issue_197_single_file_structural_only`).
/// Narrowing a demoted surface must not launder it into a finding;
/// byte-equal copies crossing files are a copy event the umbrella's
/// in-file judgment does not speak to, and still overturn the umbrella
/// that would bury them. A verbatim family confined to one file is that
/// in-file judgment's own subject — sibling scaffolding repeating a
/// mandatory line — and four byte-equal `assert` statements once
/// deleted their demoted umbrella and surfaced as an act-now finding
/// (`python-issue-71-rest-endpoint-shape`,
/// `rest_endpoint_family_with_fstring_paths_is_suppressed`).
/// Where the tiers do
/// not distinguish the views, the structurally more precise view wins
/// as before ([`structural_precision`]) — so between two credible
/// views, physical enclosure stands. Two sharper within-credible
/// comparisons were built, measured, and removed: raw support (1.0
/// against 0.89 shattered the `csharp-fact-cross-cluster` method pair
/// into fragments) and an act-now grade over measured support (0.85
/// elected a verbatim core over the credible 0.8 window enclosing it
/// and orphaned that window's other absorbed views —
/// `issue_343_sum_clamp_saturation` counted the orphan). Content
/// overturns enclosure only across the demoted/credible boundary,
/// never inside it.
fn precision_preference(proposed: &Cluster, other: &Cluster, nesting: Nesting) -> Preference {
    match (demoted(proposed), demoted(other)) {
        (false, true) => Preference::First,
        (true, false)
            if other.content.verbatim_dominated && spans_multiple_files(&other.members) =>
        {
            Preference::Second
        }
        // Within one credibility tier the enclosing view is normally
        // the duplication and the nested view re-describes it, so
        // enclosure decides and the signal grades are not compared at
        // all. They are not comparable: `structural` is a measured
        // overlap ([FUSION-SHARED-SUBTREE]) and a nested window scores
        // higher exactly to the extent that it excludes what differs. A
        // byte-identical 28-byte parameter list scored 1.00 against the
        // enclosing method's 0.88 and deleted the only whole-method
        // Type-3 clone in `ts-type3-stmt`, emptying the report (#408) —
        // the same shape as the two comparisons this function's history
        // already removed for shattering method pairs into fragments.
        _ if nesting == Nesting::ProposedEncloses
            && !nested_view_is_the_duplication(other, proposed) =>
        {
            Preference::First
        }
        _ => structural_precision(proposed, other),
    }
}

/// The exception to "the encloser wins": a nested view that is
/// **byte-proven across files** and accounts for most of the enclosing
/// view's node mass is not re-describing a fragment of the encloser —
/// it *is* the duplication, and the encloser is that duplication plus
/// surrounding code that is not duplicated at all.
///
/// Both halves are load-bearing, and each is pinned by a fixture the
/// other breaks:
///
/// - Without the **byte proof**, an unproven wider window would delete
///   a verbatim clone.
/// - Without the **coverage share**, any byte-identical sliver would do
///   it: a 28-byte parameter list, 10% of the method enclosing it,
///   deleted the whole-method Type-3 clone in `ts-type3-stmt` (#408).
///
/// Measured on the two fixtures that bracket the rule, by the share of
/// the enclosing view's bytes the proven nested view accounts for:
///
/// | fixture | encloser | nested | share | correct survivor |
/// |---|---|---|---|---|
/// | `incremental-multilang` C# | class, `structural` 0.85 | `ReconcileEntries`, byte-identical | **0.82** | nested |
/// | `javascript-type3` | function, `structural` 0.87 | `for` body, byte-identical | **0.49** | encloser |
/// | `ts-type3-stmt` | method, `structural` 0.88 | parameter list, byte-identical | **0.10** | encloser |
///
/// The C# encloser is a *container*: a class holding one duplicated
/// method plus members that differ (a `const` in one file, a `record`
/// in the other). Electing it relabels a byte-proven Type-1 clone as a
/// Type-3 near-miss and counts the non-duplicated scaffolding as
/// duplicated. The JavaScript encloser is a *duplicate*: the whole
/// function is copied bar one trailing statement, so the `for` body
/// nested inside it re-describes half of a finding that is real at full
/// extent.
///
/// [`SHARE_NUMERATOR`]`/`[`SHARE_DENOMINATOR`] sits between 0.49 and
/// 0.82 with margin on both sides. Byte span rather than node count
/// because node mass compresses the difference (0.63 against 0.76 on
/// the same two fixtures) to where no threshold separates them safely.
fn nested_view_is_the_duplication(nested: &Cluster, enclosing: &Cluster) -> bool {
    nested.content.verbatim_dominated
        && spans_multiple_files(&nested.members)
        && byte_span(nested).saturating_mul(SHARE_DENOMINATOR)
            >= byte_span(enclosing).saturating_mul(SHARE_NUMERATOR)
}

/// Numerator of the share of an enclosing view a proven nested view
/// must cover to be elected over it.
const SHARE_NUMERATOR: usize = 2;
/// Denominator of that share. Two thirds — see
/// [`nested_view_is_the_duplication`] for the measurements it separates.
const SHARE_DENOMINATOR: usize = 3;

/// Total source bytes a view claims, summed over its occurrences.
fn byte_span(cluster: &Cluster) -> usize {
    cluster
        .members
        .iter()
        .map(|member| member.byte_range.len())
        .fold(0, usize::saturating_add)
}

/// True when the report will demote or hide this view — the
/// content-credibility test [`precision_preference`] ranks tiers by.
/// [`measured_kind`] routes the same measured evidence the renderer
/// routes, minus the byte-equivalence proof, which never moves a view
/// across the demoted/credible boundary (a byte-equivalent cluster's
/// leaves agree, so its content evidence already vouches for it).
pub(super) fn demoted(cluster: &Cluster) -> bool {
    let signals: ReportSignals = cluster.signals.into();
    is_demoted_tier(measured_kind(signals, cluster.content, &cluster.members))
}

/// The pre-content tie-break for two views that do **not** nest: the
/// structurally more precise view wins. An embedding-dominant
/// nomination stands even against a more precise rival: it carries
/// semantic evidence over the same bytes that a structural view cannot
/// express.
///
/// Reached only when neither occurrence set strictly encloses the
/// other — [`evaluate_pair`] nominates the enclosing view first, in
/// both directions. That ordering matters now that `structural` is a
/// graded overlap ([FUSION-SHARED-SUBTREE]) rather than binary Merkle
/// equality: comparing the grade across two views of *different scope*
/// systematically favours the narrower one, because a window scores
/// higher exactly to the extent that it excludes what differs. Between
/// nesting views the comparison is not meaningful and enclosure
/// decides; between non-nesting views the two grades describe
/// comparable spans and the more precise one is the better view.
fn structural_precision(proposed: &Cluster, other: &Cluster) -> Preference {
    if other.signals.structural > proposed.signals.structural && !is_embedding_dominant(proposed) {
        Preference::Second
    } else {
        Preference::First
    }
}

/// Returns true for a view whose measured verdict is the semantic
/// bucket — [`ClusterKind::SameBehavior`] under [`measured_kind`], the
/// same vocabulary [`demoted`] reads, so the elected view and the
/// rendered label cannot drift. The previous predicate was a private
/// copy (`structural < 0.10 && embedding_cos >= 0.90`) whose floor sat
/// above both the ANN admission gate and the renderer's
/// `same_behavior` route: an honestly-scored Type-4 function pair at
/// cosine 0.88 *rendered* as a semantic clone yet lost its nomination
/// to the byte-identical block nested inside it, deleting the only
/// declaration-level semantic finding
/// (`dart_issue_119_embedding_role_mismatch`).
fn is_embedding_dominant(cluster: &Cluster) -> bool {
    let signals: ReportSignals = cluster.signals.into();
    measured_kind(signals, cluster.content, &cluster.members) == ClusterKind::SameBehavior
}
