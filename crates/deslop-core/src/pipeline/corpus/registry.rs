//! The language-parser registry ([PIPELINE-LANG-TRAIT]): the closed set
//! of supported parsers and the lookups every surface derives from it.
//! Split from the parent module, which owns corpus assembly.

use std::{collections::HashMap, path::Path};

use crate::lang::LanguageParser;

/// Returns the parser whose `id()` matches `language`.
pub fn parser_for_language<'a>(
    parsers: &'a [Box<dyn LanguageParser>],
    language: &str,
) -> Option<&'a dyn LanguageParser> {
    parsers
        .iter()
        .find(|parser| parser.id() == language)
        .map(|boxed| &**boxed)
}

/// Returns the registered language parsers in a stable order
/// (implements [PIPELINE-LANG-TRAIT]).
#[must_use]
pub fn default_parsers() -> Vec<Box<dyn LanguageParser>> {
    use crate::lang::{
        csharp::CSharpParser,
        dart::DartParser,
        fsharp::FSharpParser,
        go::GoParser,
        javascript::JavaScriptParser,
        php::PhpParser,
        python::PythonParser,
        rust_lang::RustParser,
        typescript::{TsxParser, TypeScriptParser},
    };
    vec![
        Box::new(CSharpParser::new()),
        Box::new(RustParser::new()),
        Box::new(PythonParser::new()),
        Box::new(DartParser::new()),
        Box::new(JavaScriptParser::new()),
        Box::new(TypeScriptParser::new()),
        Box::new(TsxParser::new()),
        Box::new(PhpParser::new()),
        Box::new(FSharpParser::new()),
        Box::new(GoParser::new()),
    ]
}

/// Stable language ids of every registered parser, in registry order.
/// Single source of truth for any surface that needs the closed set of
/// supported languages — tool schemas, language filters, docs — so the list
/// can never drift from [`default_parsers`] ([PIPELINE-LANG-TRAIT]).
#[must_use]
pub fn language_ids() -> Vec<&'static str> {
    default_parsers().iter().map(|parser| parser.id()).collect()
}

/// Detected display language id for a source path, derived from the parser
/// registry's declared extensions, or `"unknown"`. The single labeling map
/// shared by every human/agent surface (the HTML report highlighter, MCP page
/// summaries) so the detected language can never drift between them — or from
/// the registry when a language is added ([PIPELINE-LANG-TRAIT]).
#[must_use]
pub fn language_for_path(path: &Path) -> &'static str {
    let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
        return "unknown";
    };
    default_parsers()
        .iter()
        .find(|parser| {
            parser
                .file_extensions()
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(extension))
        })
        .map_or("unknown", |parser| parser.id())
}

/// Source-file extensions of every registered parser, in registry order.
/// Single source of truth for any surface that filters filesystem events
/// by extension — e.g. the LSP live watcher — so the watched set can
/// never drift from [`default_parsers`] ([PIPELINE-LANG-TRAIT]).
#[must_use]
pub fn watched_source_extensions() -> Vec<&'static str> {
    default_parsers()
        .iter()
        .flat_map(|parser| parser.file_extensions().iter().copied())
        .collect()
}

/// Builds a lowercase-extension → language-id lookup from the parser
/// registry. Returning the language id (not a parser index) lets
/// [`crate::discover::discover_files`] check [`crate::config::ExclusionConfig`]
/// before the parser is selected.
#[must_use]
pub fn build_extension_map(parsers: &[Box<dyn LanguageParser>]) -> HashMap<String, &'static str> {
    let mut out: HashMap<String, &'static str> = HashMap::new();
    for parser in parsers {
        for extension in parser.file_extensions() {
            let _previous = out.insert((*extension).to_lowercase(), parser.id());
        }
    }
    out
}
