//! [CLONE-KIND-TESTING] Actual copied members survive unrelated content and changed inputs.

use std::collections::BTreeSet;

use tree_sitter::{Node, Parser, Tree};

use super::{bucket_groups::information::SHAPE_ONLY_DISJOINT, support::*};
use crate::common::{
    cluster_kind, cluster_line_spans, clusters, occurrence_paths, occurrences, IDENTICAL_KIND,
    NEARLY_IDENTICAL_KIND, STRUCTURAL_ONLY_KIND,
};

const SCAN_DIRECTORY: &str = "src";
const ORIGINAL_PATH: &str = "b-original.rs";
const COPY_PATH: &str = "c-copy.rs";
const UNRELATED_PATH: &str = "a-unrelated.rs";
const ORIGINAL_FUNCTION: &str = "render_invoice_header";
const UNRELATED_FUNCTION: &str = "schedule_backup_window";
const INCREMENTAL_SOURCE: &str = include_str!("../incremental_equivalence.rs");
const EDIT_FUNCTION: &str = "editing_one_file_matches_the_cold_report_of_the_post_edit_tree";
const ADD_FUNCTION: &str = "adding_a_duplicate_file_matches_the_cold_report_of_the_grown_tree";
const EDIT_PATH: &str = "edit.rs";
const ADD_PATH: &str = "add.rs";
const NAME_FIELD: &str = "name";
const BODY_FIELD: &str = "body";
const LAST_STATEMENT_INDEX: usize = 3;
const FIRST_STATEMENT_INDEX: u32 = 0;
const ONE_GROUP: usize = 1;
const TWO_GROUPS: usize = 2;
const FIRST_RANK: u64 = 1;
const ZERO: u64 = 0;
const CLONE_FILES: u64 = 2;
const CLONE_LINES: u64 = 22;
const MIXED_LINES: u64 = 33;
const FULL_PERCENT: f64 = 100.0;
const MIXED_PERCENT: f64 = 66.666_666_666_666_67;
const FUNCTION_SPAN: (u64, u64) = (1, 11);
const MESSAGE_SPAN: (u64, u64) = (1, 6);
const DEFAULT_MIN_NODES: u64 = 30;
const MIN_NODES_FIELD: &str = "min_nodes";
const WHITESPACE_VARIATION: &str = "  \t  ";
const STRING_CONTENT_KIND: &str = "string_content";
const RANK_FIELD: &str = "rank";
const MASS_FIELD: &str = "mass";
const COUNT_FIELD: &str = "occurrence_count";
const CLONE_COUNT_FIELD: &str = "clusters_total";
const DUPLICATED_LINES_FIELD: &str = "duplicated_loc";
const ANALYSED_LINES_FIELD: &str = "analysed_loc";
const DUPLICATED_FILES_FIELD: &str = "duplicated_files";
const PERCENT_FIELD: &str = "duplication_percent";

fn parse_source(source: &str) -> Result<Tree> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("fixture did not parse"))?;
    anyhow::ensure!(
        !tree.root_node().has_error(),
        "fixture must be valid Rust syntax"
    );
    Ok(tree)
}

fn named_function<'tree>(tree: &'tree Tree, source: &str, name: &str) -> Result<Node<'tree>> {
    let root = tree.root_node();
    let mut cursor = root.walk();
    let function = root
        .named_children(&mut cursor)
        .find(|node| {
            node.child_by_field_name(NAME_FIELD)
                .and_then(|name| name.utf8_text(source.as_bytes()).ok())
                == Some(name)
        })
        .ok_or_else(|| anyhow::anyhow!("missing fixture function {name}"));
    function
}

fn function_source(source: &str, name: &str) -> Result<String> {
    let tree = parse_source(source)?;
    Ok(format!(
        "{}\n",
        named_function(&tree, source, name)?.utf8_text(source.as_bytes())?
    ))
}

fn message_source(name: &str) -> Result<String> {
    let tree = parse_source(INCREMENTAL_SOURCE)?;
    let function = named_function(&tree, INCREMENTAL_SOURCE, name)?;
    let body = function
        .child_by_field_name(BODY_FIELD)
        .ok_or_else(|| anyhow::anyhow!("missing fixture body"))?;
    let source = statement_suffix(body, INCREMENTAL_SOURCE)?;
    Ok(format!("fn candidate() {{\n{source}\n}}\n"))
}

fn statement_suffix<'source>(body: Node<'_>, source: &'source str) -> Result<&'source str> {
    let mut cursor = body.walk();
    let statements: Vec<_> = body.named_children(&mut cursor).collect();
    let start = statements
        .iter()
        .rev()
        .nth(LAST_STATEMENT_INDEX)
        .ok_or_else(|| anyhow::anyhow!("missing assertion sequence"))?
        .start_byte();
    let end = statements
        .last()
        .ok_or_else(|| anyhow::anyhow!("missing result expression"))?
        .end_byte();
    source
        .get(start..end)
        .ok_or_else(|| anyhow::anyhow!("invalid fixture range"))
}

fn scan_sources(sources: &[(&str, &str)]) -> Result<Value> {
    let (tmp, scan_root, output) = seeded_scan(SCAN_DIRECTORY, |root| {
        for (path, source) in sources {
            fs::write(root.join(path), source)?;
        }
        Ok(())
    })?;
    run_scan(&scan_root, &tmp.path().join(REPORT_OUTPUT_STEM), &[])?;
    let report = read_json_report(&output.json)?;
    assert_eq!(
        field(&report, MIN_NODES_FIELD).as_u64(),
        Some(DEFAULT_MIN_NODES)
    );
    Ok(report)
}

fn change_spacing(source: &str, name: &str) -> Result<String> {
    let tree = parse_source(source)?;
    let function = named_function(&tree, source, name)?;
    let body = function
        .child_by_field_name(BODY_FIELD)
        .ok_or_else(|| anyhow::anyhow!("missing fixture body"))?;
    let first = body
        .named_child(FIRST_STATEMENT_INDEX)
        .ok_or_else(|| anyhow::anyhow!("missing fixture statement"))?
        .start_byte();
    insert_gap(source, first)
}

fn insert_gap(source: &str, index: usize) -> Result<String> {
    let before = source
        .get(..index)
        .ok_or_else(|| anyhow::anyhow!("invalid statement start"))?;
    let after = source
        .get(index..)
        .ok_or_else(|| anyhow::anyhow!("invalid statement suffix"))?;
    Ok(format!("{before}{WHITESPACE_VARIATION}{after}"))
}

fn first_string_content(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == STRING_CONTENT_KIND {
        return Some(node);
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find_map(first_string_content);
    found
}

fn change_literal_spacing(source: &str) -> Result<String> {
    let tree = parse_source(source)?;
    let literal = first_string_content(tree.root_node())
        .ok_or_else(|| anyhow::anyhow!("missing fixture string literal"))?;
    insert_gap(source, literal.start_byte())
}

fn group_of_kind<'report>(report: &'report Value, kind: &str) -> Result<&'report Value> {
    clusters(report)
        .iter()
        .find(|group| cluster_kind(group) == kind)
        .ok_or_else(|| anyhow::anyhow!("missing {kind} group: {report:#}"))
}

fn assert_members(group: &Value, paths: &[&str], span: (u64, u64)) -> Result<()> {
    let actual: BTreeSet<_> = occurrence_paths(group).into_iter().collect();
    assert_eq!(
        actual,
        paths.iter().map(|path| (*path).to_owned()).collect()
    );
    assert_eq!(occurrences(group).len(), paths.len());
    assert_eq!(
        field(group, COUNT_FIELD),
        &serde_json::to_value(paths.len())?
    );
    assert_eq!(cluster_line_spans(group), vec![span; paths.len()]);
    Ok(())
}

fn assert_mass_rank(group: &Value, rank: u64) {
    assert_eq!(field(group, RANK_FIELD).as_u64(), Some(rank));
    if rank == ZERO {
        assert_eq!(field(group, MASS_FIELD).as_u64(), Some(ZERO));
    } else {
        assert!(field(group, MASS_FIELD)
            .as_u64()
            .is_some_and(|mass| mass > ZERO));
    }
}

fn assert_metrics(report: &Value, analysed: u64, percent: f64) {
    assert_eq!(
        metric_field(report, CLONE_COUNT_FIELD).as_u64(),
        Some(FIRST_RANK)
    );
    assert_eq!(
        metric_field(report, DUPLICATED_LINES_FIELD).as_u64(),
        Some(CLONE_LINES)
    );
    assert_eq!(
        metric_field(report, DUPLICATED_FILES_FIELD).as_u64(),
        Some(CLONE_FILES)
    );
    assert_eq!(
        metric_field(report, ANALYSED_LINES_FIELD).as_u64(),
        Some(analysed)
    );
    assert_eq!(metric_field(report, PERCENT_FIELD).as_f64(), Some(percent));
}

fn assert_mixed_groups(report: &Value) -> Result<()> {
    assert_eq!(clusters(report).len(), TWO_GROUPS, "{report:#}");
    let kinds: Vec<_> = clusters(report).iter().map(cluster_kind).collect();
    assert_eq!(kinds, [IDENTICAL_KIND, STRUCTURAL_ONLY_KIND], "{report:#}");
    let copied = group_of_kind(report, IDENTICAL_KIND)?;
    assert_members(copied, &[ORIGINAL_PATH, COPY_PATH], FUNCTION_SPAN)?;
    assert_mass_rank(copied, FIRST_RANK);
    let information = group_of_kind(report, STRUCTURAL_ONLY_KIND)?;
    assert_members(
        information,
        &[UNRELATED_PATH, ORIGINAL_PATH, COPY_PATH],
        FUNCTION_SPAN,
    )?;
    assert_mass_rank(information, ZERO);
    assert_metrics(report, MIXED_LINES, MIXED_PERCENT);
    Ok(())
}

// [CLONE-KIND-FOLD] [CLONE-BUCKETS-STRUCTURAL-ONLY] The unrelated member sorts first.
#[test]
fn unrelated_reference_keeps_the_copied_subset_and_lists_information_last() -> Result<()> {
    let original = function_source(SHAPE_ONLY_DISJOINT, ORIGINAL_FUNCTION)?;
    let unrelated = function_source(SHAPE_ONLY_DISJOINT, UNRELATED_FUNCTION)?;
    let baseline = scan_sources(&[(ORIGINAL_PATH, &original), (COPY_PATH, &original)])?;
    assert_eq!(clusters(&baseline).len(), ONE_GROUP, "{baseline:#}");
    let copied = group_of_kind(&baseline, IDENTICAL_KIND)?;
    assert_members(copied, &[ORIGINAL_PATH, COPY_PATH], FUNCTION_SPAN)?;
    assert_mass_rank(copied, FIRST_RANK);
    assert_metrics(&baseline, CLONE_LINES, FULL_PERCENT);
    let mixed = scan_sources(&[
        (UNRELATED_PATH, &unrelated),
        (ORIGINAL_PATH, &original),
        (COPY_PATH, &original),
    ])?;
    assert_mixed_groups(&mixed)
}

// [CLONE-BUCKETS-NORTH-STAR] Same calls and order with changed values/messages are near-copies.
#[test]
fn changed_arguments_and_messages_keep_the_screenshot_pair_nearly_identical() -> Result<()> {
    let edited = message_source(EDIT_FUNCTION)?;
    let added = message_source(ADD_FUNCTION)?;
    let report = scan_sources(&[(EDIT_PATH, &edited), (ADD_PATH, &added)])?;
    assert_eq!(clusters(&report).len(), ONE_GROUP, "{report:#}");
    let group = group_of_kind(&report, NEARLY_IDENTICAL_KIND)?;
    assert_members(group, &[EDIT_PATH, ADD_PATH], MESSAGE_SPAN)?;
    assert_mass_rank(group, FIRST_RANK);
    assert_eq!(
        metric_field(&report, CLONE_COUNT_FIELD).as_u64(),
        Some(FIRST_RANK)
    );
    Ok(())
}

// [CLONE-BUCKETS-IDENTICAL] Formatting alone does not change the identity category.
#[test]
fn whitespace_changes_within_a_function_remain_identical_code() -> Result<()> {
    let original = function_source(SHAPE_ONLY_DISJOINT, ORIGINAL_FUNCTION)?;
    let spaced = change_spacing(&original, ORIGINAL_FUNCTION)?;
    assert_ne!(
        original, spaced,
        "the fixture must actually change source bytes"
    );
    let report = scan_sources(&[(ORIGINAL_PATH, &original), (COPY_PATH, &spaced)])?;
    assert_eq!(clusters(&report).len(), ONE_GROUP, "{report:#}");
    let group = group_of_kind(&report, IDENTICAL_KIND)?;
    assert_members(group, &[ORIGINAL_PATH, COPY_PATH], FUNCTION_SPAN)?;
    assert_mass_rank(group, FIRST_RANK);
    assert_metrics(&report, CLONE_LINES, FULL_PERCENT);
    Ok(())
}

// [CLONE-BUCKETS-IDENTICAL] Whitespace inside a literal changes content, not formatting.
#[test]
fn whitespace_inside_a_string_literal_is_a_near_copy_content_change() -> Result<()> {
    let original =
        change_literal_spacing(&function_source(SHAPE_ONLY_DISJOINT, ORIGINAL_FUNCTION)?)?;
    let changed = change_literal_spacing(&original)?;
    assert_ne!(
        original, changed,
        "the fixture must change literal source bytes"
    );
    let report = scan_sources(&[(ORIGINAL_PATH, &original), (COPY_PATH, &changed)])?;
    assert_eq!(clusters(&report).len(), ONE_GROUP, "{report:#}");
    let group = group_of_kind(&report, NEARLY_IDENTICAL_KIND)?;
    assert_members(group, &[ORIGINAL_PATH, COPY_PATH], FUNCTION_SPAN)?;
    assert_mass_rank(group, FIRST_RANK);
    assert_metrics(&report, CLONE_LINES, FULL_PERCENT);
    Ok(())
}
