<!-- agent-pmo:9a71cbf -->
# Deslop Live — Agent Instructions

⚠️ KILLING A VSCODE PROCESS - EVEN IN THE BROWSER WILL BE MET WITH INSTANT, EXTREME VIOLENCE!

> ⚠️ **TOKEN DISCIPLINE.** Check file size first. `Grep` over `Read`. Use `offset`/`limit`.
> Smallest diff that solves the problem. Delete dead code, unused imports, stale comments.
> Call out irrelevant context before proceeding. Bloat degrades reasoning. ⚠️

> ⚠️ **CRITICAL: THIS CODEBASE RECEIVES A GRADE OF A+.** WE DON'T ALLOW BAD CODE. NOT EVEN FOR ONE LINE. CODE MUST PASS REVIEW AT Google / Meta / Microsoft. ANYTHING LESS IS ⛔️ ILLEGAL AND MUST BE FIXED IMMEDIATELY.

⚠️ ALL MODELS TRANSFERRED ACROSS THE WIRE MUST USE typeDiagram. NO IFS. NO BUTS
https://typediagram.dev/docs/language-reference.html ⚠️

## Project Overview

**Deslop** (a.k.a. Deslop Live) is a **live duplicate-code analysis server** for AI coding agents and the humans driving them. The shipping surfaces are `deslop-lsp` (LSP server feeding live clone warnings to any LSP-capable editor) and `deslop-mcp` (MCP server letting Claude Code / Cursor / Copilot / Continue / Codex query the running analysis mid-generation, *before* a copy-paste happens). The `deslop` CLI is the cold-cache fallback for CI gates and one-shot audits. All three binaries are thin shells over one `deslop-core` library — the LSP and MCP sit in the agent's inner loop, the CLI re-uses the same engine for batch runs. Ranking is **worst offenders first** (highest weighted duplication impact at the top). Detection and ranking ship today; AI-assisted and mechanical deduplication actions are on the roadmap. Languages start with **C#**, then Rust and Python. Parsing is always tree-sitter — regex on source is illegal.

Full spec: [docs/specs/SPEC.md](docs/specs/SPEC.md). Execution plan + live TODO: [docs/plans/PLAN.md](docs/plans/PLAN.md).
- ALL SPEC SECTIONS HAVE NON-NUMERIC HIERARCHICALLY STRUCTURED SECTIONS. ALL TESTS REFER TO SPEC IDs. ALL CODE REFERS TO SPEC IDS.

**Primary language:** Rust
**Build command:** `make ci`
**Test command:** `make test`
**Lint command:** `make lint`

**There are 7 AgentPMO make targets. Repo specific targets must have a horizontal marker.** `make test` runs the test runner with its fail-fast flag, collects coverage, asserts measured ≥ threshold from `coverage-thresholds.json`, and exits non-zero on any failure. To debug a single test, invoke `cargo test <name> -- --nocapture` directly — that is not a Makefile target.

## UI

- The initial UI is a VSIX, but we we are also working on IntelliJ and other plugins
- Consistency across UI panels is CRITICAL
- Do not DUPLICATE the rendering of text or links like clusters and occurences. Create a shared function that gets reused everywhere
- What is displayed on screen MUST BE HUMAN READABLE. The display is NOT FOR AI BY DEFAULT
- However, context menus should always have a "Copy Context For AI" item so that they can feed the context to AI directly
- Specific AI reports like the JSON file and AI reports generated from it should REMAIN human unreadable. These reports are only targeted for AI

## Architecture

```
discover files → per-language parse (tree-sitter) → normalize AST →
fingerprint subtrees → cluster → token LSH → embeddings (hybrid) →
fuse signals → rank → render report
```

### IPC

Processes communicate using IPC. Generate IPC model code with [typeDiagram](https://typediagram.dev/docs/language-reference.html). Do not store model code in git. Git ignore it.

- **`crates/deslop-core`** — analysis library. Everything non-trivial lives here. The CLI, LSP, and MCP binaries all consume this single crate.
- **`crates/deslop`** — thin CLI binary (<50 LOC of glue): arg parsing, tracing setup, invoke core, render output.
- **`crates/deslop-lsp`** — LSP server surface; streams live clone warnings to any LSP-capable editor.
- **`crates/deslop-mcp`** — MCP server surface; lets agent tools query the running analysis mid-generation.
- **`clients/vscode`** — VS Code extension (VSIX) that bundles the LSP + MCP binaries and surfaces reports in-editor.
- **`LanguageParser` trait** is the single extension point. Adding a language = implementing the trait + pinning the grammar in `Cargo.toml`, CI, and Dockerfile.
- **Normalization** strips identifiers, literals, and trivia before fingerprinting so renamed-clone detection works (Type-2). Per-language rules, identical output format across languages.
- **Fingerprinting** operates on AST subtrees, not lines. Minimum node count configurable.
- **Ranking score** weights clone size × clone count × spanned LOC — this is the user-visible product. Changes here change every report.
- **Global state** lives in exactly one file! Rust: `crates/deslop-core/src/state.rs`. Nothing escapes it. Same goes for Typescript or any other language!

## Hard Rules — Universal - No exceptions, NON-NEGOTIABLE

- CRITICAL: **Files < 500 lines.** Refactor when over.
- **NO git commands.** No `add`, `commit`, `push`, `checkout`, `merge`, `rebase`, etc. CI handles git.
- **REDUCE CODE DUPLICATION. DRY AF.** This tool detects duplication — its own codebase must be exemplary. Search before writing. Move code, don't copy.
- **Regex on source code / structured data = ⛔️ ILLEGAL.** Use tree-sitter for all source parsing.
- **NO EXCEPTIONS for control flow.** Return `Result<T,E>`. Panics are bugs.
- **NO PLACEHOLDERS.** No silent no-ops. Use proper error types.
- **Functions < 20 lines** 
- **Mandatory Bug Fix Process** = [text](.claude/skills/fix-bug/SKILL.md)
- **No legacy code.** Legacy = deleted.
- **Copying files is illegal.** MOVE them.
- **Centralize all global state** in `crates/deslop-core/src/state.rs`.
- **Never delete failing tests. Never remove assertions.** Reducing assertiveness = ⛔️ ILLEGAL.root — NOT env vars, NOT gh repo variables, NOT CI YAML. Below threshold = pipeline fails. Ratchet only.
- **Coarse E2E tests only.** No unit tests. Drive the CLI end-to-end against fixture repos and assert against rendered reports.
- **Heavy structured logging.** See Logging below.
- **No linter suppressions.** `#[allow(clippy::...)]` = ⛔️ ILLEGAL. Fix the underlying code.
- **Dependency versions in `Cargo.toml`, `.github/workflows/ci.yml`, and `.devcontainer/` stay in sync at all times.**
- **Spec IDs are hierarchical, non-numeric: `[GROUP-TOPIC]` / `[GROUP-TOPIC-DETAIL]`** (e.g., `[PARSE-CSHARP-NORMALIZE]`, `[RANK-SCORE]`). Same-group sections sit adjacent in the doc. NO sequential numbers. Code/tests referencing a spec section include the ID in a comment so `grep [PARSE-` finds spec → code → tests.

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

- ZERO duplicate CSS
- Hard CSS budget 1.5k LOC

## Logging Standards

- **`tracing` + `tracing-subscriber` only.** Never `println!`/`eprintln!` for diagnostics.
- **Log at entry/exit of significant operations.** Levels: `error|warn|info|debug|trace`.
- **Structured fields, not string interpolation.** `tracing::info!(file_count = 42, lang = "csharp")` — never format strings.
- **The CLI's report output is NOT a log.** Reports go to stdout (or `--output <path>`) via the renderer. Diagnostics go to `tracing`.
- **NEVER log file contents, paths containing user data, or secrets.** Log counts and hashes, not source.

## Testing Rules

- **Testing any UI/Extension with a fake LSP/MCP = ⛔️ ILLEGAL!!!** Tests must build and install the latest binaries before running
- **`make test` is FAIL-FAST.** Stops at first failure. Never `--no-fail-fast`.
- **`make test` ALWAYS computes coverage AND enforces it.** Threshold lives in `coverage-thresholds.json` at the repo 
- **Aim for 100% coverage and high mutation score.** LOTS OF ASSERTIONS PER TEST
- **Never delete a failing test. Never skip a test.** Add more failing tests for broken/missing functionality — never remove them.
- **Meaningful assertions only.** `assert!(true)` is illegal.
- **No try/catch that swallows errors and asserts success.**
- **Deterministic.** No `sleep`, no timing dependencies, no random state.
- **E2E tests: black-box only** — the CLI binary, fixture directories, rendered reports. Never reach into internals.
- Coverage threshold lives in `coverage-thresholds.json` and monotonically increaes -1% for rounding

Do not write assertions that guard against AI assertions! Instead, assert **for positive human readable values**. Human readable panels can have subtle technical terms for reference, but they must not **confuse or overwhelm** the user.

⛔️ BAD
```typescript
assert.doesNotMatch(
    md.value,
    /\[Type-\d|\[Type-\d\/\d|\[weak LSH\]|\[Type-\d,\s*AI match\]/,
    `human hover must not expose taxonomy labels: ${md.value}`,
);
```

✅ GOOD
```typescript
const text = inlineText(cluster(), "worst");
assert.match(text, /×\s*4/);
assert.match(text, /Alpha\.cs/);
```

## Human vs. AI Readability

There are two target audiences: AI and humans. What you write depends on who it's for.

Code comments: humans first and AI second
UI (IDE extensions, CLI): humans, but with the ability to COPY the context for AI
Formatted HTML Reports: humans
Raw JSON reports: AI, but with enough information to be able to produce a human readable version

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

If the TMC server is available: register on start (name, intent, files), lock files before editing, broadcast your plan and message others frequently, check messages periodically, release locks when done. Never edit a locked file — wait or take another approach.
