//! [CLONE-BUCKETS-STRUCTURAL-ONLY] Informational findings and renamed clones.

use super::*;

const CONSISTENT_RENAME_PARSERS: &str = r#"pub struct TypeScriptParser;

impl TypeScriptParser {
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for TypeScriptParser {
    fn id(&self) -> &'static str {
        TYPESCRIPT_LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["ts"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn parse_and_normalize(
        &self,
        source: &[u8],
        file_id: FileId,
    ) -> Result<NormalizedNode, CoreError> {
        parse_and_normalise(TYPESCRIPT_LANGUAGE_ID, &self.grammar(), source, file_id)
    }
}

pub struct TsxParser;

impl TsxParser {
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for TsxParser {
    fn id(&self) -> &'static str {
        TSX_LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["tsx"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    }

    fn parse_and_normalize(
        &self,
        source: &[u8],
        file_id: FileId,
    ) -> Result<NormalizedNode, CoreError> {
        parse_and_normalise(TSX_LANGUAGE_ID, &self.grammar(), source, file_id)
    }
}
"#;

/// Matching control flow over disjoint content: the calls and literals
/// differ, including within each body. [CLONE-BUCKETS-STRUCTURAL-ONLY]
/// makes it informational — "not a clone; listed for interest".
pub(crate) const SHAPE_ONLY_DISJOINT: &str = r#"pub fn render_invoice_header(doc: &mut Doc) {
    let title = doc.lookup("invoice");
    let title2 = doc.extract("receipt");
    doc.push(title);
    doc.push(title2);
    for row in doc.rows() {
        doc.indent(row);
        doc.emit(row);
    }
    doc.flush();
}

pub fn schedule_backup_window(queue: &mut Queue) {
    let alpha = queue.reserve("nightly");
    let beta = queue.reserve("weekly");
    queue.enqueue(alpha);
    queue.enqueue(beta);
    for slot in queue.slots() {
        queue.lock(slot);
        queue.commit(slot);
    }
    queue.drain();
}
"#;

/// The file the rename fixture is seeded into.
const RENAME_FILE: &str = "parsers.rs";
/// The file the shape-only fixture is seeded into.
const SHAPE_ONLY_FILE: &str = "shapes.rs";
/// Both fixtures hold exactly one relation, so exactly one group.
const ONE_GROUP: usize = 1;
/// Both fixtures hold exactly two occurrences of that relation.
const TWO_OCCURRENCES: u64 = 2;
/// A shape-only match contributes no duplicated lines, no duplicated
/// files and no clone groups ([CLONE-BUCKETS-STRUCTURAL-ONLY]).
const NO_DUPLICATION: u64 = 0;
/// ...and therefore no duplication percentage.
const NO_DUPLICATION_PERCENT: f64 = 0.0;
/// Human presentation expectations for an information-only scan.
const INFORMATION_NOTE: &str = "Informational — not a clone.";
const INFORMATION_DIR: &str = "shape_presentation";
const ZERO_CLONE_HEADLINE: &str = "Found 0 groups of duplicated code";
const ZERO_CLONE_HTML_TITLE: &str = "0 duplicate group(s)";
const DUPLICATION_CLAIMS: [&str; 3] = ["mass 0", "mass=0", "#0 "];

/// Seeds `source` into a fresh scan root as `name`, runs the CLI over it
/// and returns the parsed JSON report.
fn report_for(dir_name: &str, name: &str, source: &str) -> Result<Value> {
    Ok(serde_json::from_str(
        &render_for(dir_name, name, source)?.1.json,
    )?)
}

/// Runs the same fixture once and retains every human surface and the canonical JSON.
fn render_for(dir_name: &str, name: &str, source: &str) -> Result<(String, RenderedReports)> {
    let (tmp, scan_root, out) = seeded_scan(dir_name, |root| {
        fs::write(root.join(name), source)?;
        Ok(())
    })?;
    let mut cmd = deslop_command(&scan_root, &tmp.path().join(REPORT_OUTPUT_STEM))?;
    let assertion = cmd
        .args([MIN_NODES_FLAG, MIN_NODES_VALUE, NO_COLOR_FLAG])
        .assert()
        .success();
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).into_owned();
    let reports = RenderedReports {
        json: fs::read_to_string(&out.json)?,
        html: fs::read_to_string(&out.html)?,
        txt: fs::read_to_string(&out.txt)?,
    };
    Ok((stderr, reports))
}

// [CLONE-BUCKETS-ROUTING] A systematic rename remains a clone despite low raw spelling agreement.
#[test]
fn a_consistent_rename_is_a_near_copy_not_shape_only_information() -> Result<()> {
    let report = report_for("rename", RENAME_FILE, CONSISTENT_RENAME_PARSERS)?;
    let groups = clusters(&report);
    assert_eq!(
        groups.len(),
        ONE_GROUP,
        "the two renamed parser impls form one group: {report:#}"
    );
    let group = groups
        .first()
        .ok_or_else(|| anyhow::anyhow!("expected a renamed clone"))?;
    assert_eq!(
        cluster_kind(group),
        NEARLY_IDENTICAL_KIND,
        "a one-for-one identifier and literal substitution is a copy, not \
         matching shape over unrelated content — [CLONE-BUCKETS-NORTH-STAR] \
         puts a systematic rename in {NEARLY_IDENTICAL_TITLE}, and \
         {STRUCTURAL_ONLY_TITLE} is reserved for negligible shared content: {group:#}"
    );
    assert_eq!(
        field(group, "occurrence_count").as_u64(),
        Some(TWO_OCCURRENCES),
        "both impls are members: {group:#}"
    );
    assert!(
        metric_field(&report, "duplicated_loc")
            .as_u64()
            .is_some_and(|loc| loc > NO_DUPLICATION),
        "a copied region contributes duplicated lines: {report:#}"
    );
    assert_eq!(
        metric_field(&report, "clusters_total").as_u64(),
        Some(ONE_GROUP as u64),
        "the copy is counted as a clone group: {report:#}"
    );
    Ok(())
}

// [CLONE-BUCKETS-STRUCTURAL-ONLY] Source remains inspectable without duplication claims.
#[test]
fn shape_only_presentations_identify_information_and_keep_locations() -> Result<()> {
    let (stderr, reports) = render_for(INFORMATION_DIR, SHAPE_ONLY_FILE, SHAPE_ONLY_DISJOINT)?;
    assert_contains(
        &stderr,
        ZERO_CLONE_HEADLINE,
        "only clone groups count in the headline",
    );
    assert_contains(
        &reports.html,
        ZERO_CLONE_HTML_TITLE,
        "HTML title reports clone count",
    );
    for body in [&stderr, &reports.html, &reports.txt] {
        assert_informational_presentation(body);
    }
    Ok(())
}

/// Every human surface names the finding honestly and keeps its source reachable.
fn assert_informational_presentation(body: &str) {
    assert_contains(body, INFORMATION_NOTE, "shape-only is clearly labelled");
    assert_contains(body, SHAPE_ONLY_FILE, "source location remains available");
    for claim in DUPLICATION_CLAIMS {
        assert_not_contains(body, claim, "information has no duplication weight or rank");
    }
}

// [CLONE-BUCKETS-STRUCTURAL-ONLY] Informational findings contribute no duplication; source stays in the denominator.
#[test]
fn shape_only_information_is_excluded_from_every_duplication_figure() -> Result<()> {
    let report = report_for("shape_only", SHAPE_ONLY_FILE, SHAPE_ONLY_DISJOINT)?;
    let informational: Vec<&Value> = clusters(&report)
        .iter()
        .filter(|group| cluster_kind(group) == STRUCTURAL_ONLY_KIND)
        .collect();
    assert_eq!(
        informational.len(),
        ONE_GROUP,
        "the disjoint-content pair is listed for interest as \
         {STRUCTURAL_ONLY_TITLE}: {report:#}"
    );
    assert_eq!(
        metric_field(&report, "duplicated_loc").as_u64(),
        Some(NO_DUPLICATION),
        "{STRUCTURAL_ONLY_TITLE} is not duplication, so it covers no \
         duplicated lines: {report:#}"
    );
    assert_eq!(
        metric_field(&report, "duplicated_files").as_u64(),
        Some(NO_DUPLICATION),
        "a file holding only shape-only information is not a duplicated \
         file: {report:#}"
    );
    assert_eq!(
        metric_field(&report, "clusters_total").as_u64(),
        Some(NO_DUPLICATION),
        "an informational finding is not a clone group: {report:#}"
    );
    assert_eq!(
        metric_field(&report, "duplication_percent").as_f64(),
        Some(NO_DUPLICATION_PERCENT),
        "a corpus carrying only shape-only information reports zero \
         duplication: {report:#}"
    );
    assert!(
        metric_field(&report, "analysed_loc")
            .as_u64()
            .is_some_and(|loc| loc > NO_DUPLICATION),
        "shape-only source lines stay in the analysed-line denominator: {report:#}"
    );
    Ok(())
}
