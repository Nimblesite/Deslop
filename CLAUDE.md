# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

⚠️ CRITICAL: WE TREAT THIS CODEBASE WITH RESPECT. THIS CODE WOULD PASS REVIEW AT Google, Meta and Microsoft. WE DON'T ALLOW BAD CODE. NOT EVEN FOR ONE LINE. THIS CODEBASE RECEIVES A GRADE OF A+. ANYTHING LESS IS ⛔️ILLEGAL AND YOU MUST FIX IT IMMEDIATELY.

# Project

**CodeDedup** — a Rust CLI tool that detects duplicated code across a codebase and produces a report ordered by **worst offenders first** (highest duplication impact at the top).

- **Language support (initial):** C#, Rust, Python.
- **Parsing:** `tree-sitter` with the per-language Rust grammars (`tree-sitter-c-sharp`, `tree-sitter-rust`, `tree-sitter-python`). Regex-based parsing is ⛔️ ILLEGAL — always use the tree-sitter AST.
- **Output:** ordered text/JSON report. Worst offender = largest weighted duplication (clone size × clone count × span).

The binary is a CLI — no LSP, no IDE extension, no daemon. Keep it focused.

# Build Commands

Cross-platform GNU Make. Seven standard targets only — no others:

```bash
make build   # cargo build --release
make test    # FAIL-FAST tests + coverage + threshold (ONLY test entry point)
make lint    # cargo clippy + any analyzers (no formatting)
make fmt     # cargo fmt (in place)
make clean   # cargo clean + remove report artifacts
make ci      # lint + test + build (full CI simulation)
make setup   # post-create dev environment setup
```

`make test` runs the test runner with its fail-fast flag, collects coverage, asserts measured ≥ threshold from `coverage-thresholds.json` at repo root, exits non-zero on any failure. To debug a single test, invoke `cargo test <name> -- --nocapture` directly — that is **not** a Makefile target.

Three separate targets, no overlap: **fmt** writes, **lint** reads, **test** runs tests with coverage.

# Architecture

The pipeline is linear and deliberately simple:

```
discover files → per-language parse (tree-sitter) → normalize AST →
fingerprint subtrees → cluster matching fingerprints → score & rank →
render report
```

Key architectural points that span multiple files:

- **Language plugin trait** is the single extension point. Adding a language = implementing the trait + registering the grammar. C#, Rust, and Python live behind the same trait so the rest of the pipeline is language-agnostic.
- **Normalization** strips identifier names, literals, and trivia before fingerprinting so that renamed-clone detection works (Type-2 clones). Keep normalization rules per-language but the fingerprint format identical across languages.
- **Fingerprinting** operates on AST subtrees, not lines. Only subtrees above a configurable minimum node count are considered — small fragments are noise.
- **Ranking** is the user-visible product. The score weights clone size, clone count, and total spanned LOC. This is the contract — changes here change every report.
- **Global state** lives in exactly one file (e.g. `src/state.rs`). No state escapes it.

# Rules (project-wide, non-negotiable)

- **TOP PRIORITY: REDUCE CODE DUPLICATION.** This tool detects duplication — its own codebase must be exemplary. Always search for existing code before writing new code. Aggressively merge similar code into shared modules.
- **Zero duplication. DRY AF.**
- **CENTRALIZE ALL GLOBAL STATE** in one file.
- `#[allow(clippy::...)]` = ⛔️ ILLEGAL. Fix the underlying issue.
- **Regex = ⛔️ ILLEGAL.** Use tree-sitter for all source parsing.
- **No legacy code.** Legacy = DELETED.
- Keep files **under 500 LOC**. Break up larger files.
- **Copying files is illegal.** MOVE them instead.
- Do **not** use Git unless asked.
- Keep dependency versions in `.github/workflows/ci.yml` and `.devcontainer/Dockerfile` in sync at all times.

## Rust Quality Standards

- All lints at highest strictness (configure in `Cargo.toml` `[lints]`). Add lints when in doubt — never remove them.
- `unsafe` code forbidden (`unsafe_code = "deny"`).
- `unwrap()` is **always** a violation. Use `?` with proper error types.
- No `panic!`, `todo!`, `unimplemented!` — handle every case, return `Result<T, E>`.
- Run clippy and fmt routinely; fix violations immediately.

## Functional Programming Style

- `Result<T, E>` and `Option<T>` everywhere.
- Expressions over statements — `match`, `if let`, iterator chains.
- Pure functions. Minimize side effects.
- Pattern matching over casting or unwrapping.
- Early returns with `?` for clean error propagation.

## Code Structure

- Small, focused functions (<20 lines).
- Low cognitive complexity (`clippy::cognitive_complexity` enabled).
- Descriptive variable names (no single letters except in closures).
- Group related functionality into modules.
- Public APIs must have documentation.

## Logging Standards

- **Structured logging only.** Never `println!`/`eprintln!` for diagnostics. Use `tracing` + `tracing-subscriber`.
- Log at entry/exit of significant operations. Levels: `error|warn|info|debug|trace`.
- **Structured fields, not string interpolation:** `tracing::info!(file_count = 42, lang = "rust")` — never format strings.
- The CLI's *report output* (the user-facing artifact) is **not** a log. Reports go to stdout or a file via the renderer; diagnostics go to `tracing`.
- **NEVER log file contents, paths containing user data, or secrets.** Log counts and hashes, not source.

## Testing

- Aim for 100% coverage and a high mutation score.
- **NEVER delete failing tests. NEVER remove assertions that cause failures.** Add more failing tests for broken/missing functionality — never remove them.
- Reducing test assertiveness = ⛔️ ILLEGAL.
- `make test` is FAIL-FAST. Never `--no-fail-fast`.
- Coverage threshold lives in `coverage-thresholds.json` at repo root — NOT env vars, NOT GH repo variables, NOT CI YAML. Below threshold = pipeline fails. Ratchet only.
- **No unit tests. Only coarse e2e tests** that drive the CLI end-to-end on fixture repos and assert against rendered reports.

## Bug Fix Process

1. Write a test that fails because of the bug.
2. Run the test — confirm it fails **because of the bug** (right reason).
3. Repeat until it's failing for the right reason.
4. Fix the bug (do **NOT** change the test).
5. Run the test — confirm it passes.
