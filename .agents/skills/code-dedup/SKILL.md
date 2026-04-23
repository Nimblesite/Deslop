---
name: code-dedup
description: Searches for duplicate code, duplicate tests, and dead code in the Deslop Rust workspace, then safely merges or removes them. Use when the user says "deduplicate", "find duplicates", "remove dead code", "DRY up", or "code dedup". Requires test coverage — refuses to touch untested code.
---

<!-- agent-pmo:9a71cbf -->

# Code Dedup

Carefully search for duplicate code, duplicate tests, and dead code across the Rust workspace (`crates/deslop-core`, `crates/deslop`). Merge duplicates and delete dead code — but only when test coverage proves the change is safe.

## Prerequisites — hard gate

Before touching ANY code, verify these conditions. If any fail, stop and report why.

1. Run `make test` — all tests must pass. If tests fail, stop. Do not dedup a broken codebase.
2. `make test` is fail-fast AND enforces the coverage threshold from [coverage-thresholds.json](coverage-thresholds.json) (REPO-STANDARDS-SPEC [TEST-RULES], [COVERAGE-THRESHOLDS-JSON]). If anything fails, stop and fix it before deduping.
3. Rust is statically typed — proceed.

## Steps

Copy this checklist and track progress:

```
Dedup Progress:
- [ ] Step 1: Prerequisites passed (tests green, coverage met)
- [ ] Step 2: Dead code scan complete
- [ ] Step 3: Duplicate code scan complete
- [ ] Step 4: Duplicate test scan complete
- [ ] Step 5: Changes applied
- [ ] Step 6: Verification passed (tests green, coverage stable)
```

### Step 1 — Inventory test coverage

1. Run `make test` to confirm green baseline. It is fail-fast and enforces the coverage threshold from [coverage-thresholds.json](coverage-thresholds.json). Non-zero exit = stop.
2. Record the measured line-coverage percentage — this is the floor. It must not drop.
3. Identify which files/modules have coverage and which do not. Only files WITH coverage are candidates for dedup. E2E tests live in [crates/deslop/tests](crates/deslop/tests) and drive the CLI black-box per [AGENTS.md](AGENTS.md).

### Step 2 — Scan for dead code

1. Run `make lint` — `cargo clippy` already denies `dead_code`, `unused_imports`, `unused_variables`, `unused_mut`, `unused_assignments`, `unused_results` per [Cargo.toml](Cargo.toml) workspace lints. Treat every warning as dead-code evidence.
2. For each candidate, `grep` the entire workspace (including tests, fixtures, [docs/](docs/)) for references. Only mark as dead if truly zero references.
3. List all dead code found with file paths and line numbers. Do NOT delete yet.

### Step 3 — Scan for duplicate code

1. Look for functions/methods with identical or near-identical logic across [crates/deslop-core/src](crates/deslop-core/src) and [crates/deslop/src](crates/deslop/src).
2. Look for copy-pasted blocks (same structure, maybe different identifiers — Type-2 clones).
3. Check across pipeline stages (discover → parse → normalize → fingerprint → cluster → LSH → embed → fuse → rank → render). Duplicates often hide between adjacent stages.
4. Re-use the tool on itself: `cargo run --release -- <path>` against this repo and read [deslop-report.txt](deslop-report.txt). Dogfooding is the first-class duplicate signal.
5. For each duplicate pair: note both locations, what they do, and how they differ (if at all). Do NOT merge yet.

### Step 4 — Scan for duplicate tests

1. Look for E2E tests that assert the same rendered-report behaviour against the same fixture.
2. Look for duplicated fixture directories or helper functions across [crates/deslop/tests](crates/deslop/tests).
3. If an integration-level E2E fully covers what a narrower E2E also covers, mark the narrower one redundant. Per [AGENTS.md](AGENTS.md): coarse E2E tests only — never delete a failing test, never remove assertions.
4. List all duplicate tests found. Do NOT delete yet.

### Step 5 — Apply changes (one at a time)

For each change, follow: **change → `make test` → verify coverage → continue or revert**.

#### 5a. Remove dead code
- Delete dead code identified in Step 2.
- After each deletion: run `make test` (fail-fast + coverage + threshold).
- If `make test` exits non-zero (test failure OR coverage drop): **revert immediately** and investigate.

#### 5b. Merge duplicate code
- For each duplicate pair: extract shared logic into [crates/deslop-core](crates/deslop-core) (the library owns all non-trivial logic — the binary is <50 LOC of glue per [AGENTS.md](AGENTS.md)).
- Update call sites to use the shared version.
- After each merge: run `make test`.
- If tests fail: **revert immediately**. The duplicates may have subtle differences you missed.
- If coverage drops: add E2E tests exercising the shared code before proceeding.

#### 5c. Remove duplicate tests
- Delete the redundant test (keep the more thorough one). Never remove an assertion per [AGENTS.md](AGENTS.md).
- After each deletion: run `make test`.
- If coverage drops below threshold, **revert immediately** — the "duplicate" was covering something the other wasn't.

### Step 6 — Final verification

1. Run `make lint` — clippy must pass with zero warnings under `-D warnings`.
2. Run `make test` — tests must pass AND coverage must remain ≥ the baseline from Step 1.
3. If coverage rose, ratchet [coverage-thresholds.json](coverage-thresholds.json) up in the same PR (subtract 1% rounding buffer per [COVERAGE-THRESHOLDS-JSON]).
4. Report: what was removed, what was merged, final coverage vs baseline.

## Rules

- **No test coverage = do not touch.** If a file has no E2E coverage, leave it alone entirely.
- **Coverage must not drop.** The Step 1 floor is sacred. Revert on any regression.
- **One change at a time.** Make one dedup change, run `make test`, verify coverage. Never batch.
- **When in doubt, leave it.** False dedup is worse than duplication. Only merge when you are 100% sure behaviour is identical.
- **Preserve public API surface.** Do not change public function signatures, crate-level exports, or CLI flags without a spec update in [docs/specs/SPEC.md](docs/specs/SPEC.md).
- **Three similar lines is fine.** Only dedup when the shared logic is substantial (>10 lines) or there are 3+ copies.
- **No linter suppressions.** `#[allow(clippy::...)]` is ⛔️ ILLEGAL per [AGENTS.md](AGENTS.md). Fix the underlying code.
- **No regex on source.** Tree-sitter only — the tool's own codebase must obey its own rules.
