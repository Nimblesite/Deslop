//! [FUSED-SHARED-SUBTREE-CORE] One pair, one verdict, whichever surface
//! answers.
//!
//! A below-threshold cross-file pair reaches a report only through the
//! rescue, which judges the code the two endpoints share. `compare_pair`
//! answers the same question about the same two occurrences when a user
//! taps them in the editor. Both readings here come from the production
//! entry points — one real [`PipelineSession`] over real files, its own
//! report, and its own `compare_pair` — so neither reading can mirror a
//! suspected implementation, and a repair to either caller moves this
//! pin.

use std::{
    fs,
    path::{Path, PathBuf},
};

use tempfile::TempDir;

use crate::{
    ast::{ByteRange, NormalizedNode},
    embedding::EmbeddingMode,
    lang::{python::PythonParser, LanguageParser},
    pipeline::{EmbeddingSettings, PipelineSession},
    registry_fixtures::python_pair_ids,
    report::{PairComparison, PairComparisonParams, PairEndpoint, ReportCluster},
    state::FileId,
};

/// The scan floor these fixtures are sized for.
const MIN_NODES: u32 = 30;

/// The left-hand file name inside every scanned workspace.
const LEFT_FILE: &str = "settle.py";

/// The right-hand file name inside every scanned workspace.
const RIGHT_FILE: &str = "apply.py";

/// The normalised kind of a Python indented body — the interior window
/// these fixtures compare, carved from inside its function.
const BLOCK: &str = "block";

/// A posting routine carrying no literal at all, so a rename has nothing
/// but identifiers to anchor on.
const SETTLE_WITHOUT_LITERALS: &str = "\
def settle(ledger, invoice):
    total = ledger.opening
    total = total + invoice.amount
    total = total - invoice.credit
    total = total + invoice.freight
    ledger.record(invoice.id, total)
    return total
";

/// The same routine under a complete rename of every local and
/// collaborator, still without a literal.
const APPLY_WITHOUT_LITERALS: &str = "\
def apply(book, bill):
    running = book.opening
    running = running + bill.amount
    running = running - bill.credit
    running = running + bill.freight
    book.record(bill.id, running)
    return running
";

/// The same copy whose rename of the accumulator breaks at one position,
/// so no contradiction-free rename can vouch for the pair and the verdict
/// falls to the pooled support the two surfaces are asked for.
const APPLY_WITH_BROKEN_RENAME: &str = "\
def apply(book, bill):
    running = book.opening
    running = running + bill.amount
    running = running - bill.credit
    running = running + bill.freight
    book.record(bill.id, carried)
    return running
";

/// The same copy with one statement inserted, so the pair is the Type-3
/// near-miss the rescue exists for rather than a shape-equal pair.
const APPLY_WITH_INSERTION: &str = "\
def apply(book, bill):
    running = book.opening
    running = running + bill.amount
    running = running + bill.surcharge
    running = running - bill.credit
    running = running + bill.freight
    book.record(bill.id, running)
    return running
";

/// A routine whose shared code is a run of statements in its middle, with
/// a head of its own — so the occurrence the scan elects is a window
/// inside the function rather than the declaration.
const SETTLE_SHARED_RUN: &str = "\
def settle(ledger, invoice):
    ledger.begin(invoice)
    ledger.lock(invoice)
    ledger.stamp(invoice)
    total = ledger.opening
    total = total + invoice.amount
    total = total - invoice.credit
    total = total + invoice.freight
    total = total + invoice.rounding
    return total
";

/// The same run under a complete rename, behind a head built from
/// different statement kinds so the two declarations cannot match whole.
const APPLY_SHARED_RUN: &str = "\
def apply(book, bill):
    for entry in book.entries:
        book.replay(entry)
    while book.pending:
        book.drain()
    running = book.opening
    running = running + bill.amount
    running = running - bill.credit
    running = running + bill.freight
    running = running + bill.rounding
    return running
";

/// A routine that shares nothing with [`SETTLE_WITHOUT_LITERALS`] but the
/// language it is written in — the refused control.
const UNRELATED_ROUTINE: &str = "\
def describe(widget):
    if widget.width == widget.height:
        return widget.name
    for corner in widget.corners:
        widget.mark(corner)
    while widget.pending:
        widget.flush()
    return widget.name
";

/// [FUSED-SHARED-SUBTREE-CORE] The scan and the explicit comparison must
/// reach one verdict on one pair.
///
/// The scan admits a below-threshold cross-file pair only through the
/// rescue's judgement of the code the endpoints share; `compare_pair`
/// re-derives that judgement to title the comparison and to fold the
/// cluster's clone kind. Whichever way a pair goes, the two must go the
/// same way: a pair the report never grouped cannot be one the comparison
/// calls admitted, and a pair the report published cannot be one the
/// comparison rejects.
#[test]
fn the_scan_and_the_comparison_reach_one_verdict_on_one_pair() -> Result<(), String> {
    let readings = published_readings()?;
    let described: Vec<String> = readings
        .iter()
        .map(|(index, interior, comparison)| describe_published(*index, *interior, comparison))
        .collect();
    assert!(
        readings.iter().any(|(_, interior, _)| *interior),
        "reach guard: at least one published pair must be two windows strictly inside \
         authored functions, or nothing here exercises `PairScope::interior` and the \
         parity below proves nothing about it — {described:?}"
    );
    assert!(
        readings
            .iter()
            .all(|(_, _, comparison)| comparison.evidence.admitted),
        "one pair, one verdict: the report published each of these as a two-occurrence \
         cluster — the only pair it has, with no third member to weld it through — and \
         `compare_pair` refuses to admit it — {described:?}"
    );
    Ok(())
}

/// [FUSED-SHARED-SUBTREE-CORE] The controls that stop the parity pin
/// passing on two surfaces that both say nothing.
///
/// A verbatim renamed copy must be grouped by the scan *and* admitted by
/// the comparison; two routines sharing only their language must be
/// refused by both. Without these, a build that reported nothing at all
/// and admitted nothing at all would satisfy the parity assertion above.
#[test]
fn a_renamed_copy_is_admitted_and_an_unrelated_routine_is_refused() -> Result<(), String> {
    let copy = Scan::run(SETTLE_WITHOUT_LITERALS, APPLY_WITHOUT_LITERALS)?;
    let copy_evidence = copy.comparison(&copy.published_pair()?)?;
    assert!(
        copy.grouped_by_the_scan() && copy_evidence.evidence.admitted,
        "positive control: the scan must group a complete rename of one routine and \
         `compare_pair` must admit it — {}",
        describe(0, copy.grouped_by_the_scan(), &copy_evidence)
    );
    let stranger = Scan::run(SETTLE_WITHOUT_LITERALS, UNRELATED_ROUTINE)?;
    let stranger_evidence = stranger.comparison(&stranger.endpoints)?;
    assert!(
        !stranger.grouped_by_the_scan() && !stranger_evidence.evidence.admitted,
        "negative control: neither surface may claim two unrelated routines — {}",
        describe(1, stranger.grouped_by_the_scan(), &stranger_evidence)
    );
    Ok(())
}

/// The pairs the parity is held over: a literal-free complete rename, the
/// same rename one statement wider, and a rename that breaks at one
/// position so nothing but pooled support can carry the pair.
fn parity_cases() -> [(&'static str, &'static str); 4] {
    [
        (SETTLE_WITHOUT_LITERALS, APPLY_WITHOUT_LITERALS),
        (SETTLE_WITHOUT_LITERALS, APPLY_WITH_INSERTION),
        (SETTLE_WITHOUT_LITERALS, APPLY_WITH_BROKEN_RENAME),
        (SETTLE_SHARED_RUN, APPLY_SHARED_RUN),
    ]
}

/// Every parity case scanned, with the pair its report published, whether
/// that pair is the interior shape, and what `compare_pair` says about it.
fn published_readings() -> Result<Vec<(usize, bool, PairComparison)>, String> {
    parity_cases()
        .into_iter()
        .enumerate()
        .map(|(index, (left_source, right_source))| {
            let scan = Scan::run(left_source, right_source)?;
            let published = scan.published_pair()?;
            let interior = scan.published_pair_is_interior(&published)?;
            Ok((index, interior, scan.comparison(&published)?))
        })
        .collect()
}

/// One published pair's reading, with whether it is the interior shape
/// `PairScope::interior` is the only rule about.
fn describe_published(index: usize, interior: bool, comparison: &PairComparison) -> String {
    format!(
        "{} interior_windows={interior}",
        describe(index, true, comparison)
    )
}

/// One pair's two readings side by side, so a failure names which surface
/// saw what.
fn describe(index: usize, grouped: bool, comparison: &PairComparison) -> String {
    let evidence = &comparison.evidence;
    format!(
        "pair {index}: report grouped={grouped}; compare_pair admitted={} classification={:?} structural={:.4} token={:.4} agreement={:.4} rename={:.4} — {}",
        evidence.admitted,
        evidence.classification,
        evidence.structural,
        evidence.token_jaccard,
        evidence.agreement,
        evidence.rename_consistency,
        evidence.explanation,
    )
}

/// One workspace scanned by the production pipeline, holding the report
/// that run produced and the session the comparison is asked on.
struct Scan {
    /// Keeps the scanned files on disk for the session's lifetime.
    workspace: TempDir,
    /// The session `compare_pair` is asked on.
    session: PipelineSession,
    /// The report the same run produced.
    report: crate::report::Report,
    /// The two body windows the controls ask about directly.
    endpoints: PairComparisonParams,
    /// The left file's bytes, for locating authored declarations.
    left_source: String,
    /// The right file's bytes, for the same.
    right_source: String,
}

impl Scan {
    /// Writes both sources into a fresh workspace and runs the production
    /// pipeline over it, embeddings off.
    fn run(left_source: &str, right_source: &str) -> Result<Self, String> {
        let workspace = tempfile::tempdir().map_err(|error| format!("workspace: {error}"))?;
        let left = write_source(workspace.path(), LEFT_FILE, left_source)?;
        let right = write_source(workspace.path(), RIGHT_FILE, right_source)?;
        let (session, report) = scan_workspace(workspace.path())?;
        Ok(Self {
            workspace,
            session,
            report,
            endpoints: PairComparisonParams {
                left: body_endpoint(&left, left_source)?,
                right: body_endpoint(&right, right_source)?,
            },
            left_source: left_source.to_owned(),
            right_source: right_source.to_owned(),
        })
    }

    /// What `compare_pair` — the surface a user taps — says about two
    /// occurrences.
    fn comparison(&self, endpoints: &PairComparisonParams) -> Result<PairComparison, String> {
        self.session
            .compare_pair(endpoints, None)
            .map_err(|error| format!("`compare_pair` must resolve both endpoints: {error}"))
    }

    /// Whether the run's own report grouped the two files together.
    fn grouped_by_the_scan(&self) -> bool {
        self.report.clusters.iter().any(spans_both_files)
    }

    /// The two occurrences the report itself published as one
    /// two-member cluster — the pair a user taps, named by the report
    /// rather than by this test.
    fn published_pair(&self) -> Result<PairComparisonParams, String> {
        let cluster = self
            .report
            .clusters
            .iter()
            .find(|cluster| spans_both_files(cluster) && cluster.occurrence_count == PAIR_MEMBERS)
            .ok_or_else(|| {
                format!(
                    "fixture guard: the scan must publish these two files as one \
                     two-occurrence cluster; it published {:?}",
                    self.summarise_clusters()
                )
            })?;
        published_endpoints(cluster, self.root())
    }

    /// The workspace every published occurrence path is relative to.
    fn root(&self) -> &Path {
        self.workspace.path()
    }

    /// Whether both published endpoints are windows strictly inside an
    /// authored function — the only shape `PairScope::interior` speaks
    /// about, and so the only shape on which the rescue's scope and the
    /// comparison's can differ.
    fn published_pair_is_interior(&self, published: &PairComparisonParams) -> Result<bool, String> {
        Ok(is_interior_window(&self.left_source, &published.left)?
            && is_interior_window(&self.right_source, &published.right)?)
    }

    /// Every published cluster as its id, kind and occurrence count.
    fn summarise_clusters(&self) -> Vec<String> {
        self.report
            .clusters
            .iter()
            .map(|cluster| {
                format!(
                    "{} {:?} x{}",
                    cluster.id, cluster.kind, cluster.occurrence_count
                )
            })
            .collect()
    }
}

/// A cluster with exactly one pair in it.
const PAIR_MEMBERS: usize = 2;

/// The published cluster's first occurrence in each fixture file, as the
/// endpoints a user would tap.
fn published_endpoints(
    cluster: &ReportCluster,
    root: &Path,
) -> Result<PairComparisonParams, String> {
    let named = |wanted: &str| {
        cluster
            .occurrences
            .iter()
            .find(|occurrence| {
                occurrence
                    .path
                    .file_name()
                    .is_some_and(|name| name == wanted)
            })
            .map(|occurrence| PairEndpoint {
                path: root.join(&occurrence.path),
                start_byte: occurrence.start_byte,
                end_byte: occurrence.end_byte,
            })
    };
    Ok(PairComparisonParams {
        left: named(LEFT_FILE).ok_or_else(|| "the cluster must name the left file".to_owned())?,
        right: named(RIGHT_FILE)
            .ok_or_else(|| "the cluster must name the right file".to_owned())?,
    })
}

/// Whether one reported cluster holds an occurrence in each fixture file.
fn spans_both_files(cluster: &ReportCluster) -> bool {
    let named = |wanted: &str| {
        cluster.occurrences.iter().any(|occurrence| {
            occurrence
                .path
                .file_name()
                .is_some_and(|name| name == wanted)
        })
    };
    named(LEFT_FILE) && named(RIGHT_FILE)
}

/// Runs the production pipeline over one workspace, embeddings off.
fn scan_workspace(root: &Path) -> Result<(PipelineSession, crate::report::Report), String> {
    PipelineSession::initialise(root.to_path_buf(), MIN_NODES, false, None, embeddings_off())
        .map_err(|error| format!("the pipeline must run over the fixture: {error}"))
}

/// The embedding policy every fixture scans under: measured evidence
/// only, so the verdict is the deterministic one.
fn embeddings_off() -> EmbeddingSettings<'static> {
    EmbeddingSettings {
        mode: EmbeddingMode::Off,
        provider: None,
        batch_yield: None,
        progress: None,
    }
}

/// Writes one fixture file into the workspace and returns its path.
fn write_source(root: &Path, name: &str, source: &str) -> Result<PathBuf, String> {
    let path = root.join(name);
    fs::write(&path, source).map_err(|error| format!("writing {name}: {error}"))?;
    Ok(path)
}

/// The endpoint a user would name for the first function body in
/// `source` — a window strictly inside an authored declaration, which is
/// the only shape `PairScope::interior` speaks about.
fn body_endpoint(path: &Path, source: &str) -> Result<PairEndpoint, String> {
    let (file_id, _unused) = python_pair_ids();
    let tree = parse_python(source, file_id)?;
    let block = find_kind(&tree, BLOCK).ok_or_else(|| "the fixture must hold a body".to_owned())?;
    Ok(PairEndpoint {
        path: path.to_path_buf(),
        start_byte: block.byte_range.start,
        end_byte: block.byte_range.end,
    })
}

/// The normalised kind of a Python function declaration.
const FUNCTION_DEFINITION: &str = "function_definition";

/// Whether `endpoint` sits strictly inside an authored function in
/// `source` — the same relation `DeclarationScopes::enclosing` resolves,
/// read here off this test's own parse of the same bytes.
fn is_interior_window(source: &str, endpoint: &PairEndpoint) -> Result<bool, String> {
    let (file_id, _unused) = python_pair_ids();
    let tree = parse_python(source, file_id)?;
    let span = ByteRange {
        start: endpoint.start_byte,
        end: endpoint.end_byte,
    };
    Ok(encloses_strictly(&tree, span))
}

/// Whether any authored function in the tree strictly encloses `span`.
fn encloses_strictly(node: &NormalizedNode, span: ByteRange) -> bool {
    (node.kind == FUNCTION_DEFINITION && node.byte_range.strictly_encloses(span))
        || node
            .children
            .iter()
            .any(|child| encloses_strictly(child, span))
}

/// The first node of `kind` under `root`, in pre-order.
fn find_kind<'tree>(root: &'tree NormalizedNode, kind: &str) -> Option<&'tree NormalizedNode> {
    if root.kind == kind {
        return Some(root);
    }
    root.children
        .iter()
        .find_map(|child| find_kind(child, kind))
}

/// Parses `source` through the real Python plug-in.
fn parse_python(source: &str, file_id: FileId) -> Result<NormalizedNode, String> {
    PythonParser
        .parse_and_normalize(source.as_bytes(), file_id)
        .map_err(|error| format!("the Python fixture must parse: {error}"))
}
