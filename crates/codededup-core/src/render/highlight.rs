//! Server-side syntax highlighter for HTML snippets.
//!
//! Walks the tree-sitter parse of a snippet (one of C# / Rust / Python)
//! and emits `<span class="tok-XXX">` runs with HTML-escaped source
//! between them. No JavaScript, no external assets — the rendered HTML
//! is fully self-contained per [OUTPUT-HUMAN-HTML].
//!
//! Token classes are deliberately coarse: `keyword`, `type`, `string`,
//! `number`, `comment`, `function`, `attribute`, `operator`,
//! `punctuation`, `identifier`. Anything we don't recognise falls
//! through as plain escaped text.

use std::fmt::Write as _;

use tree_sitter::{Node, Parser};

/// Renders `source` as a syntax-highlighted HTML fragment for `language`
/// (`"csharp"`, `"rust"`, `"python"`). Falls back to plain escaped text
/// when the language is unknown or the parser cannot be initialised —
/// the renderer never panics on a snippet.
#[must_use]
pub fn highlight_snippet(source: &str, language: &str) -> String {
    let Some(grammar) = grammar_for(language) else {
        return escape(source);
    };
    let mut parser = Parser::new();
    if parser.set_language(&grammar).is_err() {
        return escape(source);
    }
    let bytes = source.as_bytes();
    let Some(tree) = parser.parse(bytes, None) else {
        return escape(source);
    };
    let spans = collect_spans(tree.root_node(), bytes, language);
    render_spans(source, &spans)
}

/// One coloured run produced by the highlighter. Byte offsets refer to
/// the snippet, not the original file.
#[derive(Debug, Clone, Copy)]
struct Span {
    /// Inclusive start byte within the snippet.
    start: usize,
    /// Exclusive end byte within the snippet.
    end: usize,
    /// CSS class suffix (`"keyword"` becomes `class="tok-keyword"`).
    class: &'static str,
}

/// Maps a `language` id to its tree-sitter grammar. `None` for unknown
/// languages so the caller falls back to plain escape.
fn grammar_for(language: &str) -> Option<tree_sitter::Language> {
    match language {
        "csharp" => Some(tree_sitter_c_sharp::language()),
        "rust" => Some(tree_sitter_rust::language()),
        "python" => Some(tree_sitter_python::language()),
        _ => None,
    }
}

/// Walks the parse tree depth-first and emits one [`Span`] per leaf
/// node whose kind maps to a recognised colour class. Non-leaf nodes
/// only contribute spans through their children.
fn collect_spans(root: Node<'_>, source: &[u8], language: &str) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    let mut stack: Vec<Node<'_>> = vec![root];
    while let Some(node) = stack.pop() {
        if node.child_count() == 0 {
            if let Some(class) = leaf_class(&node, source, language) {
                out.push(Span {
                    start: node.start_byte(),
                    end: node.end_byte(),
                    class,
                });
            }
            continue;
        }
        let mut cursor = node.walk();
        let mut children: Vec<Node<'_>> = node.children(&mut cursor).collect();
        children.reverse();
        stack.extend(children);
    }
    out.sort_by_key(|s| s.start);
    out.dedup_by(|a, b| a.start == b.start && a.end == b.end);
    out
}

/// Maps a leaf tree-sitter node to a CSS class. Returns `None` for
/// uninteresting leaves (whitespace, structural punctuation we don't
/// want coloured).
fn leaf_class(node: &Node<'_>, source: &[u8], language: &str) -> Option<&'static str> {
    let kind = node.kind();
    if let Some(common) = common_class(kind) {
        return Some(common);
    }
    match language {
        "csharp" => csharp_class(node, kind, source),
        "rust" => rust_class(node, kind, source),
        "python" => python_class(node, kind, source),
        _ => None,
    }
}

/// Cross-language tokens that always map to the same class regardless
/// of grammar (string / number / comment literals, basic operators).
fn common_class(kind: &str) -> Option<&'static str> {
    match kind {
        "string_literal"
        | "verbatim_string_literal"
        | "interpolated_string_text"
        | "string_content"
        | "raw_string_literal"
        | "string"
        | "string_fragment"
        | "character_literal"
        | "char_literal"
        | "byte_string_literal"
        | "\""
        | "'" => Some("string"),
        "integer_literal" | "real_literal" | "float_literal" | "integer" | "float" => {
            Some("number")
        }
        "comment" | "line_comment" | "block_comment" => Some("comment"),
        "boolean_literal" | "true" | "false" | "null_literal" | "none" => Some("keyword"),
        _ => None,
    }
}

/// C#-specific leaf classes. Identifies keywords by their literal
/// node-kind string (tree-sitter-c-sharp uses the keyword text as the
/// kind for keywords).
fn csharp_class(node: &Node<'_>, kind: &str, source: &[u8]) -> Option<&'static str> {
    if is_csharp_keyword(kind) {
        return Some("keyword");
    }
    if kind == "identifier" {
        return identifier_class(node, source);
    }
    if kind == "predefined_type" {
        return Some("type");
    }
    operator_class(kind)
}

/// Rust-specific leaf classes.
fn rust_class(node: &Node<'_>, kind: &str, source: &[u8]) -> Option<&'static str> {
    if is_rust_keyword(kind) {
        return Some("keyword");
    }
    if kind == "primitive_type" {
        return Some("type");
    }
    if kind == "identifier" {
        return identifier_class(node, source);
    }
    operator_class(kind)
}

/// Python-specific leaf classes.
fn python_class(node: &Node<'_>, kind: &str, source: &[u8]) -> Option<&'static str> {
    if is_python_keyword(kind) {
        return Some("keyword");
    }
    if kind == "identifier" {
        return identifier_class(node, source);
    }
    operator_class(kind)
}

/// Decides between `function`, `type`, and `identifier` for a bare
/// identifier leaf based on its parent context. Cheap heuristic, no
/// scope analysis.
fn identifier_class(node: &Node<'_>, source: &[u8]) -> Option<&'static str> {
    let parent_kind = node.parent().map(|p| p.kind()).unwrap_or_default();
    if matches!(
        parent_kind,
        "function_definition"
            | "function_declaration"
            | "method_declaration"
            | "function_item"
            | "call"
            | "call_expression"
            | "invocation_expression"
    ) {
        return Some("function");
    }
    let text = std::str::from_utf8(source.get(node.start_byte()..node.end_byte())?).ok()?;
    if text
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
    {
        return Some("type");
    }
    Some("identifier")
}

/// Maps single-character operator / punctuation kinds.
fn operator_class(kind: &str) -> Option<&'static str> {
    match kind {
        "+" | "-" | "*" | "/" | "%" | "=" | "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&"
        | "||" | "!" | "&" | "|" | "^" | "~" | "+=" | "-=" | "*=" | "/=" | "%=" | "=>"
        | "->" | "::" | "?" | "??" | "?." => Some("operator"),
        "(" | ")" | "[" | "]" | "{" | "}" | ";" | "," | ":" | "." => Some("punctuation"),
        _ => None,
    }
}

/// Renders `spans` against `source` into an HTML fragment. Bytes
/// between spans pass through HTML-escaped without a wrapping `<span>`.
fn render_spans(source: &str, spans: &[Span]) -> String {
    let mut out = String::with_capacity(source.len().saturating_mul(2));
    let bytes = source.as_bytes();
    let mut cursor = 0_usize;
    for span in spans {
        if span.start < cursor || span.end > bytes.len() || span.start >= span.end {
            continue;
        }
        if let Some(prefix) = bytes.get(cursor..span.start) {
            push_escaped(&mut out, prefix);
        }
        let _ = write!(out, "<span class=\"tok-{class}\">", class = span.class);
        if let Some(slice) = bytes.get(span.start..span.end) {
            push_escaped(&mut out, slice);
        }
        out.push_str("</span>");
        cursor = span.end;
    }
    if let Some(tail) = bytes.get(cursor..) {
        push_escaped(&mut out, tail);
    }
    out
}

/// Appends `bytes` to `out`, escaping the four HTML-significant
/// characters and dropping any invalid UTF-8 silently — snippets come
/// straight from disk and may not always be valid UTF-8.
fn push_escaped(out: &mut String, bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    out.push_str(&escape(&text));
}

/// HTML-escapes `input`. Mirrors the tiny escape helper in
/// [`super::html`] to avoid a cross-module dep that has nothing to do
/// with rendering.
fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

/// Returns true when `kind` matches a C# keyword as reported by
/// `tree-sitter-c-sharp` (which uses the keyword text as the node kind).
fn is_csharp_keyword(kind: &str) -> bool {
    matches!(
        kind,
        "abstract"
            | "as"
            | "async"
            | "await"
            | "base"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "default"
            | "delegate"
            | "do"
            | "else"
            | "enum"
            | "event"
            | "explicit"
            | "extern"
            | "finally"
            | "fixed"
            | "for"
            | "foreach"
            | "get"
            | "goto"
            | "if"
            | "implicit"
            | "in"
            | "init"
            | "interface"
            | "internal"
            | "is"
            | "lock"
            | "namespace"
            | "new"
            | "operator"
            | "out"
            | "override"
            | "params"
            | "partial"
            | "private"
            | "protected"
            | "public"
            | "readonly"
            | "record"
            | "ref"
            | "return"
            | "sealed"
            | "set"
            | "sizeof"
            | "stackalloc"
            | "static"
            | "struct"
            | "switch"
            | "this"
            | "throw"
            | "try"
            | "typeof"
            | "unsafe"
            | "using"
            | "var"
            | "virtual"
            | "void"
            | "volatile"
            | "where"
            | "while"
            | "yield"
    )
}

/// Returns true when `kind` matches a Rust keyword.
fn is_rust_keyword(kind: &str) -> bool {
    matches!(
        kind,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "yield"
    )
}

/// Returns true when `kind` matches a Python keyword.
fn is_python_keyword(kind: &str) -> bool {
    matches!(
        kind,
        "and" | "as"
            | "assert"
            | "async"
            | "await"
            | "break"
            | "class"
            | "continue"
            | "def"
            | "del"
            | "elif"
            | "else"
            | "except"
            | "finally"
            | "for"
            | "from"
            | "global"
            | "if"
            | "import"
            | "in"
            | "is"
            | "lambda"
            | "nonlocal"
            | "not"
            | "or"
            | "pass"
            | "raise"
            | "return"
            | "try"
            | "while"
            | "with"
            | "yield"
    )
}
