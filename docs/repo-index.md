# Repo index

Reference material for agents. Read it when you need to locate something — it is not required reading.

## Paths

| Path | Role |
|---|---|
| `crates/deslop-core` | Analysis library. Everything non-trivial lives here. |
| `crates/deslop` | Thin CLI binary, <50 LOC of glue. Cold-cache path for CI gates and one-shot audits. |
| `crates/deslop-lsp` | Streams live clone warnings to any LSP-capable editor. |
| `crates/deslop-mcp` | Lets agents query the running analysis mid-generation, before a paste happens. |
| `clients/vscode` | VSIX; bundles the LSP + MCP binaries. |
| `site/` | Eleventy static site. Zero duplicate CSS, hard budget 1.8k LOC. |
| `docs/specs/SPEC.md` | Full research + design spec. |
| `docs/plans/PLAN.md` | Phased execution plan, live TODO at the bottom. |
| `docs/snippets/agents-md-recipe.md` | Paste-ready Rule-zero recipe for other repos. |
| `.claude/skills/` | ci-prep, code-dedup, fix-bug, submit-pr. |
| `coverage-thresholds.json` | Single source of truth for coverage. |
| `Cargo.toml` | Workspace + strict lints. |
| `.github/workflows/ci.yml` · `.devcontainer/` | CI and dev container; dependency versions mirror `Cargo.toml`. |

## Pipeline stages

- **`LanguageParser`** is the single extension point. New language = implement the trait + pin the grammar in `Cargo.toml`, CI, and Dockerfile.
- **Normalization** strips identifiers, literals, and trivia so renamed-clone (Type-2) detection works. Per-language rules, identical output format across languages.
- **Fingerprinting** operates on AST subtrees, not lines. Minimum node count is configurable.
- **Ranking** weights clone size × clone count × spanned LOC, worst offenders first.

Languages: C#, Rust, Python, Dart, JavaScript, TypeScript/TSX, PHP, F#, Go. Detection and ranking ship today; dedup actions are roadmap.

## Migration to `lspkit`

The LSP+MCP scaffolding here is the "one engine, two surfaces" pattern being distilled into the sibling `lspkit-*` workspace. Analysis code — parsing, fingerprinting, clustering, ranking, embeddings — is accuracy-critical and stays here; only protocol shells move. Prefer `lspkit-*` for new infrastructure; when patching existing scaffolding, flag `lspkit` overlap in the PR and reference the upstream crate.

| Current path | Toolkit crate |
|---|---|
| `live/api.rs` `LiveApi` | `lspkit::EngineApi` — the headline contract |
| `live/session.rs` `AnalysisSession` + `LiveService` | `lspkit-live::Session` + consumer `Analyzer` |
| `live/watcher.rs` | `lspkit-live::watcher::FileWatcher` |
| `live/scheduler.rs` | `lspkit-live::scheduler::spawn` |
| `deslop-lsp/src/main.rs` + `backend.rs` (tower-lsp) | `lspkit-server` — the toolkit does not depend on `tower-lsp` (unmaintained) |
| `deslop-mcp/src/server.rs` + `protocol.rs` | `lspkit-mcp` (rmcp behind a newtype wall) |
| `deslop-mcp/src/tools/mod.rs` | `lspkit-mcp::tools::ToolRegistry` |
| `config.rs` `.deslop.toml` loader | `lspkit-config::load_from_ancestor` |
| `deslop-lsp/src/observability.rs` | `lspkit::tracing_setup::TracingBuilder` |

`state.rs` `FileRegistry` and the `LiveBackend` query path stay engine-internal. Nothing here is being removed — this repo stays canonical until the toolkit matures.
