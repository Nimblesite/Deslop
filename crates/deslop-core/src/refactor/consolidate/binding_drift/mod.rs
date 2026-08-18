//! Binding-drift gate ([AUTOFIX-CONSOLIDATE-GATE] v1.1).
//!
//! Byte-equivalence of the moved definitions is not sufficient: a free
//! name inside a definition may resolve to a module-local item that
//! differs across the duplicate files — the traffic-light shape,
//! identical `run` bodies each calling their own `next`. Consolidating
//! would re-bind such references, violating Schäfer's
//! `lookup(ref)_after == lookup(ref)_before`.
//!
//! The gate **proves stability or refuses** — it never assumes it
//! ([AUTOFIX-ZERO-RISK], hardened after the review):
//!
//! - free value names and type names must be top-level items defined
//!   byte-equivalently in every occurrence file (checked
//!   **transitively** through their own free names), bound by
//!   textually identical non-glob `use` declarations, or std-prelude
//!   names;
//! - names matching any *nested* definition (associated items in
//!   `impl` blocks, items inside `mod` blocks, enum variants) refuse —
//!   resolution through containers is not mechanically decidable here;
//! - a glob `use …::*` in any occurrence file makes every otherwise
//!   unproven name refuse;
//! - method-call names refuse when any occurrence file's `impl` blocks
//!   define an associated item of that name (receiver types are not
//!   resolved).
//!
//! The AST walks live in [`scan`]; this module owns every prove/refuse
//! decision.

mod scan;

use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
};

use scan::{
    glob_import_exists, impl_defines, names_in_definition, nested_definition_exists,
    occurrence_files, top_level_definitions, use_texts, ParsedFile,
};

use crate::{
    ast::ByteRange,
    lang::{shared::parse_source, LanguageParser},
    refactor::{consolidate::DefinitionSite, preconditions::raw_slices_equivalent, RefactorError},
};

/// Rust std-prelude names (types, traits, variants) plus ubiquitous
/// std macros — all resolve identically from any sibling module.
/// Primitive types parse as `primitive_type` nodes and never reach the
/// gate.
const RUST_PRELUDE: &[&str] = &[
    "AsMut",
    "AsRef",
    "Box",
    "Clone",
    "Copy",
    "Debug",
    "Default",
    "DoubleEndedIterator",
    "Drop",
    "Eq",
    "Err",
    "ExactSizeIterator",
    "Extend",
    "Fn",
    "FnMut",
    "FnOnce",
    "From",
    "FromIterator",
    "Hash",
    "Into",
    "IntoIterator",
    "Iterator",
    "None",
    "Ok",
    "Option",
    "Ord",
    "PartialEq",
    "PartialOrd",
    "Result",
    "Send",
    "Sized",
    "Some",
    "String",
    "Sync",
    "ToOwned",
    "ToString",
    "TryFrom",
    "TryInto",
    "Unpin",
    "Vec",
    "assert",
    "assert_eq",
    "assert_ne",
    "cfg",
    "concat",
    "dbg",
    "env",
    "eprint",
    "eprintln",
    "file",
    "format",
    "include_str",
    "line",
    "matches",
    "option_env",
    "print",
    "println",
    "stringify",
    "vec",
    "write",
    "writeln",
];

/// How one referenced name proved stable.
enum Stability {
    /// Proven without further work (use-bound, prelude).
    Proven,
    /// A top-level definition — its own references must recurse.
    TopLevel(ByteRange),
}

/// Runs the gate over every symbol group. The inner `Err` carries the
/// refusal reason naming the drifting or unprovable symbol.
///
/// # Errors
///
/// Returns [`RefactorError::Core`] when an occurrence file fails to
/// parse.
pub(super) fn gate<S: ::std::hash::BuildHasher>(
    groups: &[Vec<DefinitionSite>],
    sources: &HashMap<PathBuf, Vec<u8>, S>,
    parser: &dyn LanguageParser,
) -> Result<Result<(), String>, RefactorError> {
    let mut files = Vec::new();
    for path in occurrence_files(groups) {
        let Some(source) = sources.get(path) else {
            return Ok(Err(format!("no source for {}", path.display())));
        };
        let tree = parse_source(parser.id(), &parser.grammar(), source)?;
        files.push(ParsedFile { path, tree, source });
    }
    let consolidated: BTreeSet<String> = groups
        .iter()
        .filter_map(|group| group.first())
        .map(|site| site.name.clone())
        .collect();
    Ok(check_groups(groups, &files, parser, &consolidated))
}

/// Seeds the worklist from every canonical definition and drains it.
fn check_groups(
    groups: &[Vec<DefinitionSite>],
    files: &[ParsedFile<'_>],
    parser: &dyn LanguageParser,
    consolidated: &BTreeSet<String>,
) -> Result<(), String> {
    let mut pending: Vec<String> = Vec::new();
    let mut methods: BTreeSet<String> = BTreeSet::new();
    for canonical in groups.iter().filter_map(|group| group.first()) {
        let Some(file) = files.iter().find(|file| *file.path == canonical.path) else {
            continue;
        };
        let sets = names_in_definition(file, canonical.item_span, parser)?;
        pending.extend(sets.values_and_types);
        methods.extend(sets.methods);
    }
    drain_worklist(pending, &mut methods, files, parser, consolidated)?;
    for method in &methods {
        if impl_defines(method, files) {
            return Err(format!(
                "`{method}` may resolve to an impl-defined method the move would re-bind (v1 gate, issue #279)"
            ));
        }
    }
    Ok(())
}

/// Proves every pending name stable, recursing through top-level
/// definitions with a visited set.
fn drain_worklist(
    mut pending: Vec<String>,
    methods: &mut BTreeSet<String>,
    files: &[ParsedFile<'_>],
    parser: &dyn LanguageParser,
    consolidated: &BTreeSet<String>,
) -> Result<(), String> {
    let mut visited: BTreeSet<String> = BTreeSet::new();
    pending.sort_unstable_by(|left, right| right.cmp(left));
    while let Some(name) = pending.pop() {
        if consolidated.contains(&name) || !visited.insert(name.clone()) {
            continue;
        }
        if let Stability::TopLevel(span) = prove_stable(&name, files)? {
            let Some(canonical) = files.first() else {
                continue;
            };
            let sets = names_in_definition(canonical, span, parser)?;
            pending.extend(sets.values_and_types);
            methods.extend(sets.methods);
        }
    }
    Ok(())
}

/// Proves one name stable or refuses with the reason.
fn prove_stable(name: &str, files: &[ParsedFile<'_>]) -> Result<Stability, String> {
    if files
        .iter()
        .any(|file| nested_definition_exists(file, name))
    {
        return Err(format!(
            "`{name}` matches a definition nested inside another item (impl/mod/enum) — resolution is not mechanically decidable (v1 gate, issue #279)"
        ));
    }
    let definitions: Vec<Vec<ByteRange>> = files
        .iter()
        .map(|file| top_level_definitions(file, name))
        .collect();
    if definitions.iter().any(|spans| !spans.is_empty()) {
        return definitions_equivalent(name, files, &definitions);
    }
    if files.iter().any(|file| !use_texts(file, name).is_empty()) {
        return use_declarations_identical(name, files);
    }
    if files.iter().any(glob_import_exists) {
        return Err(format!(
            "`{name}` may be bound by a glob `use …::*` — not mechanically decidable (v1 gate, issue #279)"
        ));
    }
    if RUST_PRELUDE.contains(&name) {
        return Ok(Stability::Proven);
    }
    Err(format!(
        "`{name}` cannot be proven binding-stable across the duplicate files (v1 gate, issue #279)"
    ))
}

/// Module-local definitions of `name` must be exactly one per file and
/// byte-equivalent across all of them; the canonical span recurses.
fn definitions_equivalent(
    name: &str,
    files: &[ParsedFile<'_>],
    definitions: &[Vec<ByteRange>],
) -> Result<Stability, String> {
    if definitions.iter().any(|spans| spans.len() != 1) {
        return Err(format!(
            "`{name}` is not defined exactly once in every duplicate file — the moved reference would re-bind (issue #279)"
        ));
    }
    let slices: Option<Vec<&[u8]>> = files
        .iter()
        .zip(definitions)
        .map(|(file, spans)| {
            spans
                .first()
                .and_then(|span| file.source.get(span.start..span.end))
        })
        .collect();
    if !slices.is_some_and(|slices| raw_slices_equivalent(&slices)) {
        return Err(format!(
            "`{name}` is defined differently across the duplicate files — the moved reference would re-bind (issue #279)"
        ));
    }
    let span = definitions
        .first()
        .and_then(|spans| spans.first().copied())
        .ok_or_else(|| format!("`{name}` has no canonical definition span"))?;
    Ok(Stability::TopLevel(span))
}

/// `use` declarations mentioning `name` must be textually identical
/// across every occurrence file.
fn use_declarations_identical(name: &str, files: &[ParsedFile<'_>]) -> Result<Stability, String> {
    let per_file: Vec<BTreeSet<String>> = files.iter().map(|file| use_texts(file, name)).collect();
    let all_equal = per_file
        .windows(2)
        .all(|pair| matches!(pair, [left, right] if left == right));
    if all_equal {
        Ok(Stability::Proven)
    } else {
        Err(format!(
            "`use` declarations binding `{name}` differ across the duplicate files (issue #279)"
        ))
    }
}
