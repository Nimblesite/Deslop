<!-- agent-pmo:b636503 -->
# Deslop Live — Agent Instructions

NEVER CLOSE GH ISSUES, EVEN WITH PR COMMENTS!

Deslop is a duplicate-code detector. **We are in accuracy-audit mode.** One measure outranks everything else — not features, not languages, not UI, not performance:

> **Every reported cluster is a real duplicate, and every real duplicate is reported.** And, the reported figures like percentages are **transparent, accurate** calculations 

## The value assertion — test assertions outrank code

**A test that fails because of a bug is worth more than code that might produce a false positive or false negative.** A red test pins the defect, survives refactors, and turns a suspicion into an enforceable contract. Code you *believe* is correct is a liability until an assertion proves it.

- Write the test before the fix. If you can only do one, write the test.
- Leaving a red test in the tree is a correct outcome. Weakening it is not.
- Assert the cluster, occurrence count, file paths, bucket, and ranking order. A test that would still pass if the detector went blind asserts nothing.
- Many assertions per test beats many tests with one assertion.

## 🛑 STRICT NO INACCURATE CODE RULE

If you encounter code that could cause a false negative or a false positive — whether or not it is the code you were sent to change — do this, in order, and nothing else:

1. **Write a test that fails because of the bug.** It must fail for the real reason, and you must watch it fail.
2. **Replace all the defective code with a `panic!`**, commented with what the code did, why it was deleted, and which test pins it. You need to put a panic = "deny" ignore in for these kinds of panics
3. **Report to the user** — file, defect, failing test, what you removed — and why.
4. 🛑 **STOP.** Do not repair it, do not work around it, do not resume the original task.

**Panics are NOT ALLOWED for control flow or error handling — but are MANDATED where code is causing inaccuracies.** The quarantine `panic!` is not optional and not merely permitted; it is the required outcome, and it overrides the Rust no-panic rule below. Silently-wrong output is worse than a crash: a panic is found in seconds, a false negative is never found at all.

## BUG FIXING PROCESS

CRITICAL: YOU ARE NOT ALLOWED TO BACK OFF ANY TESTS OR ASSERTIONS
YOU MUST FIX THE ROOT CAUSE OF BUGS; NOT WORK AROUND THEM
REPLACE BROKEN CODE; DON'T WRITE NEW CODE WITH A DUPLICATE PATH

## Standing prohibitions

- ⚠️ **Never kill a VS Code process**, including browser-hosted instances. The user cannot recover from this.
- ⚠️ **No git.** No `add`, `commit`, `push`, `checkout`, `merge`, `rebase`, `worktree`. Never push to `main`, never stamp yourself as co-author. One branch at a time; never start a new branch when a feature branch exists; converge branches before other work. CI handles git. (`gh issue create` excepted.)
- ⚠️ **No using text pattern matching on source code or structured data. No RegEx on code**. USE THE AST!
- ⚠️ **Token discipline.** Check file size before reading. `Grep` over `Read`; use `offset`/`limit`. Smallest diff that solves the problem. Delete dead code, unused imports, stale comments. Call out irrelevant context — bloat degrades reasoning.
- ⚠️ **A+ quality bar.** Every change must pass review at a top-tier engineering org. Substandard code is fixed immediately, never deferred.
- ⚠️ **"Deslop.live" (reactive) means the whole loop** — watcher → scheduler → session → broadcast → UI. An incremental update drives the entire pipeline, including a reactive UI refresh.

## Testing — the accuracy enforcement surface

- **Coarse E2E, black-box only.**. Drive the CLI against fixture repos; assert against rendered reports. Never reach into internals.
- **Many user interactions per test, MANY assertions per user interaction**
- **Every confirmed false positive or false negative earns a fixture** that would have caught it.
- **Never delete a failing test, never skip one, never remove an assertion.** Reducing assertiveness is prohibited. Add failing tests for broken or missing functionality.
- **Unit tests are only for isolating behavior of functions**
- **Meaningful assertions only.** `assert!(true)` is banned. Assert positive, human-readable values — not the absence of AI-style labels.
- **No try/catch that swallows an error and then asserts success.**
- **No fake LSP/MCP.** UI and extension tests build and install the latest binaries first.
- **`make test` is fail-fast** and always enforces coverage. Never `--no-fail-fast`. Target 100% coverage and a high mutation score.
- **Deterministic.** No `sleep`, no timing dependencies, no random state.
- **Coverage thresholds live in `coverage-thresholds.json`** at the repo root — never env vars, GitHub variables, or CI YAML. Monotonic increase only (−1% rounding allowance); falling below fails the pipeline.

## Universal rules

- **Files < 500 lines. Functions < 20 lines.** Refactor when over.
- **Act autonomously.** Do not stop for confirmation — except where the strict accuracy rule says STOP. Record assumptions and continue.
- **Aggressively DRY.** This tool detects duplication; its own codebase must be exemplary. Move code, don't copy. Copying files is prohibited.
- **Tree-sitter only.** Regex on source code or structured data is prohibited.
- **No placeholders, no silent no-ops, no legacy code.** Use proper error types; legacy is deleted.
- **No linter suppressions.** `#[allow(clippy::...)]` is prohibited — fix the code.
- **All global state in `crates/deslop-core/src/state.rs`.** Same rule for TypeScript and every other language.
- **Bug fixes follow the [fix-bug skill](.claude/skills/fix-bug/SKILL.md).**
- **Spec IDs are hierarchical and non-numeric** — `[GROUP-TOPIC]` / `[GROUP-TOPIC-DETAIL]`, e.g. `[PARSE-CSHARP-NORMALIZE]`, `[RANK-SCORE]`. Same-group sections sit adjacent; no sequential numbers. Code and tests carry the ID in a comment so `grep [PARSE-` finds spec → code → tests.
- **Dependency versions stay in sync** across `Cargo.toml`, `.github/workflows/ci.yml`, and `.devcontainer/`.
- **Auto-memory is off.** Durable rules go through reviewed changes to this file.
- **IPC wire models are generated from [typeDiagram](https://typediagram.dev/docs/language-reference.html)** — never hand-written. Generated code is git-ignored, never committed.
- **The VSIX is the only distribution.** Every build target leaves artifacts under `target/` only; `cargo install --path crates/deslop-*` is prohibited. Releases ship via `.vsix`, plus Homebrew/Scoop for the CLI.
- **External MCP clients use the absolute VSIX path** — `~/.vscode/extensions/nimblesite.deslop-live-<VERSION>-<platform>/bin/<platform>/deslop-mcp`, equivalent on Windows; every doc snippet shows this form. A `PATH` binary shadows the versioned bundle and drifts analysis off the wire contract — an accuracy defect by construction. Bare-name `deslop-mcp` is valid only for Homebrew/Scoop installs.
- **Logging:** `tracing` only, never `println!`/`eprintln!`. Structured fields, not interpolation — `tracing::info!(file_count = 42, lang = "csharp")`. Log entry and exit of significant operations (`error|warn|info|debug|trace`). Never log file contents, user-data paths, or secrets — counts and hashes only. Reports are not logs: they go to stdout or `--output <path>` via the renderer.

## Rust

- No `unwrap()`/`expect()` in production code; tests may `expect` with a message.
- No `panic!`/`todo!`/`unimplemented!`/`unreachable!` for control flow or error handling — return `Result<T, E>`; a panic there is a bug. Mandated only for the accuracy quarantine above.
- No `unsafe {}` (`unsafe_code = "deny"`). All public items carry `///` docs (`missing_docs = "deny"`).
- `thiserror` in `deslop-core`; `anyhow` allowed in the `deslop` binary.
- Pattern matching over casting. Expressions over statements. Iterator chains over imperative loops. Early return with `?`.
- Descriptive names — no single letters except in closures.

## Documentation

- Each spec section must have a unique, heirarchical non-numeric spec Id
- Spec ids must be cross referenced across tests, code specs and plans
- Code, specs, and tests MUST agree. Where they don't, 🛑 STOP and report the issue to the user
- Don't use line endings to force word wrap. Allow text to wrap naturally.
- Keep PR documentation TIGHT and HUMAN READABLE (except for the AI section)
- Remove line endings that only exist to wrap text
- Remove fluff from the specs that don't specify anything

## Run Deslop on Deslop

**Use the Deslop MCP if it is available; fall back to the `deslop` CLI if it is not.**

- **Prevent:** before writing any code unit past a few lines, `find-similar`. On a strong match, reuse the canonical occurrence — do not write the near-copy.
- **Clean up:** `top-offenders` → `cluster-by-id` for existing duplicates; `report-for-file` / `report-for-range` for a specific target.
- **A wrong, stale, or missing result from either surface is an accuracy defect** — `gh issue create` with the cluster id or triggering snippet. Never work around a defect, widen a threshold, or hide a cluster. (`gh` is the sole exception to the no-git rule.)

## Architecture

```
discover files → per-language parse (tree-sitter) → normalize AST →
fingerprint subtrees → cluster → token LSH → embeddings (hybrid) →
fuse signals → rank → render report
```

Every stage can lose accuracy. Normalization that strips too much manufactures false positives; too little manufactures false negatives. Ranking changes change every report — they need new assertions, not adjusted ones.

**Build:** `make ci` · **Test:** `make test` · **Lint:** `make lint`. There are 7 AgentPMO make targets; repo-specific targets sit below a horizontal marker. Debug one test with `cargo test <name> -- --nocapture` — not a Makefile target.

Paths, pipeline-stage detail, and the `lspkit` migration map live in [docs/repo-index.md](docs/repo-index.md) — read it only when you need to locate something.

## UI and audience

- VSIX first; IntelliJ and other plugins in progress. Consistency across panels is critical.
- **Never duplicate the rendering of text or links** (clusters, occurrences). One shared function, reused everywhere.
- Context menus always include "Copy Context For AI".
- Humans first for code comments, UI, and HTML reports. Raw JSON reports target AI — complete enough to reconstruct the human view.

## Multi-agent (Too Many Cooks)

If the TMC server is available: register on start (name, intent, files), lock files before editing, broadcast your plan, check messages periodically, release locks when done. Never edit a locked file.
