# Language Roadmap

Parser rollout status and remaining language work.

## [LANG-SHIPPED] Shipped baseline

C#, Rust, Python, JavaScript/TypeScript (including JSX/TSX), Dart, PHP, F#, and Go parse, normalize, and cluster through [PIPELINE-LANG-TRAIT]. Grammar pins remain in `Cargo.toml`. The restored per-language contracts are in [languages.md](../specs/languages.md); the parser registry is `crates/deslop-core/src/lang/mod.rs` and the fixture suites sit under `crates/deslop/tests/`.

## TODO

- [ ] **[LANG-PHP-WIRING] PHP filter parity** — PHP parses and clusters but has no language-specific cluster filters (`crates/deslop-core/src/cluster_filters/` carries ECMAScript, Python, Rust, and Dart filters; nothing for PHP). Bring PHP noise filtering to parity with the other shipped languages.
- [ ] **[LANG-JAVA] Java** — the next planned language slice: a `LanguageParser` under [PIPELINE-LANG-TRAIT], a contract in [languages.md](../specs/languages.md), and a fixture suite under `crates/deslop/tests/`.
