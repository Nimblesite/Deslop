# Deslop Live — Agent Instructions

Deslop is a duplicate-code detector. **Accuracy-audit mode:** one measure outranks features, languages, UI and performance:

> **Every reported cluster is a real duplicate, every real duplicate is reported, and every figure (percentages included) is a transparent, accurate calculation.**

## Invariants

- 🛑 **Never kill a VS Code process**, including browser-hosted instances.
- 🛑 **Never host a report.** ❌ artifacts, hosted pages, uploads. Every report is a Markdown file in this repo, emitted **mechanically** by the code that ran the measurement; agent-typed figures can't be reproduced or diffed. Point the user at the path.
- 🛑 **One branch, one worktree.** Merge loose code, stashes and branches into a single branch.
- **Tree-sitter only.** ❌ regex or text matching on source code or structured data.
- **Core calculations live only in Rust.** Duplication maths in TypeScript (or anywhere else) = hard violation. Scripts/utilities exempt.
- ❌ placeholders, silent no-ops, legacy code, lint suppressions (`#[allow(clippy::...)]`). Proper error types; fix the code.
- **Bugs follow the [fix-bug skill](.claude/skills/fix-bug/SKILL.md):** root cause, replace broken code, ❌ a duplicate path beside it.
- 🛑 **Never work around a bug outside this repo** (grammars, dependencies, Osprey). Log it on the owner's GitHub repo, demand the fix over Too Many Cooks, bump the pin when it lands. The test stays red until then.
- **IPC wire models are generated from [typeDiagram](https://typediagram.dev/docs/language-reference.html)** — never hand-written or committed.

## Rust

- ❌ `unwrap()`/`expect()` in production; tests may `expect` with a message.
- ❌ `panic!`/`todo!`/`unimplemented!`/`unreachable!` for control flow or errors — return `Result<T, E>`. Mandated only for the quarantine.
- ❌ `unsafe {}` (`unsafe_code = "deny"`). Public items carry `///` docs (`missing_docs = "deny"`).
- `thiserror` in `deslop-core`; `anyhow` in the `deslop` binary.
- Pattern matching over casting. Expressions over statements. Iterator chains over loops. Early return with `?`. Descriptive names — single letters only in closures.

## Workflow

- **Act autonomously.** ❌ stopping for confirmation except where the quarantine says STOP. Record assumptions, continue.
- 📖 **RTFM.** Before asking the user what Deslop *should* do, `grep` `docs/specs/` for the rule (spec IDs, fixture names, the measured axis) and read it. The spec decides; tests and code answer to it. Ask only when the spec is silent or contradicts code/tests — and then quote the section you read.
- **Offload legwork (repetitive, bounded work) to pi agents**, exact ids in this precedence: `kimi-coding/k3.1-pro-highspeed` (UI) → `zai/glm-5.3` (scoped implementation/reviews) → `zai/glm-5.3-flash` (fixtures, test expansion) → `deepseek/deepseek-flash` (oracles/tests); `deepseek/deepseek-v4-pro` only for narrow audits. `pi auth check --model <id>` first; out of credit → next. Register each pi on Too Many Cooks (`PiDeslop<Task>N`), run with a timeout, monitor its log, remove it after exit. **You own its output:** verify, correct, record model/date/result in `~/Documents/Code/PiNotes`.
- **Aggressively DRY.** A duplication detector's codebase must be exemplary: run deslop on itself routinely.
- **A+ bar.** Fix substandard code now, never defer.
- **Tokens:** check file size first, `Grep` over `Read`, `offset`/`limit`, smallest diff. Delete dead code, unused imports, stale comments.
- **Headlines only.** Replies are a status line: result, the number that moved, the file path. ❌ restating the request, narrating steps, repeating report tables. 🛑 blocker, 🔴 regression/failing gate, ✅ pass.
- **Too Many Cooks** for file locking and agent messaging.
- **Close a GitHub issue only after verifying the fix on the officially released version** (Marketplace/Homebrew/Scoop build, never a dev build; ❌ auto-close via PR), with the test process — version, steps, inputs, observed result — documented on the ticket.
- Delete every temp file you write.
- Files < 500 LOC, functions < 20 LOC. **Named constants, not literals** — in tests too; every threshold is a config-overridable default.
- **Spec IDs, not issue numbers.** Hierarchical, non-numeric (`[GROUP-TOPIC-DETAIL]`), cited in spec, code and tests so `grep [RANK-` finds all three. Same-group sections sit adjacent. Only explicit regression tests may cite a GitHub issue.
- **Global state** lives in `crates/deslop-core/src/state.rs` (one equivalent file per other language).
- **Dependency versions stay in sync** across `Cargo.toml`, `.github/workflows/ci.yml`, `.devcontainer/`.
- **Auto-memory is off.** Durable rules = reviewed changes to this file.
- **VSIX is the only distribution**, plus Homebrew/Scoop for the CLI. Build output under `target/`; ❌ `cargo install --path crates/deslop-*`.
- **Docs point external MCP clients at the absolute VSIX path** `~/.vscode/extensions/nimblesite.deslop-live-<VERSION>-<platform>/bin/<platform>/deslop-mcp` (Windows likewise); a `PATH` binary drifts off the wire contract. Bare `deslop-mcp` = release-locked installs only: Homebrew, Scoop, or the published fail-closed curl installer ([DEPLOY-EXTERNAL-MCP-CONSUMER]).
- **Structured logging through `tracing` only** (❌ `println!`/`eprintln!`): the sink is configuration, never code. Log with fields (`tracing::info!(file_count = 42)`) at the entry/exit of significant operations — counts and hashes, **never** file contents, user paths or secrets. Reports are output (stdout or `--output <path>`), not logs.
- **"Deslop.live"** = the whole reactive loop: watcher → scheduler → session → broadcast → UI refresh.

## Testing — the accuracy enforcement surface

- **Logical folders and files.** ❌ dumping tests in `crates/deslop/tests` named after an issue.
- **Coarse E2E, black-box.** Drive the CLI against fixture repos; assert on rendered reports. ❌ reaching into internals.
- **Loaded tests:** many user interactions per test, many assertions per interaction.
- **Every confirmed false positive/negative earns a fixture** that would have caught it.
- **Never back off a test.** ❌ deleting, skipping, removing assertions, reducing assertiveness. Add failing tests for broken or missing functionality.
- ❌ regression-named tests/files; group regressions logically (a subtle issue-number mention is fine).
- **Unit tests** only to isolate a function's behaviour.
- **Meaningful assertions.** ❌ `assert!(true)`; assert positive, human-readable values, not the absence of AI-style labels. ❌ try/catch that swallows an error then asserts success.
- **No fake LSP/MCP.** UI and extension tests build and install the latest binaries first.
- **`make test` is fail-fast** and enforces coverage. ❌ `--no-fail-fast`. Target 💯 coverage and a high mutation score.
- **Deterministic.** ❌ `sleep`, timing dependencies, random state.
- **Coverage thresholds live in `coverage-thresholds.json`** at the repo root — never env vars, GitHub variables or CI YAML. Monotonic ⬆️ (−1% rounding allowance); below = pipeline fails.

## 🛑 Accuracy quarantine — tests outrank code

A red test that pins a false positive/negative beats code that might be right. When you find code that could produce one, whatever your task:

1. 🛑 **STOP.**
2. **Delete the code.** Replace it with a `panic!` saying what it did and why; allow `panic = "deny"` on that line only. A panic is found in seconds, a silent false negative never — the one place panics are mandated.
3. **Write the test that now fails:** assert cluster, occurrence count, file paths, bucket and ranking order. A test that passes when the detector goes blind asserts nothing.
4. **Report** file, defect, failing test and what you removed, then wait.

After the user acknowledges, write a **fresh implementation** that turns the test green. ❌ patching or working around the deleted code.

## Version comparison — the corpus run

"Compare to version X" means **only** `scripts/compare-versions.sh <X> HEAD` ([CORPUS-SCORE-COMPARE]): both builds scored against `corpus/register/`. It writes `compare-<UTC>.md` + `.json` ([CORPUS-REPORT]), `INDEX.md` and per-repo `SUMMARY.md` under `.corpus/version-compare/reports/`. Point the user at them. ❌ substituting an LLM, subagent or code review.

## Documentation

- 🔗 Cross-reference spec IDs across tests, code, specs and plans; each spec section has a unique hierarchical, non-numeric ID.
- ⚖️ Code, specs and tests must agree. Where they don't, 🛑 STOP and report.
- 🤖 Routinely update `site/src/docs/for-ai.md` with instructions for agents using Deslop.
- 🧑 Specs are for humans first: jargon goes in a separate "For AI" section. ✂️ Remove fluff that specifies nothing.
- 📝 PR documentation tight and human-readable (except the AI section). Prose wraps naturally.

## Architecture

```mermaid
flowchart LR
    discover[Discover] --> parse[tree-sitter parse] --> normalize[Normalize AST] --> fingerprint[Fingerprint] --> cluster[Cluster] --> lsh[Token LSH] --> embed[Embeddings] --> fuse[Fuse] --> rank[Rank] --> render[Render]
```

Every stage can lose accuracy: over-normalizing manufactures false positives, under-normalizing false negatives. Ranking changes change every report — new assertions, not adjusted ones.

`make ci` · `make test` · `make lint`; debug one test with `cargo test <name> -- --nocapture`. Paths and stage detail: [docs/repo-index.md](docs/repo-index.md), read only when locating something.

## UI and audience

- VSIX first; IntelliJ and other plugins in progress. Panel consistency is critical.
- **Never duplicate rendering of text or links** (clusters, occurrences): one shared function, reused everywhere.
- Context menus always include "Copy Context For AI".
- Humans first for comments, UI and HTML reports. Raw JSON reports target AI — complete enough to reconstruct the human view.
