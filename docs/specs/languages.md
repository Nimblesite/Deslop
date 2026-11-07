# Language contracts

The parser interface is [PIPELINE-LANG-TRAIT]; shared normalization follows [PIPELINE-NORMALIZE-AST]. These contracts preserve the shipped language behavior formerly documented in the language roadmap. Grammar versions remain defined in `Cargo.toml`.

## [LANG-CAND-OSPREY] Osprey

Files ending in `.osp` use Osprey's native default-flavor tree-sitter grammar. Identifier leaves collapse to `__ident__`; numbers, booleans, strings, string contents and escape sequences collapse to `__literal__`. Line, documentation and inner documentation comments disappear. Qualified names, type wrappers, effects, handlers, patterns and expression structure remain. String interpolation retains its expression descendants and operator fields; it must never become an opaque literal. Imports are scaffolding rather than copied implementation.

All three source flavors have separate registered parser identities: `osprey`, `osprey-ml` and `osprey-python`. Native ML `.ospml` uses its own grammar, verified against the upstream source corpus and structural checks before registration. ML block comments are trivia; layout, interpolation and flat-versus-curried syntax remain structural. A parse containing error or missing nodes must return an error before ML normalization.

**[LANG-CAND-OSPREY-UNPARSED]** A file that cannot be analysed, because its parse has errors or it nests too deeply, is skipped and the scan carries on. One broken file, including one half-typed in the editor, never stops a CLI run or the live server. Nothing is hidden: the report lists every skipped file and why in `files_skipped`, the CLI and the text and HTML reports print them, the VS Code Session panel counts them, and they are left out of `files_analysed` and the duplication percentage. A skipped file returns to the analysis as soon as it parses (pinned by `malformed_osprey_ml_is_skipped_visibly_and_the_scan_continues`).

Ordinary `.py` files remain Python unless a leading parsed comment `# osprey: flavor=python`, an explicit `[project].python_sources` path or a Python `[project].entry` opts them into Osprey. These paths are literal file paths relative to `osprey.toml`, not glob patterns. Project `flavor` accepts `default`, `ml` or `python` and overrides syntax for claimed sources. Without that override, a contradictory or unknown flavor marker in an `.osp` or `.ospml` file is an explicit error, and any marker other than `flavor=python` in an unclaimed `.py` file is an ordinary comment, so the file stays Python (pinned by `foreign_markers_leave_ordinary_python_analysable`). An `osprey.toml` that cannot be read or parsed is an explicit error for the `.osp` and `.ospml` files under its directory, and only for them: the scan of anything else continues (pinned by `broken_project_manifests_leave_ordinary_python_analysable` and `broken_nested_manifests_leave_sources_outside_them_analysable`). An unknown project `flavor` is an error for the sources that project claims. An `osprey.toml` under a path the exclusion config excludes is outside the analysed workspace: it can neither fail the scan nor select a flavor (pinned by `excluded_project_manifests_never_block_or_steer_the_scan`).

Project source roots default to `src` when that directory exists, otherwise the manifest directory; explicit `source_roots` replaces that default. Nested projects take precedence. Saved manifest changes must flow through watcher, scheduler, session, broadcast and UI. Report occurrences and file metrics retain the selected parser identity under [REPORT-CONTEXT-CLUSTER]. Cross-flavor clone equivalence is not established by sharing a display name or normalization helper.

Implemented by `crates/deslop-core/src/lang/osprey.rs` and `osprey_selection.rs`; exercised through the CLI in `crates/deslop/tests/cli/detection/osprey.rs` and `osprey_flavors.rs`, with real LSP/MCP coverage in `crates/deslop-mcp/tests/languages/osprey.rs`. Development status and delivery constraints live in [the integration plan](../plans/osprey-support.md). Osprey receives a gold bird icon and prominent registry placement under [VSIX-LANGUAGE-ICONS]; language identity never alters clone scores or ranking. Osprey's Python parser does not inherit Python refactoring actions.

## [LANG-CAND-C] C

Files ending in `.c` and `.h` use the C grammar. Headers are claimed as C; a C++ header is outside the contract until C++ is registered. Identifier leaves — `identifier`, `field_identifier`, `type_identifier`, `statement_identifier` and `primitive_type` — collapse to `__ident__`, so renaming a function, a struct member, a typedef, a `goto` label or swapping `int` for `long` never hides a clone. `number_literal`, `character`, `string_content`, `escape_sequence`, `system_lib_string`, `true`, `false` and `null` collapse to `__literal__`; the `string_literal`, `char_literal` and `concatenated_string` wrappers stay structural, so adjacent-string concatenation keeps a shape distinct from one literal. Comments disappear. Pointer, array and function declarators, casts, `sizeof`, compound literals, initializer lists, designators, storage classes, type qualifiers and every statement form remain structural.

**[LANG-CAND-C-PREPROC]** The grammar never parses what a macro expands to: `#define` bodies are opaque `preproc_arg` leaves, so two unrelated macros would otherwise fingerprint identically. `preproc_include`, `preproc_def` and `preproc_function_def` are therefore import-style scaffolding ([PIPELINE-BOILERPLATE-FILTER]): they are never clone evidence and never appear in a cluster on their own. Conditional blocks (`preproc_if`, `preproc_ifdef`, `preproc_elif`, `preproc_else`) contain real code and stay structural. Duplicated macro bodies are a known, documented false negative, not a tolerated false positive. A header's include guard wraps the whole file in one `preproc_ifdef`; the guard is never evidence on its own because its `#define` is scaffolding and the declarations inside it are compared on their own shape.

**[LANG-CAND-C-PROTOTYPE]** A function prototype — a `declaration` whose declarator is a `function_declarator`, possibly behind a `pointer_declarator` — is scaffolding ([PIPELINE-BOILERPLATE-FILTER]). After normalisation every one-argument prototype has the same shape, so a header listing `sqrt`, `exp` and `log` would otherwise cluster with any header listing three unrelated functions; Osprey's `klee/include/math.h` showed exactly that. The prototype promises a body that lives elsewhere, and the body is the evidence, so prototypes never appear in a cluster and a header made only of an include guard and prototypes reports nothing.

Function bodies are the function-scope unit for the signature-only filters: `function_definition` exposes a `body` field ([CLONE-NOISE-SIGNATURE-ONLY]).

Implemented by `crates/deslop-core/src/lang/c.rs`; exercised through the CLI in `crates/deslop/tests/cli/detection/c.rs` with the `c-small`, `c-type3`, `c-dissimilar-functions`, `c-prologue-false-positive` and `c-headers` fixtures, and the C AST golden in `cli/cache_and_debug.rs`. VS Code activation, editor synchronization and JetBrains file matching register `c` and `h`.

## [LANG-CAND-DART] Dart

Files ending in `.dart` use the Dart grammar. Identifier leaves collapse to `__ident__`; numeric, boolean, null, symbol, and string literals collapse to `__literal__`; comments disappear. Type wrappers, interpolation expressions, records, and patterns retain their structure. Renamed declaration signatures are scaffolding, not proof of copied bodies.

Implemented by `crates/deslop-core/src/lang/dart.rs`; exercised by `crates/deslop/tests/dart_signatures.rs`, `dart_issue_119_embedding_role_mismatch.rs`, and the Dart golden in `cli/cache_and_debug.rs`.

## [LANG-CAND-GO] Go

Files ending in `.go` use the Go grammar. Identifier leaves, including package, type, field, blank, and label identifiers, collapse to `__ident__`. Constants and string contents collapse to `__literal__`; comments disappear. Composite literals and function literals retain their structure. The mandatory package clause is excluded from whole-file clone views under [PIPELINE-FINGERPRINT-MERKLE-ROOT].

Implemented by `crates/deslop-core/src/lang/go.rs`; exercised by the Go CLI detection and AST-golden cases in `crates/deslop/tests/cli/`, including closure-body and package-clause coverage.

## [LANG-CAND-JAVASCRIPT] JavaScript and JSX

`.js`, `.mjs`, `.cjs`, and `.jsx` files use the JavaScript grammar. JavaScript shares its normalization table with TypeScript: identifier leaves collapse, literals and textual JSX content collapse, and comments and hashbangs disappear. Qualified names and template substitutions retain their structure. JSX uses the JavaScript grammar rather than the TypeScript grammar.

Implemented by `lang/javascript.rs` and `lang/ecmascript.rs` in `deslop-core`; exercised by `js_language_features.rs`, `js_ts_extensions.rs`, `jsx_tsx_components.rs`, and the JavaScript/JSX AST goldens in `deslop`'s CLI suite.

## [LANG-CAND-TYPESCRIPT] TypeScript and TSX

`.ts` files use the TypeScript grammar; `.tsx` files use its TSX entry point. Both share the JavaScript normalization table. Identifier and literal spelling is erased while type annotations, qualified names, JSX structure, and interpolated expressions remain structural.

Implemented by `lang/typescript.rs` and `lang/ecmascript.rs` in `deslop-core`; exercised by `typescript_features.rs`, `jsx_tsx_components.rs`, `js_ts_signatures.rs`, and the TypeScript/TSX AST goldens in `deslop`'s CLI suite.

## [LANG-CAND-KOTLIN] Kotlin is not registered

Kotlin is not a supported parser in [PIPELINE-LANG-TRAIT]. A live `find_similar` request naming Kotlin must return `UnsupportedLanguage`, rather than an empty match list. `crates/deslop-core/tests/live.rs` pins that refusal; adding Kotlin requires a parser and the same fixture coverage as a shipped language.

## [PARSE-FSHARP-NORMALIZE] F# normalization

`.fs` and `.fsx` use the F# source grammar. Identifier and operator-identifier leaves collapse to `__ident__`; numeric, character, boolean, unit, and string literals collapse to `__literal__`. Line comments, block comments, and XML documentation disappear. Qualified names, interpolation expressions, active patterns, computation expressions, pipelines, units of measure, and quotations retain their structure.

Implemented by `crates/deslop-core/src/lang/fsharp.rs`; the F# AST golden in `crates/deslop/tests/cli/cache_and_debug.rs` pins normalized output.

## [PARSE-PHP-NORMALIZE] PHP normalization

`.php` files use the PHP grammar. Name leaves collapse to `__ident__`, including the name inside a variable. Numeric, boolean, null, and string-family nodes collapse to `__literal__`; their children still undergo normalization. Comments and documentation comments disappear; remaining named nodes retain their structure.

Implemented by `crates/deslop-core/src/lang/php.rs`; the PHP AST golden in `crates/deslop/tests/cli/cache_and_debug.rs` pins normalized output.
