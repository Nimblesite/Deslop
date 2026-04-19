<!-- agent-pmo:424c8f8 -->
# CodeDedup — Agent Instructions

> ⚠️ **TOKEN DISCIPLINE.** Check file size first. `Grep` over `Read`. Use `offset`/`limit`.
> Smallest diff that solves the problem. Delete dead code, unused imports, stale comments.
> Call out irrelevant context before proceeding. Bloat degrades reasoning. ⚠️

> ⚠️ **CRITICAL: THIS CODEBASE RECEIVES A GRADE OF A+.** WE DON'T ALLOW BAD CODE. NOT EVEN FOR ONE LINE. CODE MUST PASS REVIEW AT Google / Meta / Microsoft. ANYTHING LESS IS ⛔️ ILLEGAL AND MUST BE FIXED IMMEDIATELY.

> Read this file in full. Rules below are NON-NEGOTIABLE — violations are rejected in review.

## Project Overview

**CodeDedup** is a Rust CLI that detects duplicated code across a codebase and reports the **worst offenders first** (highest weighted duplication impact at the top). Language support starts with **C#**, then Rust and Python. Parsing is always tree-sitter — regex on source is illegal.

Full spec: [docs/specs/SPEC.md](docs/specs/SPEC.md). Execution plan + live TODO: [docs/plans/PLAN.md](docs/plans/PLAN.md).

**Primary language:** Rust
**Build command:** `make ci`
**Test command:** `make test`
**Lint command:** `make lint`

## Architecture

```
discover files → per-language parse (tree-sitter) → normalize AST →
fingerprint subtrees → cluster → token LSH → embeddings (hybrid) →
fuse signals → rank → render report
```

- **`crates/codededup-core`** — analysis library. Everything non-trivial lives here. A future LSP consumes the same crate.
- **`crates/codededup`** — thin CLI binary (<50 LOC of glue): arg parsing, tracing setup, invoke core, render output.
- **`LanguageParser` trait** is the single extension point. Adding a language = implementing the trait + pinning the grammar in `Cargo.toml`, CI, and Dockerfile.
- **Normalization** strips identifiers, literals, and trivia before fingerprinting so renamed-clone detection works (Type-2). Per-language rules, identical output format across languages.
- **Fingerprinting** operates on AST subtrees, not lines. Minimum node count configurable.
- **Ranking score** weights clone size × clone count × spanned LOC — this is the user-visible product. Changes here change every report.
- **Global state** lives in exactly one file: `crates/codededup-core/src/state.rs`. Nothing escapes it.

## Hard Rules — Universal (no exceptions)

- **NO git commands.** No `add`, `commit`, `push`, `checkout`, `merge`, `rebase`, etc. CI handles git.
- **REDUCE CODE DUPLICATION. DRY AF.** This tool detects duplication — its own codebase must be exemplary. Search before writing. Move code, don't copy.
- **Regex on source code = ⛔️ ILLEGAL.** Use tree-sitter for all source parsing.
- **NO EXCEPTIONS for control flow.** Return `Result<T,E>`. Panics are bugs.
- **NO REGEX on structured data.** Use real parsers for JSON/YAML/TOML/code.
- **NO PLACEHOLDERS.** No silent no-ops. Use proper error types.
- **Functions < 20 lines. Files < 500 lines.** Refactor when over.
- **No legacy code.** Legacy = deleted.
- **Copying files is illegal.** MOVE them.
- **Centralize all global state** in `crates/codededup-core/src/state.rs`.
- **Never delete failing tests. Never remove assertions.** Reducing assertiveness = ⛔️ ILLEGAL.
- **`make test` is FAIL-FAST.** Stops at first failure. Never `--no-fail-fast`.
- **`make test` ALWAYS computes coverage AND enforces it.** Threshold lives in `coverage-thresholds.json` at the repo root — NOT env vars, NOT gh repo variables, NOT CI YAML. Below threshold = pipeline fails. Ratchet only.
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
- `thiserror` for library errors in `codededup-core`. `anyhow` allowed in the `codededup` binary.
- Pattern matching over casting. Expressions over statements. Iterator chains over imperative loops.
- Early return with `?` for clean error propagation.
- Descriptive variable names — no single letters except in closures.

## Logging Standards

- **`tracing` + `tracing-subscriber` only.** Never `println!`/`eprintln!` for diagnostics.
- **Log at entry/exit of significant operations.** Levels: `error|warn|info|debug|trace`.
- **Structured fields, not string interpolation.** `tracing::info!(file_count = 42, lang = "csharp")` — never format strings.
- **The CLI's report output is NOT a log.** Reports go to stdout (or `--output <path>`) via the renderer. Diagnostics go to `tracing`.
- **NEVER log file contents, paths containing user data, or secrets.** Log counts and hashes, not source.

## Testing Rules

- **Aim for 100% coverage and high mutation score.**
- **Never delete a failing test. Never skip a test.** Add more failing tests for broken/missing functionality — never remove them.
- **Specific assertions only.** `assert!(true)` is illegal.
- **No try/catch that swallows errors and asserts success.**
- **Deterministic.** No `sleep`, no timing dependencies, no random state.
- **E2E tests: black-box only** — the CLI binary, fixture directories, rendered reports. Never reach into internals.
- Coverage threshold lives in `coverage-thresholds.json` and only goes up.

### Bug Fix Process

1. Write a test that fails because of the bug.
2. Run the test — confirm it fails **because of the bug** (right reason).
3. Repeat until it's failing for the right reason.
4. Fix the bug (do **NOT** change the test).
5. Run the test — confirm it passes.

## Build Commands

Cross-platform GNU Make. On Windows: `choco install make` or use the one in Git for Windows.

```bash
make build   # cargo build --release
make test    # FAIL-FAST tests + coverage + threshold (ONLY test entry point)
make lint    # cargo clippy (read-only, no formatting)
make fmt     # cargo fmt (in place; CHECK=1 for read-only CI check)
make clean   # cargo clean + remove report artifacts
make ci      # lint + test + build (full CI simulation)
make setup   # post-create dev environment setup
```

**There are exactly 7 targets. No others.** `make test` runs the test runner with its fail-fast flag, collects coverage, asserts measured ≥ threshold from `coverage-thresholds.json`, and exits non-zero on any failure. To debug a single test, invoke `cargo test <name> -- --nocapture` directly — that is not a Makefile target.

**`make fmt`** formats. **`make lint`** reads. **`make test`** runs tests with coverage. Three separate targets — no overlap.

## Repo Structure

```
crates/
├── codededup-core/         # library: pipeline stages
│   └── src/
│       ├── lib.rs
│       └── state.rs        # single global-state file
└── codededup/              # thin CLI binary
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

If the TMC server is available: register on start (name, intent, files), lock files before editing, broadcast your plan, check messages periodically, release locks when done. Never edit a locked file — wait or take another approach.
