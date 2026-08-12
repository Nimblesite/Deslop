# Language Roadmap

Parser rollout status and remaining language work.

## [LANG-SHIPPED] Shipped baseline

C#, Rust, Python, JavaScript/TypeScript (including JSX/TSX), Dart, PHP, F#, and Go (P-LANG-3, shipped 2026-07-26) parse, normalize, and cluster on the tree-sitter 0.26.8 runtime with modern `LanguageFn` grammar pins (P-LANG-0) and Rust / Python AST-golden coverage. The durable contract per language is [pipeline.md](../specs/pipeline.md) plus the per-language fixture suites under `crates/deslop/tests/`.

## TODO

- [ ] **[LANG-PHP-WIRING] PHP filter parity** — PHP parses and clusters but has no language-specific cluster filters (`crates/deslop-core/src/cluster_filters/` carries ECMAScript, Python, Rust, and Dart filters; nothing for PHP). Bring PHP noise filtering to parity with the other shipped languages.
- [ ] **P-LANG-5 Java** — the next planned language slice.
