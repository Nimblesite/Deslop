//! Behaviour-preservation preconditions ([AUTOFIX-MERGE-SAFETY]).
//!
//! Check A (structural, consistent holes) is established by the gate;
//! this module adds B (single-entry/single-exit; its
//! declared-inside-read-after dataflow half is the shared
//! [`read_after_check`] in `preconditions`, which extract rule 6 also
//! runs — issue #278), the Baker rename lifting (gate step 3), C's
//! shadow-free naming inputs, and D's value-parameter typing. Any
//! undecidable check refuses — a false "unsafe" beats a false "safe"
//! (Opdyke bias, [AUTOFIX-ZERO-RISK]).

use std::collections::{HashMap, HashSet};

use tree_sitter::Node;

use crate::{
    ast::ByteRange,
    lang::{shared::LITERAL_KIND, LanguageParser},
    refactor::{
        merge::gate::Hole,
        preconditions::{node_text, read_after_check, run_bound_names, OccurrenceScope},
        tables::{MergeTables, ScopeKinds},
    },
};

/// One hole promoted to a typed value parameter, or lifted as a
/// consistent local rename.
#[derive(Debug)]
pub enum HoleRole {
    /// Consistent bijective local rename — keeps the canonical site's
    /// name, no parameter ([AUTOFIX-MERGE-GATE] step 3).
    LiftedRename,
    /// A typed value parameter ([AUTOFIX-MERGE-SAFETY] D).
    Parameter {
        /// Unified declared type across every site.
        type_name: String,
    },
}

/// Runs checks B–D. Returns one [`HoleRole`] per hole (gate order).
///
/// # Errors
///
/// `Err` carries the human-readable refusal reason.
pub fn evaluate(
    scopes: &[OccurrenceScope<'_>],
    source: &[u8],
    holes: &[Hole],
    parser: &dyn LanguageParser,
    tables: &'static MergeTables,
    scope_kinds: &'static ScopeKinds,
) -> Result<Vec<HoleRole>, String> {
    boundary_check(scopes, tables, scope_kinds)?;
    read_after_check(scopes, source, parser, scope_kinds)?;
    let bound_per_site = bound_names_per_site(scopes, source, parser, scope_kinds);
    classify_holes(scopes, source, holes, parser, tables, &bound_per_site)
}

/// Check B (control flow): no control transfer crossing the span
/// boundary. A transfer is neutralised only by an allowed container
/// that itself sits inside the span; nested scope frames are opaque.
fn boundary_check(
    scopes: &[OccurrenceScope<'_>],
    tables: &'static MergeTables,
    scope_kinds: &'static ScopeKinds,
) -> Result<(), String> {
    for scope in scopes {
        for node in &scope.run {
            boundary_scan(*node, scope.span(), tables, scope_kinds)?;
        }
    }
    Ok(())
}

/// Recursive control-transfer scan for one subtree.
fn boundary_scan(
    node: Node<'_>,
    span: ByteRange,
    tables: &'static MergeTables,
    scope_kinds: &'static ScopeKinds,
) -> Result<(), String> {
    if scope_kinds
        .frame_kinds
        .iter()
        .any(|frame| frame.node_kind == node.kind())
    {
        return Ok(());
    }
    if let Some(boundary) = tables
        .boundary_kinds
        .iter()
        .find(|entry| entry.node_kind == node.kind())
    {
        if !transfer_is_contained(node, span, boundary.allowed_containers) {
            return Err(format!(
                "`{}` transfers control across the merge boundary",
                node.kind()
            ));
        }
    }
    for child in crate::refactor::preconditions::named_children(node) {
        boundary_scan(child, span, tables, scope_kinds)?;
    }
    Ok(())
}

/// True when a control transfer has an allowed container fully inside
/// the span (a `break` inside its own loop).
fn transfer_is_contained(node: Node<'_>, span: ByteRange, containers: &[&str]) -> bool {
    let mut current = node.parent();
    while let Some(candidate) = current {
        if candidate.start_byte() < span.start || candidate.end_byte() > span.end {
            return false;
        }
        if containers.contains(&candidate.kind()) {
            return true;
        }
        current = candidate.parent();
    }
    false
}

/// Per-site bound-name sets, used by the rename lifting.
fn bound_names_per_site(
    scopes: &[OccurrenceScope<'_>],
    source: &[u8],
    parser: &dyn LanguageParser,
    scope_kinds: &'static ScopeKinds,
) -> Vec<HashSet<String>> {
    scopes
        .iter()
        .map(|scope| run_bound_names(scope, source, parser, scope_kinds))
        .collect()
}

/// Gate step 3 + check D: identifier holes whose every site names an
/// in-span local lift as consistent renames; the rest become typed
/// value parameters or refuse.
fn classify_holes(
    scopes: &[OccurrenceScope<'_>],
    source: &[u8],
    holes: &[Hole],
    parser: &dyn LanguageParser,
    tables: &'static MergeTables,
    bound_per_site: &[HashSet<String>],
) -> Result<Vec<HoleRole>, String> {
    let mut rename_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut roles = Vec::with_capacity(holes.len());
    for hole in holes {
        if hole.normalized_kind == LITERAL_KIND {
            interpolation_guard(hole, scopes, parser)?;
            roles.push(literal_role(hole, scopes, tables)?);
        } else if hole_is_local_everywhere(hole, bound_per_site) {
            lift_rename(hole, &mut rename_map)?;
            roles.push(HoleRole::LiftedRename);
        } else {
            roles.push(identifier_role(hole, source, scopes, tables, parser)?);
        }
    }
    bijective_guard(&rename_map)?;
    Ok(roles)
}

/// True when every site's text names a local bound inside that site's
/// own span.
fn hole_is_local_everywhere(hole: &Hole, bound_per_site: &[HashSet<String>]) -> bool {
    hole.per_site
        .iter()
        .zip(bound_per_site)
        .all(|(site, bound)| bound.contains(&site.text))
}

/// Records one rename tuple, refusing on an inconsistent mapping.
fn lift_rename(hole: &Hole, rename_map: &mut HashMap<String, Vec<String>>) -> Result<(), String> {
    let canonical = hole
        .per_site
        .first()
        .map(|site| site.text.clone())
        .unwrap_or_default();
    let tuple: Vec<String> = hole.per_site.iter().map(|site| site.text.clone()).collect();
    match rename_map.get(&canonical) {
        Some(existing) if *existing != tuple => Err(format!(
            "local `{canonical}` is renamed inconsistently across occurrences"
        )),
        Some(_) => Ok(()),
        None => {
            let _new = rename_map.insert(canonical, tuple);
            Ok(())
        }
    }
}

/// Rejects two canonical locals mapping onto one target name at any
/// site — a collision would change bindings (Schäfer invariant).
fn bijective_guard(rename_map: &HashMap<String, Vec<String>>) -> Result<(), String> {
    let tuples: Vec<&Vec<String>> = rename_map.values().collect();
    let site_count = tuples.first().map_or(0, |tuple| tuple.len());
    for site in 0..site_count {
        let mut seen = HashSet::new();
        for tuple in &tuples {
            let Some(name) = tuple.get(site) else {
                continue;
            };
            if !seen.insert(name.clone()) {
                return Err(format!(
                    "two locals collapse onto `{name}` at one occurrence — rename is not bijective"
                ));
            }
        }
    }
    Ok(())
}

/// Refuses literal holes that interpolate identifiers (Dart template
/// strings, C# interpolated strings): passing them as call arguments
/// would smuggle in-span locals out of scope.
fn interpolation_guard(
    hole: &Hole,
    scopes: &[OccurrenceScope<'_>],
    parser: &dyn LanguageParser,
) -> Result<(), String> {
    let references = parser.identifier_reference_kinds();
    for (site, scope) in hole.per_site.iter().zip(scopes) {
        let Some(raw) = raw_node_at(scope, site.range) else {
            continue;
        };
        let mut stack = vec![raw];
        while let Some(node) = stack.pop() {
            if node.id() != raw.id() && references.reference_kinds.contains(&node.kind()) {
                return Err(
                    "a differing literal interpolates identifiers — not a value parameter"
                        .to_owned(),
                );
            }
            stack.extend(crate::refactor::preconditions::named_children(node));
        }
    }
    Ok(())
}

/// Check D for a literal hole: the raw literal kinds must map to one
/// declared type across every site.
fn literal_role(
    hole: &Hole,
    scopes: &[OccurrenceScope<'_>],
    tables: &'static MergeTables,
) -> Result<HoleRole, String> {
    let types: Option<Vec<&str>> = hole
        .per_site
        .iter()
        .zip(scopes)
        .map(|(site, scope)| literal_type(site.range, scope, tables))
        .collect();
    let Some(types) = types else {
        return Err("a differing literal has no unifiable declared type".to_owned());
    };
    let Some(first) = types.first().copied() else {
        return Err("a differing literal has no sites".to_owned());
    };
    if types.iter().any(|entry| *entry != first) {
        return Err("differing literals disagree on type across occurrences".to_owned());
    }
    Ok(HoleRole::Parameter {
        type_name: first.to_owned(),
    })
}

/// Declared type for one literal leaf, via the raw node kind —
/// climbing same-span ancestors because grammars nest quote-variant
/// nodes inside the literal (Dart's `string_literal_double_quotes`).
fn literal_type(
    range: ByteRange,
    scope: &OccurrenceScope<'_>,
    tables: &'static MergeTables,
) -> Option<&'static str> {
    let mut raw = raw_node_at(scope, range)?;
    loop {
        let mapped = tables
            .literal_types
            .iter()
            .find(|(kind, _)| *kind == raw.kind())
            .map(|(_, type_name)| *type_name);
        if mapped.is_some() {
            return mapped;
        }
        let parent = raw.parent()?;
        if parent.start_byte() != range.start || parent.end_byte() != range.end {
            return None;
        }
        raw = parent;
    }
}

/// Check D for a free-identifier hole: every site's identifier must
/// carry the same explicit declared type and never be written inside
/// its span.
fn identifier_role(
    hole: &Hole,
    source: &[u8],
    scopes: &[OccurrenceScope<'_>],
    tables: &'static MergeTables,
    parser: &dyn LanguageParser,
) -> Result<HoleRole, String> {
    let mut unified: Option<String> = None;
    for (site, scope) in hole.per_site.iter().zip(scopes) {
        if written_in_span(scope, &site.text, source, tables) {
            return Err(format!(
                "`{}` is written inside the span — call-time evaluation would change behaviour",
                site.text
            ));
        }
        let function = scope.function.ok_or_else(|| {
            "identifier parameters need an enclosing function for type lookup".to_owned()
        })?;
        let declared = parser
            .declared_type_of(function, &site.text, source)
            .ok_or_else(|| format!("no explicit declared type found for `{}`", site.text))?;
        match &unified {
            Some(existing) if *existing != declared => {
                return Err(format!(
                    "`{}` unifies to `{declared}` but another site declared `{existing}`",
                    site.text
                ));
            }
            Some(_) => {}
            None => unified = Some(declared),
        }
    }
    unified
        .map(|type_name| HoleRole::Parameter { type_name })
        .ok_or_else(|| "identifier hole has no sites".to_owned())
}

/// True when `name` is an assignment target anywhere inside the
/// occurrence span.
fn written_in_span(
    scope: &OccurrenceScope<'_>,
    name: &str,
    source: &[u8],
    tables: &'static MergeTables,
) -> bool {
    scope.run.iter().any(|node| {
        let mut stack = vec![*node];
        while let Some(current) = stack.pop() {
            if let Some((_, field)) = tables
                .write_kinds
                .iter()
                .find(|(kind, _)| *kind == current.kind())
            {
                let target = current
                    .child_by_field_name(*field)
                    .and_then(|child| node_text(child, source));
                if target.as_deref() == Some(name) {
                    return true;
                }
            }
            stack.extend(crate::refactor::preconditions::named_children(current));
        }
        false
    })
}

/// Raw tree-sitter node covering `range` within one occurrence's run.
fn raw_node_at<'t>(scope: &OccurrenceScope<'t>, range: ByteRange) -> Option<Node<'t>> {
    scope
        .run
        .iter()
        .find(|node| node.start_byte() <= range.start && range.end <= node.end_byte())
        .and_then(|node| node.named_descendant_for_byte_range(range.start, range.end))
}
