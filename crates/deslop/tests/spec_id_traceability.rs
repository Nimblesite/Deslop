//! [SPEC-ID-GATE] A rule identifier cited in code or tests must resolve to a
//! written rule.
//!
//! Comments cite a rule by a bracketed identifier so that searching for it
//! finds the written rule, the code implementing it, and the tests holding it.
//! That three-way trace is how documents, code and tests are kept honest with
//! one another, and it silently returns nothing the moment an identifier is
//! invented rather than looked up. Thirty-six of them had drifted that way, and
//! twenty were cited from shipped code — including the identifier the
//! contributor instructions use as their worked example. A test kept two
//! commands deleted under a rule nobody could read.
//!
//! Identifiers are read from parsed comment nodes, never from a text scan of
//! source. Documents are prose, so a heading or a bold requirement lead-in is
//! read as written.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use tree_sitter::{Node, Parser};

/// Directories whose sources cite rules: every crate's code and suites, and
/// the extension host.
const SOURCE_ROOTS: [&str; 2] = ["crates", "clients/vscode/src"];

/// The documentation tree every citation must resolve into.
const DOC_ROOT: &str = "docs";

/// Source extensions this gate can parse.
const RUST_EXTENSION: &str = "rs";
/// TypeScript source extension.
const TS_EXTENSION: &str = "ts";
/// TypeScript-with-JSX source extension.
const TSX_EXTENSION: &str = "tsx";

/// Markdown specification extension.
const MARKDOWN_EXTENSION: &str = "md";
/// Type-diagram model extension; its comments carry the wire rules.
const TYPEDIAGRAM_EXTENSION: &str = "td";

/// Tree-sitter node kinds that hold a comment in the grammars scanned.
const COMMENT_KINDS: [&str; 3] = ["line_comment", "block_comment", "comment"];

/// Fence marker; identifiers inside a fenced block are examples, not rules.
const FENCE: &str = "```";
/// Heading marker.
const HEADING: char = '#';
/// List and bold markers may precede an identifier that leads a requirement,
/// as may an ordinal; nothing else may.
const LEAD_IN_MARKS: [char; 6] = ['#', '*', '-', '.', ' ', '\t'];

/// Directories that hold no first-party source.
const SKIPPED_DIRECTORIES: [&str; 4] = ["node_modules", "target", "out", "media"];

/// The repository root, reached from this crate's manifest so the gate reads
/// the same tree whatever directory the runner starts in.
fn repo_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    manifest.canonicalize().unwrap_or(manifest)
}

#[test]
fn every_cited_rule_identifier_resolves_to_a_written_rule() {
    let defined = documented_identifiers();
    let cited = cited_identifiers();
    let orphans: BTreeMap<&String, &BTreeSet<String>> = cited
        .iter()
        .filter(|(identifier, _)| !defined.contains(*identifier))
        .collect();
    assert!(
        orphans.is_empty(),
        "{} rule identifier(s) are cited in code or tests and defined in no \
         document under {DOC_ROOT}/. Give each one a section carrying its \
         identifier, rename the citation to the identifier it meant, or delete \
         the citation with the code it describes:\n{}",
        orphans.len(),
        render(&orphans)
    );
}

#[test]
fn an_invented_identifier_is_refused_by_the_same_rule_the_gate_applies() {
    let defined = documented_identifiers();
    assert!(
        !defined.contains("SPEC-ID-GATE-NO-SUCH-RULE"),
        "the gate's own negative control must name a rule no document defines"
    );
    assert!(
        defined.contains("SPEC-ID-GATE"),
        "this suite's own identifier must resolve, or the gate cannot be traced"
    );
}

/// Names every orphan with the sites that cite it.
fn render(orphans: &BTreeMap<&String, &BTreeSet<String>>) -> String {
    orphans
        .iter()
        .map(|(identifier, sites)| {
            let listed: Vec<&str> = sites.iter().map(String::as_str).collect();
            format!("  [{identifier}]\n      {}", listed.join("\n      "))
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// Every identifier the documents define, from headings and bold requirement
/// lead-ins.
fn documented_identifiers() -> BTreeSet<String> {
    let mut defined = BTreeSet::new();
    let docs = repo_root().join(DOC_ROOT);
    for path in files_under(&docs, &[MARKDOWN_EXTENSION, TYPEDIAGRAM_EXTENSION]) {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let model = path
            .extension()
            .is_some_and(|kind| kind == TYPEDIAGRAM_EXTENSION);
        collect_definitions(&text, model, &mut defined);
    }
    defined
}

/// Reads one document's definition lines into `defined`.
fn collect_definitions(text: &str, model: bool, defined: &mut BTreeSet<String>) {
    let mut fenced = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(FENCE) {
            fenced = !fenced;
            continue;
        }
        defined.extend(defined_by(line, trimmed, model, fenced));
    }
}

/// The identifiers one line defines, as opposed to merely mentions. A heading
/// defines every identifier it carries, wherever in the heading it sits. Any
/// other line defines an identifier only when that identifier leads it.
fn defined_by(line: &str, trimmed: &str, model: bool, fenced: bool) -> BTreeSet<String> {
    if model {
        return if trimmed.starts_with(HEADING) {
            identifiers(line)
        } else {
            BTreeSet::new()
        };
    }
    if fenced {
        return BTreeSet::new();
    }
    if line.starts_with(HEADING) {
        return identifiers(line);
    }
    leading_identifier(line)
}

/// The identifier a requirement line leads with, after list markers, ordinals
/// and bold markers.
fn leading_identifier(line: &str) -> BTreeSet<String> {
    let Some((lead_in, rest)) = line.split_once('[') else {
        return BTreeSet::new();
    };
    if !lead_in.trim_matches(is_lead_in).is_empty() {
        return BTreeSet::new();
    }
    let Some((candidate, _)) = rest.split_once(']') else {
        return BTreeSet::new();
    };
    identifiers(&format!("[{candidate}]"))
}

/// Whether `character` may sit between the start of a requirement line and the
/// identifier it leads with — a list marker, an ordinal, or a bold marker.
fn is_lead_in(character: char) -> bool {
    LEAD_IN_MARKS.contains(&character) || character.is_ascii_digit()
}

/// Every identifier cited by a comment, mapped to the sites citing it.
fn cited_identifiers() -> BTreeMap<String, BTreeSet<String>> {
    let mut cited: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let root_directory = repo_root();
    for root in SOURCE_ROOTS {
        let extensions = [RUST_EXTENSION, TS_EXTENSION, TSX_EXTENSION];
        for path in files_under(&root_directory.join(root), &extensions) {
            collect_citations(&path, &mut cited);
        }
    }
    cited
}

/// Parses one source file and records every identifier its comments cite.
fn collect_citations(path: &Path, cited: &mut BTreeMap<String, BTreeSet<String>>) {
    let Ok(source) = fs::read(path) else { return };
    let Some(mut parser) = parser_for(path) else {
        return;
    };
    let Some(tree) = parser.parse(&source, None) else {
        return;
    };
    let mut pending = vec![tree.root_node()];
    while let Some(node) = pending.pop() {
        if COMMENT_KINDS.contains(&node.kind()) {
            record(node, &source, path, cited);
            continue;
        }
        pending.extend(children(node));
    }
}

/// The named and unnamed children of `node`, so comments inside any construct
/// are reached.
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}

/// Records one comment's identifiers against its `path:line`.
fn record(
    node: Node<'_>,
    source: &[u8],
    path: &Path,
    cited: &mut BTreeMap<String, BTreeSet<String>>,
) {
    let Some(bytes) = source.get(node.byte_range()) else {
        return;
    };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return;
    };
    let line = node.start_position().row.saturating_add(1);
    let site = format!("{}:{line}", path.display());
    for identifier in identifiers(text) {
        let _recorded = cited.entry(identifier).or_default().insert(site.clone());
    }
}

/// A parser for `path`'s grammar, or `None` when the extension is not scanned.
fn parser_for(path: &Path) -> Option<Parser> {
    let language = match path.extension()?.to_str()? {
        RUST_EXTENSION => tree_sitter_rust::LANGUAGE.into(),
        TS_EXTENSION => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        TSX_EXTENSION => tree_sitter_typescript::LANGUAGE_TSX.into(),
        _ => return None,
    };
    let mut parser = Parser::new();
    parser.set_language(&language).ok()?;
    Some(parser)
}

/// Reads bracket-delimited rule slugs without treating code as text patterns.
/// A slug is two or more upper-case alphanumeric parts joined by hyphens, at
/// least one of which carries a letter — so a character class such as `[0-9]`
/// inside a comment is not mistaken for a rule.
fn identifiers(text: &str) -> BTreeSet<String> {
    text.split('[')
        .skip(1)
        .filter_map(|fragment| fragment.split_once(']'))
        .map(|(candidate, _)| candidate)
        .filter(|candidate| is_rule_slug(candidate))
        .map(str::to_owned)
        .collect()
}

/// Whether `candidate` is shaped like a hierarchical rule identifier.
fn is_rule_slug(candidate: &str) -> bool {
    let parts: Vec<&str> = candidate.split('-').collect();
    parts.len() > 1
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
                && part
                    .chars()
                    .all(|character| !character.is_ascii_lowercase())
        })
        && parts.iter().any(|part| {
            part.chars()
                .any(|character| character.is_ascii_alphabetic())
        })
}

/// Every file under `root` carrying one of `extensions`, skipping build output
/// and vendored trees.
fn files_under(root: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            visit(&entry.path(), extensions, &mut pending, &mut found);
        }
    }
    found.sort();
    found
}

/// Queues a directory or keeps a file of interest.
fn visit(path: &Path, extensions: &[&str], pending: &mut Vec<PathBuf>, found: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let skipped = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| SKIPPED_DIRECTORIES.contains(&name));
        if !skipped {
            pending.push(path.to_path_buf());
        }
        return;
    }
    let matched = path
        .extension()
        .and_then(|kind| kind.to_str())
        .is_some_and(|kind| extensions.contains(&kind));
    if matched {
        found.push(path.to_path_buf());
    }
}
