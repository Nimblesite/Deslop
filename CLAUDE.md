<!-- agent-pmo:b636503 -->
# Deslop Live — Agent Instructions

## Rule zero — query the Deslop MCP before you write code

Deslop ships the duplicate-detector; its own code must be the cleanest in the
fleet. That only holds if **every agent queries the live Deslop MCP before
authoring code** — prevention, not cleanup.

**LAW — call `find-similar` BEFORE writing any code unit** (function, method,
class, helper, fixture, test setup, parser branch, error type, route handler,
view model — anything past a few lines). Pass a byte range (`path`,
`start_byte`, `end_byte`) or a snippet (`snippet`, `language`). Do NOT skip it
because the code "looks new" — most clones are written by someone certain it
was new.

- `signals.fused ≥ 0.85`, or bucket `identical` / `nearly_identical` → **do not
  write the copy.** Reuse the canonical occurrence; extract a helper if needed.
- `fused < 0.6` (empty or structurally distant) → author it.
- `0.6 ≤ fused < 0.85` → read the canonical occurrence and bias toward reuse.
- `structural_only` → shape-only match, often sibling boilerplate — read first.

**STOP DEAD if the MCP is unreachable or wrong.** If the Deslop MCP is
unavailable, errors, returns stale data, or gives an answer you can tell is
incorrect: **halt. Do not write code, do not guess, do not fall back to memory.**
A duplicate that lands because the gate was down is the exact failure this repo
exists to prevent. Tell the user and wait.

**LOG A GH ISSUE for every Deslop defect.** False positive, wrong bucket, stale
generation, missing cluster, MCP/IPC error — file it immediately with
`gh issue create` (include the cluster id or the triggering snippet). Never
silently work around a defect, widen thresholds, or mark clusters hidden. (`gh`
is the GitHub CLI, not `git` — the one allowed exception to the no-git rule.)

**Other Deslop tools:** dedup of *existing* duplicates → `top-offenders` then
`cluster-by-id`; a whole file → `report-for-file`; a block → `report-for-range`;
the JSON shapes → `schema-doc` once per session.

⚠️ **Never kill a VS Code process — including browser-hosted instances.** The user cannot recover from this; do not do it. ⚠️

⚠️ **Token discipline.** Check file size before reading. Prefer `Grep` over `Read`; use `offset`/`limit`. Make the smallest diff that solves the problem. Delete dead code, unused imports, and stale comments. Call out irrelevant context before proceeding — bloat degrades reasoning. ⚠️

⚠️ **Quality bar.** This codebase is held to an A+ standard: every change must pass review at a top-tier engineering organization (Google / Meta / Microsoft caliber). No exceptions, not even for a single line. Substandard code is fixed immediately, never deferred. ⚠️

⚠️ **"Deslop.live" (reactive) means the whole loop** (watcher → scheduler → session → broadcast → UI). An incremental update drives the entire pipeline, including a reactive UI refresh. ⚠️

⚠️ DO NOT USE GIT, ESPECIALLY NOT STAMPING YOURSELF AS COAUTHOR ⚠️

## Project Overview

**Deslop** (a.k.a. Deslop Live) is a **live duplicate-code analysis server** for AI coding agents and the humans driving them. The shipping surfaces are `deslop-lsp` (LSP server feeding live clone warnings to any LSP-capable editor) and `deslop-mcp` (MCP server letting Claude Code / Cursor / Copilot / Continue / Codex query the running analysis mid-generation, *before* a copy-paste happens). The `deslop` CLI is the cold-cache fallback for CI gates and one-shot audits. All three binaries are thin shells over one `deslop-core` library — the LSP and MCP sit in the agent's inner loop, the CLI re-uses the same engine for batch runs. Ranking is **worst offenders first** (highest weighted duplication impact at the top). Detection and ranking ship today; AI-assisted and mechanical deduplication actions are on the roadmap. Languages today: **C#, Rust, Python, and Dart**; TypeScript/JavaScript and Go are on the roadmap. Parsing is always tree-sitter — regex on source is prohibited.

## Prevention beats cure

The whole point of Deslop is that a duplicate never lands — see **Rule zero** at
the top of this file, which carries the `find-similar` LAW. Post-hoc scrubbing is
what every static analyzer already does; Deslop's edge is being live in the
agent's inner loop. Paste-ready recipe for other repos' `AGENTS.md` / `CLAUDE.md`:
see [docs/snippets/agents-md-recipe.md](docs/snippets/agents-md-recipe.md).

Full spec: [docs/specs/SPEC.md](docs/specs/SPEC.md). Execution plan and live TODO: [docs/plans/PLAN.md](docs/plans/PLAN.md).
- All spec sections have non-numeric, hierarchically structured IDs. All tests refer to spec IDs. All code refers to spec IDs.

**Primary language:** Rust
**Build command:** `make ci`
**Test command:** `make test`
**Lint command:** `make lint`

**There are 7 AgentPMO make targets; repo-specific targets must sit below a horizontal marker.** `make test` runs the test runner with its fail-fast flag, collects coverage, asserts measured ≥ threshold from `coverage-thresholds.json`, and exits non-zero on any failure. To debug a single test, invoke `cargo test <name> -- --nocapture` directly — that is not a Makefile target.

## UI

- The initial UI is a VSIX; IntelliJ and other plugins are in progress.
- Consistency across UI panels is critical.
- Do not duplicate the rendering of text or links (clusters, occurrences, etc.). Create a shared function and reuse it everywhere.
- On-screen output must be human-readable. The display is not for AI by default.
- Context menus must always include a "Copy Context For AI" item so users can feed context to AI directly.
- AI-targeted reports (the raw JSON file and reports generated from it) stay AI-targeted — they are not meant to be human-readable.

## Architecture

```
discover files → per-language parse (tree-sitter) → normalize AST →
fingerprint subtrees → cluster → token LSH → embeddings (hybrid) →
fuse signals → rank → render report
```

### IPC

Processes communicate over IPC. Generate IPC model code with [typeDiagram](https://typediagram.dev/docs/language-reference.html). Do not store generated model code in git — git-ignore it.

- **`crates/deslop-core`** — analysis library. Everything non-trivial lives here. The CLI, LSP, and MCP binaries all consume this single crate.
- **`crates/deslop`** — thin CLI binary (<50 LOC of glue): arg parsing, tracing setup, invoke core, render output.
- **`crates/deslop-lsp`** — LSP server surface; streams live clone warnings to any LSP-capable editor.
- **`crates/deslop-mcp`** — MCP server surface; lets agent tools query the running analysis mid-generation.
- **`clients/vscode`** — VS Code extension (VSIX) that bundles the LSP + MCP binaries and surfaces reports in-editor.
- **`LanguageParser` trait** is the single extension point. Adding a language = implementing the trait + pinning the grammar in `Cargo.toml`, CI, and Dockerfile.
- **Normalization** strips identifiers, literals, and trivia before fingerprinting so renamed-clone detection works (Type-2). Per-language rules, identical output format across languages.
- **Fingerprinting** operates on AST subtrees, not lines. Minimum node count is configurable.
- **Ranking score** weights clone size × clone count × spanned LOC — this is the user-visible product. Changes here change every report.
- **Global state** lives in exactly one file — Rust: `crates/deslop-core/src/state.rs`. Nothing escapes it. The same rule applies to TypeScript and any other language.

## Hard Rules — Universal (non-negotiable)

- **Files < 500 lines.** Refactor when over.
**Wire models use typeDiagram.** Every model sent across the wire (IPC) is generated from a [typeDiagram](https://typediagram.dev/docs/language-reference.html) definition — never a hand-written wire struct.
- **Act autonomously.** Do not stop mid-task for confirmation or approval. Choose the most reasonable default, record the assumption, and continue to completion.
- **No git commands.** No `add`, `commit`, `push`, `checkout`, `merge`, `rebase`, etc. CI handles git. (`gh issue create` for Deslop defects is the sole exception — see Rule zero.)
- **Git discipline:** never push to `main`, never list an AI/agent co-author, work on exactly one branch at a time, never start a new branch when a feature branch already exists, converge multiple feature branches before other work, and never use `git worktree`.
- **Auto-memory is off.** `.claude/settings.json` sets `"autoMemoryEnabled": false`; durable rules go through reviewed changes to this file.
- **Reduce code duplication — be aggressively DRY.** This tool detects duplication; its own codebase must be exemplary. Search before writing. Move code, don't copy.
- **Regex on source code or structured data is prohibited.** Use tree-sitter for all source parsing.
- **No exceptions for control flow.** Return `Result<T, E>`. Panics are bugs.
- **No placeholders.** No silent no-ops. Use proper error types.
- **Functions < 20 lines.**
- **Mandatory bug-fix process** — see the [fix-bug skill](.claude/skills/fix-bug/SKILL.md).
- **No legacy code.** Legacy is deleted.
- **Copying files is prohibited.** Move them.
- **The VSIX is the only legitimate distribution, and building must not install binaries to `PATH`.** `cargo build`, `make build`, `make ci`, and every other build target leave artifacts under `target/` only. There is no `make install-binary` target — `cargo install --path crates/deslop-*` is prohibited on this repo. The release pipeline ships binaries via the `.vsix` (and via Homebrew/Scoop for the CLI). Local builds are for testing the source you just changed; they are not a distribution channel.
- **External MCP clients (Claude Code, Claude Desktop, Codex, Cursor, Continue) must point at the VSIX-bundled binary by absolute path** — `~/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/<platform>/deslop-mcp` on Unix, the equivalent on Windows. The bare-name `deslop-mcp` (PATH lookup) form is only valid for users who installed the CLI via Homebrew/Scoop. A locally-built binary on `PATH` would shadow the shipwright-versioned bundle and silently drift the agent's analysis off the extension's wire contract. Every doc that shows an MCP config snippet uses the absolute VSIX path as the primary form.
- **Centralize all global state** in `crates/deslop-core/src/state.rs`.
- **Never delete a failing test, and never remove an assertion.** Reducing assertiveness is prohibited.
- **Coverage thresholds live in `coverage-thresholds.json` at the repo root** — not in environment variables, GitHub repo variables, or CI YAML. Falling below threshold fails the pipeline. The threshold only ratchets upward.
- **Coarse E2E tests only.** No unit tests. Drive the CLI end-to-end against fixture repos and assert against rendered reports.
- **Heavy structured logging.** See Logging below.
- **No linter suppressions.** `#[allow(clippy::...)]` is prohibited. Fix the underlying code.
- **Dependency versions in `Cargo.toml`, `.github/workflows/ci.yml`, and `.devcontainer/` stay in sync at all times.**
- **Spec IDs are hierarchical and non-numeric: `[GROUP-TOPIC]` / `[GROUP-TOPIC-DETAIL]`** (e.g. `[PARSE-CSHARP-NORMALIZE]`, `[RANK-SCORE]`). Same-group sections sit adjacent in the doc. No sequential numbers. Code and tests referencing a spec section include the ID in a comment so `grep [PARSE-` finds spec → code → tests.

## Hard Rules — Rust

- No `unwrap()`/`expect()` in production code (tests may `expect` with a message).
- No `panic!`/`todo!`/`unimplemented!`/`unreachable!`.
- No `unsafe {}`. Workspace lint is `unsafe_code = "deny"`.
- All public items have `///` doc comments (workspace lint: `missing_docs = "deny"`).
- `thiserror` for library errors in `deslop-core`. `anyhow` allowed in the `deslop` binary.
- Pattern matching over casting. Expressions over statements. Iterator chains over imperative loops.
- Early return with `?` for clean error propagation.
- Descriptive variable names — no single letters except in closures.

## Website

- Zero duplicate CSS.
- Hard CSS budget: 1.8k LOC.

## Logging Standards

- **`tracing` + `tracing-subscriber` only.** Never `println!`/`eprintln!` for diagnostics.
- **Log at entry and exit of significant operations.** Levels: `error|warn|info|debug|trace`.
- **Structured fields, not string interpolation.** `tracing::info!(file_count = 42, lang = "csharp")` — never format strings.
- **The CLI's report output is not a log.** Reports go to stdout (or `--output <path>`) via the renderer. Diagnostics go to `tracing`.
- **Never log file contents, paths containing user data, or secrets.** Log counts and hashes, not source.

## Testing Rules

- **Testing any UI/extension against a fake LSP/MCP is prohibited.** Tests must build and install the latest binaries before running.
- **`make test` is fail-fast.** It stops at the first failure. Never use `--no-fail-fast`.
- **`make test` always computes coverage and enforces it.** The threshold lives in `coverage-thresholds.json` at the repo root.
- **Aim for 100% coverage and a high mutation score.** Use many assertions per test.
- **Never delete a failing test, and never skip a test.** Add more failing tests for broken or missing functionality — never remove them.
- **Meaningful assertions only.** `assert!(true)` is not allowed.
- **No try/catch that swallows errors and then asserts success.**
- **Deterministic.** No `sleep`, no timing dependencies, no random state.
- **E2E tests are black-box only** — the CLI binary, fixture directories, rendered reports. Never reach into internals.
- The coverage threshold lives in `coverage-thresholds.json` and increases monotonically (−1% allowance for rounding).
- Do not write assertions that merely guard against AI-style labels. Instead, assert **positive, human-readable values**. 

## Human vs. AI Readability

There are two target audiences: AI and humans. What you write depends on who it is for.

- Code comments: humans first, AI second.
- UI (IDE extensions, CLI): humans, but with the ability to copy the context for AI.
- Formatted HTML reports: humans.
- Raw JSON reports: AI, but with enough information to reconstruct a human-readable version.

## Repo Structure

```
crates/
├── deslop-core/         # library: pipeline stages
│   └── src/
│       ├── lib.rs
│       └── state.rs        # single global-state file
├── deslop/              # thin CLI binary
├── deslop-lsp/          # LSP server binary
└── deslop-mcp/          # MCP server binary
clients/
└── vscode/              # VS Code extension (VSIX) — bundles LSP + MCP
site/                    # Eleventy static site
docs/
├── specs/SPEC.md           # full research + design spec
└── plans/PLAN.md           # phased execution plan with TODO at bottom
.github/workflows/ci.yml    # CI
.devcontainer/              # dev container
.claude/skills/             # repo-local skills: ci-prep, code-dedup, submit-pr
Makefile                    # 7 standard targets
coverage-thresholds.json    # single source of truth for coverage thresholds
Cargo.toml                  # workspace + strict lints
rustfmt.toml
```

## Too Many Cooks (Multi-Agent Coordination)

If the TMC server is available: register on start (name, intent, files), lock files before editing, broadcast your plan and message others frequently, check messages periodically, and release locks when done. Never edit a locked file — wait or take another approach.

## Migration to `lspkit`

The cross-cutting LSP+MCP scaffolding in this repo is the prime example of the "one engine, two surfaces" pattern. That pattern is being distilled into the generic `lspkit-*` workspace (a separate sibling repo). Domain-specific analysis (parsing, fingerprinting, clustering, ranking, embeddings) stays here; the protocol shells are what migrate.

**For new LSP/MCP infrastructure work:** prefer `lspkit-*` crates over reinventing it here.
**For changes to existing scaffolding in this repo:** flag in the PR description if the patch duplicates `lspkit` functionality, and reference the upstream crate.

Mapping (current → toolkit crate):

| Current path | Toolkit crate |
|---|---|
| `crates/deslop-core/src/live/api.rs` `LiveApi` trait | `lspkit::EngineApi` — the headline contract (associated `Report` / `Query` / `Error`, `generation()`, `report()`, `rescan()`, `subscribe()`, `shutdown()`) |
| `crates/deslop-core/src/live/session.rs` `AnalysisSession` + `LiveService` | `lspkit-live::Session` + consumer-implemented `Analyzer` |
| `crates/deslop-core/src/live/watcher.rs` notify-driven watcher | `lspkit-live::watcher::FileWatcher` |
| `crates/deslop-core/src/live/scheduler.rs` debouncer | `lspkit-live::scheduler::spawn` |
| `crates/deslop-core/src/state.rs` `FileRegistry` | (engine-internal — stays here) |
| `crates/deslop-lsp/src/main.rs` + `backend.rs` LSP entrypoint (tower-lsp) | `lspkit-server` (hand-rolled JSON-RPC + `Dispatcher` + `Capabilities`) — **note:** toolkit does not depend on `tower-lsp` (unmaintained) |
| `crates/deslop-mcp/src/server.rs` + `protocol.rs` hand-rolled JSON-RPC | `lspkit-mcp` (rmcp adapter behind a newtype wall) |
| `crates/deslop-mcp/src/tools/mod.rs` tool dispatch table | `lspkit-mcp::tools::ToolRegistry` + `lspkit-mcp::Adapter::register` |
| `crates/deslop-mcp/src/backend/mod.rs` `LiveBackend` IPC client | (engine query path; both LSP and MCP consume the same `EngineApi` impl in-process) |
| `crates/deslop-core/src/config.rs` `.deslop.toml` loader | `lspkit-config::load_from_ancestor` |
| `crates/deslop-lsp/src/observability.rs` tracing setup | `lspkit::tracing_setup::TracingBuilder` (feature `tracing-setup`) |
| `crates/deslop-mcp/src/main.rs:71–76` path canonicalization | (not yet in toolkit; v0.1 follow-up) |

Code in this repo is **not** being removed — it stays canonical until the toolkit matures. This note exists so future agents reuse `lspkit` for new servers and avoid widening this repo's scaffolding.
