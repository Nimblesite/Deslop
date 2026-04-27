//! Value-token comparison used while materialising report buckets.
//!
//! Normalised structural hashes intentionally collapse identifiers so
//! renamed-variable Type-2 clones still match. For [CLONE-BUCKETS], however,
//! a C# cluster with different literals or member-access names must not be
//! presented as user-facing identical code.

use std::{collections::HashMap, hash::BuildHasher};

use tree_sitter::Node;

use crate::{
    ast::ByteRange,
    fingerprint::Fingerprint,
    lang::{csharp, shared::parse_source},
    state::FileId,
};

/// Returns true when supported-language members carry the same value tokens.
pub(crate) fn value_tokens_are_identical<S: BuildHasher>(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
) -> bool {
    let Some(first) = members
        .first()
        .and_then(|member| member_value_tokens(member, sources, file_languages))
    else {
        return true;
    };
    members.iter().skip(1).all(|member| {
        member_value_tokens(member, sources, file_languages).map_or(true, |values| values == first)
    })
}

/// Extracts value tokens for one report member when its language is supported.
fn member_value_tokens<S: BuildHasher>(
    member: &Fingerprint,
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
) -> Option<Vec<Vec<u8>>> {
    let language = file_languages.get(&member.file_id)?;
    let source = sources.get(&member.file_id)?;
    match *language {
        "csharp" => csharp_value_tokens(source, member.byte_range),
        _ => None,
    }
}

/// Parses a C# source file and extracts value tokens inside `range`.
fn csharp_value_tokens(source: &[u8], range: ByteRange) -> Option<Vec<Vec<u8>>> {
    let language = tree_sitter_c_sharp::LANGUAGE.into();
    let tree = parse_source("csharp", &language, source).ok()?;
    let mut values = Vec::new();
    collect_csharp_values(tree.root_node(), source, range, &mut values);
    Some(values)
}

/// Walks C# syntax, collecting literals and member-access names.
fn collect_csharp_values(node: Node<'_>, source: &[u8], range: ByteRange, out: &mut Vec<Vec<u8>>) {
    if outside_range(node, range) {
        return;
    }
    if csharp::is_literal_kind(node.kind()) {
        push_node_text(b'L', node, source, out);
    } else if node.kind() == "member_access_expression" {
        push_member_name(node, source, out);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_csharp_values(child, source, range, out);
    }
}

/// Adds the final named child of a `member_access_expression`.
fn push_member_name(node: Node<'_>, source: &[u8], out: &mut Vec<Vec<u8>>) {
    let mut cursor = node.walk();
    let mut last = None;
    for child in node.named_children(&mut cursor) {
        last = Some(child);
    }
    if let Some(child) = last {
        push_node_text(b'M', child, source, out);
    }
}

/// Adds one prefixed source slice to the token list.
fn push_node_text(prefix: u8, node: Node<'_>, source: &[u8], out: &mut Vec<Vec<u8>>) {
    if let Some(bytes) = source.get(node.start_byte()..node.end_byte()) {
        let mut token = Vec::with_capacity(bytes.len().saturating_add(1));
        token.push(prefix);
        token.extend_from_slice(bytes);
        out.push(token);
    }
}

/// Returns true when `node` cannot overlap `range`.
fn outside_range(node: Node<'_>, range: ByteRange) -> bool {
    node.end_byte() <= range.start || node.start_byte() >= range.end
}
