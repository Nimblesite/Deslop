# Language Roadmap

Parser rollout status and remaining language work.

## [LANG-SHIPPED] Shipped baseline

C#, Rust, Python, JavaScript/TypeScript (including JSX/TSX), Dart, PHP, F#, and Go ([LANG-GO], shipped 2026-07-26) parse, normalize, and cluster on the tree-sitter 0.26.8 runtime with modern `LanguageFn` grammar pins ([LANG-GRAMMAR-PINS]) and Rust / Python AST-golden coverage. The durable contract per language is [PARSE-LANGUAGES] in [pipeline.md](../specs/pipeline.md); the parser registry is `crates/deslop-core/src/lang/mod.rs` and the per-language fixture suites sit under `crates/deslop/tests/`.

## TODO

- [ ] **[LANG-PHP-WIRING] PHP filter parity** — PHP parses and clusters but has no language-specific cluster filters (`crates/deslop-core/src/cluster_filters/` carries ECMAScript, Python, Rust, and Dart filters; nothing for PHP). Bring PHP noise filtering to parity with the other shipped languages.
- [ ] **[LANG-JAVA] Java** — the next planned language slice: a `LanguageParser` in `crates/deslop-core/src/lang/`, a normalization table under [PARSE-LANGUAGES], and a fixture suite under `crates/deslop/tests/`.
