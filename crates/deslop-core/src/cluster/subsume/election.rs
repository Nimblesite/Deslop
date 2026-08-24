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
/// The reverse arm carries a byte-proof bar with a mass floor: a
/// demoted view yields only to *verbatim-proven* duplication
/// ([`crate::content::ContentEvidence::verbatim_dominated`]) whose
/// every occurrence carries real statement mass
/// ([`has_overturn_mass`]). Any sub-window of a demoted surface
/// measures higher agreement than the surface itself — the divergent
/// positions are what the window excludes — so an unconditional
/// "credible beats demoted" let the #197 in-file sibling-method family
/// resurface as a credible six-line window family the moment its
/// demoted umbrella died (`dart_issue_197_single_file_structural_only`).
/// The byte proof alone is not enough either, in either direction of
/// file spread: in one file, four byte-equal `assert` statements once
/// deleted their umbrella and surfaced as an act-now finding
/// (`python-issue-71-rest-endpoint-shape`,
/// `rest_endpoint_family_with_fstring_paths_is_suppressed`); across
/// files, one byte-equal mandatory kwargs line deleted the umbrella
/// over two *different* model constructors, and the fifteen absorbed
/// micro-views it released were published as four visible clusters no
/// noise filter could recognise (`python-issue-100-kwargs-ctor`,
/// `message_vs_agentlog_kwargs_constructors_do_not_cluster`).
///
/// A byte-proven view with real statement mass is different in kind
/// from those idiom lines: it is a copied block, the demoted encloser
/// is that block plus a remainder whose content evidence failed, and a
/// judgment that failed on its own content cannot vouch for code that
/// is byte-provenly repeated. The verbatim 158-byte five-statement run
/// inside `csharp-merge-readafter`'s two methods was deleted by the
/// `structural` 0.85 straddle enclosing it — a Type-1 false negative
/// that also claimed nine duplicated lines where five are
/// (`byte_identical_clone_survives_a_demoted_enclosing_view_in_one_file`,
/// `declared_inside_read_after_refuses`) — and the byte-identical
/// `if` block shared by two otherwise-divergent functions was deleted
/// by the whole-file echo enclosing it
/// (`content_proven_nested_clone_survives_content_poor_enclosing_view`).
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
        (true, false) if other.content.verbatim_dominated && has_overturn_mass(other) => {
            Preference::Second
        }
        // The occurrence-count exception below was measured between two
        // demoted views: seven shape-only methods against the two class
        // containers enclosing them. It is not independent content proof
        // and must not jump the tier boundary. Otherwise a synthetic
        // statement window can exclude the one differing endpoint, certify
        // itself from the substitutions it retained, then use its seven
        // occurrences to delete the demoted view that still carries the
        // contradiction (`rename_needs_an_anchor`).
        (true, false) if nesting == Nesting::ProposedEncloses => Preference::First,
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
            && !nested_view_is_the_duplication(other, proposed)
            && !nested_view_outnumbers(other, proposed) =>
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
/// The `incremental-multilang` C# encloser [`accounts_for_bulk`]
/// separates is a *container*: a class holding one duplicated method
/// plus members that differ (a `const` in one file, a `record` in the
/// other). Electing it relabels a byte-proven Type-1 clone as a Type-3
/// near-miss and counts the non-duplicated scaffolding as duplicated.
/// The `javascript-type3` encloser is a *duplicate*: the whole function
/// is copied bar one trailing statement, so the `for` body nested
/// inside it re-describes half of a finding that is real at full
/// extent.
fn nested_view_is_the_duplication(nested: &Cluster, enclosing: &Cluster) -> bool {
    nested.content.verbatim_dominated
        && spans_multiple_files(&nested.members)
        && accounts_for_bulk(nested, enclosing)
}

/// True when `nested` claims at least
/// [`SHARE_NUMERATOR`]`/`[`SHARE_DENOMINATOR`] of `enclosing`'s bytes —
/// the boundary between "the enclosing view is this duplication plus
/// code that is not duplicated" and "the nested view re-describes a
/// fragment of a finding that is real at full extent". Measured, by the
/// share of the enclosing view's bytes the byte-proven nested view
/// accounts for:
///
/// | fixture | share | correct survivor |
/// |---|---|---|
/// | `incremental-multilang` C# class around a copied method | **0.82** | nested |
/// | `javascript-type3` function around a copied `for` body | **0.49** | encloser |
/// | `ts-type3-stmt` method around a copied parameter list | **0.10** | encloser |
///
/// Two thirds sits between 0.49 and 0.82 with margin on both sides.
/// Byte span rather than node count because node mass compresses the
/// bracketing measurements (0.63 against 0.76) to where no threshold
/// separates them safely.
fn accounts_for_bulk(nested: &Cluster, enclosing: &Cluster) -> bool {
    byte_span(nested).saturating_mul(SHARE_DENOMINATOR)
        >= byte_span(enclosing).saturating_mul(SHARE_NUMERATOR)
}

/// True when every occurrence of a byte-proven view carries at least
/// [`VERBATIM_OVERTURN_MIN_NODES`] normalised nodes — a copied
/// *block*, with the standing to overturn a demoted enclosing view,
/// rather than a mandatory idiom line repeating inside sibling
/// scaffolding. The minimum over occurrences, not the sum: a family of
/// many tiny repeats gains no standing from its cardinality.
///
/// Measured on the fixtures that bracket the demoted-encloser
/// exception, by `--min-nodes` bisection of the deciding view:
///
/// | fixture | view | nodes per occurrence | verdict |
/// |---|---|---|---|
/// | `csharp-merge-readafter` | five-statement verbatim run | ≥ 28 | overturns |
/// | `alpha`/`beta` shared-logic pair | byte-identical `if` block | 24–27 | overturns |
/// | `python-issue-71-rest-endpoint-shape` | byte-equal `assert` line | 8–9 | absorbed |
/// | `python-issue-100-kwargs-ctor` | byte-equal kwargs line | 8–9 | absorbed |
///
/// Sixteen sits between 9 and 24 with real margin on both sides, and
/// above the saturated 13-node fragment this module's history already
/// records as the canonical sliver (#408).
fn has_overturn_mass(cluster: &Cluster) -> bool {
    cluster
        .members
        .iter()
        .map(|member| member.node_count)
        .min()
        .is_some_and(|smallest| smallest >= VERBATIM_OVERTURN_MIN_NODES)
}

/// Node floor for [`has_overturn_mass`] — see its measurement table.
const VERBATIM_OVERTURN_MIN_NODES: usize = 16;

/// The second exception to "the encloser wins": a nested view with
/// strictly **more occurrences** than the view enclosing it is not
/// re-describing that view — it is a family the encloser cannot
/// express, and electing the encloser deletes the surplus findings
/// outright.
///
/// [`nested_view_is_the_duplication`] cannot cover this case: it asks
/// for a *byte proof*, and a shape-only family has none by
/// construction — normalisation strips the identifiers that differ
/// between siblings, which is the only reason they cluster at all.
///
/// The three fixtures bracketing the byte-proof rule all pit **two**
/// occurrences against **two** (`ts-type3-stmt` parameter list vs
/// method, `javascript-type3` `for` body vs function,
/// `incremental-multilang` method vs class), so this test cannot fire
/// on them and their survivors are unchanged. It fires where the counts
/// genuinely disagree: seven shape-identical Dart API methods across two
/// files, enclosed by a two-occurrence whole-class view measuring
/// `structural` 0.75 against the family's Merkle-saturated 1.00. Electing
/// the class dropped five of the seven occurrences and counted each
/// class's constructor, fields and imports as duplicated — reporting
/// `inventory_api.dart` 100% duplicated when only its method bodies
/// repeat (`rank_structural_only_policy`, [RANK-STRUCTURAL-ONLY]).
///
/// Counting occurrences is safe where comparing `structural` grades is
/// not: a nested window scores higher merely by excluding what differs,
/// but it cannot *invent* occurrences the enclosing view lacks.
fn nested_view_outnumbers(nested: &Cluster, enclosing: &Cluster) -> bool {
    nested.members.len() > enclosing.members.len()
}

/// Numerator of the share of an enclosing view a proven nested view
/// must cover to be elected over it.
const SHARE_NUMERATOR: usize = 2;
/// Denominator of that share. Two thirds — see [`accounts_for_bulk`]
/// for the measurements it separates.
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
