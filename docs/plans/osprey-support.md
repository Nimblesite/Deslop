# Osprey integration

## Native default syntax

Implement [LANG-CAND-OSPREY] with Osprey's tree-sitter grammar for `.osp`, preserving effects, handlers, patterns and interpolation expressions through the shared discovery, cache, incremental session, ranking and rendering paths. Verify through the CLI fixtures and the real LSP/MCP language-label contract. Language registration must also reach VS Code activation, editor synchronization and JetBrains file matching.

Both native grammar dependencies use the published Osprey revision `865f0469acaec12ba7e448491fc9ec2cc877d57c` in `Cargo.toml`. Builds must resolve these dependencies without a sibling Osprey checkout. Remote deployment is deferred by the user.

## All three source flavors

The user requires default `.osp`, ML `.ospml`, and Osprey's Python-looking `.py` flavor. Default and ML need their own native tree-sitter grammars; Python flavor uses the same Python CST as Osprey's compiler, with a separate Osprey parser identity and no inherited Python refactoring actions. A leading flavor marker or explicit project `python_sources`/Python entry opts `.py` into Osprey; ordinary Python remains Python. Project `flavor` overrides syntax, while contradictory markers and extensions without an override must produce a visible error.

Keep parser/cache identities separate (`osprey`, `osprey-ml`, `osprey-python`) and carry them on occurrence and per-file report rows under [REPORT-CONTEXT-CLUSTER]. UI surfaces display one Osprey family and official Osprey icon. Saved `osprey.toml` changes must reach watcher, scheduler, session, broadcast and refreshed UI. Black-box contracts live in `crates/deslop/tests/cli/detection/osprey_flavors.rs`. The native ML grammar must pass its real source corpus before registration; the mechanically emitted corpus result is `target/osprey-work/ml-corpus-batch.md`.

## Royal presentation

Implement [VSIX-LANGUAGE-ICONS] with one shared artwork source for native tree language groups, files, occurrences and metrics, plus all three webviews. Give Osprey the leading registry position and the official cyan Osprey icon. Keep accessible language names and neutral fallback artwork. Preserve the engine's ranking and duplication calculations unchanged. Verify the registry, native icon consumers and browser rendering tests.

Update the supported-language documentation and add `site/src/blog/osprey-first-class.md`. Build the site and type-check the article's Osprey program. The article must state the verified flavor boundary and make no cross-flavor clone-equivalence or unmeasured accuracy claims.
