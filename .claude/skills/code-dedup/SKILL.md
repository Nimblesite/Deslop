---
name: code-dedup
description: Searches for duplicate code, duplicate tests, and dead code in the Deslop Rust workspace, then safely merges or removes them. Use when the user says "deduplicate", "find duplicates", "remove dead code", "DRY up", or "code dedup". Requires test coverage — refuses to touch untested code.
---

<!-- agent-pmo:b636503 -->

# Code Dedup

Carefully search for duplicate code, duplicate tests, and dead code across the Rust workspace (`crates/deslop-core`, `crates/deslop`). Merge duplicates and delete dead code — but only when test coverage proves the change is safe.

## Prerequisites — hard gate

Before touching ANY code:

1. **Dogfood Deslop before editing.** Prefer the live MCP (`top-offenders`,
   `cluster-by-id`, `find-similar`). If MCP is unavailable, run the release CLI
   against the assigned scope with `target/release/deslop <scope> --output
   target/deslop-dedup --no-incremental` and inspect the generated JSON report.
   Build that binary first when absent. A wrong or stale result from either
   surface is an accuracy defect: stop and file a GitHub issue. Tool absence
   alone is not a blocker while the other surface works.
2. **Do NOT run `make test` — or any test — as part of this skill.** Deduping is a
   pure refactor. Tests run exactly once, at the very end, through the **ci-prep**
   skill (Step 6) — never before or between dedup edits.
3. Rust is statically typed — the compiler and ci-prep catch breakage.

## Steps

Copy this checklist and track progress:

```
Dedup Progress:
- [ ] Step 1: Deslop surface reachable, duplicate surface inventoried
- [ ] Step 2: Dead code scan complete
- [ ] Step 3: Duplicate code scan complete (via Deslop MCP)
- [ ] Step 4: Duplicate test scan complete
- [ ] Step 5: Changes applied (no tests run)
- [ ] Step 6: ci-prep skill run AFTER dedup — green
```

### Step 1 — Inventory the duplicate surface

1. Query the Deslop MCP `top-offenders` (worst-first) and note the clusters you
   intend to merge. When MCP is unavailable, scan the assigned scope with the
   release CLI command from the prerequisite and inspect the JSON report. This
   — not a test run — is where dedup starts.
2. Identify which files have E2E coverage in [crates/deslop/tests](crates/deslop/tests);
   prefer deduping covered files. ci-prep enforces the coverage floor at the end.
3. Do NOT run `make test` here. Do not run it anywhere except via ci-prep in Step 6.

### Step 2 — Scan for dead code

1. Run `make lint` — `cargo clippy` already denies `dead_code`, `unused_imports`, `unused_variables`, `unused_mut`, `unused_assignments`, `unused_results` per [Cargo.toml](Cargo.toml) workspace lints. Treat every warning as dead-code evidence.
2. For each candidate, `grep` the entire workspace (including tests, fixtures, [docs/](docs/)) for references. Only mark as dead if truly zero references.
3. List all dead code found with file paths and line numbers. Do NOT delete yet.

### Step 3 — Scan for duplicate code

1. Look for functions/methods with identical or near-identical logic across [crates/deslop-core/src](crates/deslop-core/src) and [crates/deslop/src](crates/deslop/src).
2. Look for copy-pasted blocks (same structure, maybe different identifiers — Type-2 clones).
3. Check across pipeline stages (discover → parse → normalize → fingerprint → cluster → LSH → embed → fuse → rank → render). Duplicates often hide between adjacent stages.
4. Dogfood the live Deslop MCP — `top-offenders`, then `cluster-by-id` for each
   cluster you'll merge. If MCP is unavailable, use the release CLI fallback
   from Step 1. Do not run tests for this inventory.
5. For each duplicate pair: note both locations, what they do, and how they differ (if at all). Do NOT merge yet.

### Step 4 — Scan for duplicate tests

1. Look for E2E tests that assert the same rendered-report behaviour against the same fixture.
2. Look for duplicated fixture directories or helper functions across [crates/deslop/tests](crates/deslop/tests).
3. If an integration-level E2E fully covers what a narrower E2E also covers, mark the narrower one redundant. Per [CLAUDE.md](CLAUDE.md): coarse E2E tests only — never delete a failing test, never remove assertions.
4. List all duplicate tests found. Do NOT delete yet.

### Step 5 — Apply changes (one at a time)

Make one dedup change at a time so a mistake is easy to isolate. **Do NOT run
`make test` between changes** — the compiler catches type breakage and ci-prep
(Step 6) is the single validation gate.

#### 5a. Remove dead code
- Delete dead code identified in Step 2.

#### 5b. Merge duplicate code
- Extract shared logic into [crates/deslop-core](crates/deslop-core) (the library
  owns all non-trivial logic — the binary is <50 LOC of glue per [CLAUDE.md](CLAUDE.md)).
  Shared test scaffolding belongs in each test crate's `tests/common/mod.rs`.
- Update call sites to use the shared version.

#### 5c. Remove duplicate tests
- Delete the redundant test (keep the more thorough one). Never remove an
  assertion per [CLAUDE.md](CLAUDE.md).

### Step 6 — Validate with ci-prep (AFTER all dedup)

Run the **ci-prep** skill now — and only now. It runs lint, the full test suite,
and the coverage gate exactly as CI does. This is the first and only time tests
run in this workflow.

1. Invoke the **ci-prep** skill. If it reports failures, fix them and re-run it
   until green.
2. If coverage rose, ratchet [coverage-thresholds.json](coverage-thresholds.json)
   up in the same PR (−1% rounding buffer per [COVERAGE-THRESHOLDS-JSON]).
3. Report: what was removed, what was merged, and the ci-prep result.

## Rules

- **No test coverage = do not touch.** If a file has no E2E coverage, leave it alone entirely.
- **Coverage is enforced by ci-prep, at the end — not mid-dedup.** Never run `make test` while deduping. If ci-prep shows coverage dropped, the "duplicate" was covering something — restore it.
- **One change at a time.** Make one dedup change, then the next. Never batch, so a mistake is easy to isolate when ci-prep runs.
- **When in doubt, leave it.** False dedup is worse than duplication. Only merge when you are 100% sure behaviour is identical.
- **Preserve public API surface.** Do not change public function signatures, crate-level exports, or CLI flags without a spec update in [docs/specs/SPEC.md](docs/specs/SPEC.md).
- **Three similar lines is fine.** Only dedup when the shared logic is substantial (>10 lines) or there are 3+ copies.
- **No linter suppressions.** `#[allow(clippy::...)]` is ⛔️ ILLEGAL per [CLAUDE.md](CLAUDE.md). Fix the underlying code.
- **No regex on source.** Tree-sitter only — the tool's own codebase must obey its own rules.
