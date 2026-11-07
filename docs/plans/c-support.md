# C language support

Implements [LANG-CAND-C] and [LANG-CAND-C-PREPROC]. Template: the Go slice (`lang/go.rs`, PR #311). Grammar: `tree-sitter-c = "=0.24.2"`, pinned in `Cargo.toml` and `.github/workflows/ci.yml`.

## Checklist

- [ ] **Engine** — `crates/deslop-core/src/lang/c.rs` (`CParser`, id `c`, extensions `c`/`h`), registered in `pipeline/corpus/registry.rs`; `boilerplate.rs` carrier row (`preproc_include`, `preproc_def`, `preproc_function_def`) and mandated prologue (include guard `preproc_ifdef` wrapper is **not** mandated — only the `preproc_include` carrier); `cluster_filters/mod.rs` function-scope row (`function_definition`); `cluster_filters/snippets.rs` grammar row; `render/html.rs` display name; `lang/shared.rs` operator-leaf table row; `deslop-test-support` corpus data-table grammar row.
- [ ] **Tests** — fixtures under `crates/deslop/tests/fixtures/`: `c-small` (Type-2 rename), `c-type3` (near-miss), `c-dissimilar-functions` (must report nothing), `c-prologue-false-positive` (shared `#include`/`#define` prologue, no cluster), `c-headers` (`.h` claimed, include guard excluded from whole-file view), `ast-golden-c/Sample.c` + `.expected.ast`. Test module `crates/deslop/tests/cli/detection/c.rs`. Registry parity test `lang_registry_vsix_parity.rs` extends automatically.
- [ ] **Editors** — `clients/vscode/src/types/languages.ts` (`c`, `h` → `c`; display `C`; editor id `c`), `languageIcons.ts`, `package.json` keywords/activation events; `clients/jetbrains/.../DeslopSupportedFiles.kt` + `plugin.xml` glob; `scripts/verify-jetbrains-package.mjs` if it lists extensions.
- [ ] **Docs** — spec section (done), `docs/plans/lang-roadmap.md` shipped list, `docs/specs/pipeline.md` v1 list, `site/src/docs/*` supported-language mentions, `site/src/blog/find-duplicate-code-c.md`.
- [ ] **Gates** — `make lint`, `make test`, deslop self-scan, CI green, PR squash-merged, release tag.

## Decisions

- `.h` is C. C++ headers parse with errors and are skipped visibly under [LANG-CAND-OSPREY-UNPARSED]-style skip reporting rather than silently misreported.
- `primitive_type` collapses with identifiers (parity with C# `predefined_type`, Rust `primitive_type`).
- Macro definitions are scaffolding, never clone evidence: the grammar cannot see inside them.
