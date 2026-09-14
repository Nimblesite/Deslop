# VSIX Testing API migration — measure the artifact that ships

Owns [gh #440](https://github.com/Nimblesite/Deslop/issues/440). Spec: [`vsix.md` §VSIX-TESTING-COVERAGE](../specs/vsix.md#vsix-testing-coverage).

> **The extension-host coverage number must describe `dist/extension.js` — the code that actually ships in the VSIX — measured by the same run that proves the suite passes.**

## What is already true (do not redo it)

#440 was filed on a correct observation and a wrong conclusion.

**Correct:** the desktop extension host writes no V8 profile for extension code. A raw `NODE_V8_COVERAGE` run captures **1171 script entries, none of them ours** — that profile belongs to VS Code's main process. Re-aiming `c8` at a different artifact cannot recover a profile that was never taken. `@vscode/test-cli@0.0.15` does not change this: `--coverage` scoped to our files reports `0/0`.

**Wrong:** that this makes the coverage unmeasurable. It makes it unmeasurable *through V8*. The counters now travel inside the code instead — `scripts/instrument-out.mjs` compiles Istanbul counters into every module, `src/test/coverage-dump.ts` writes the table from inside the host on exit, and `scripts/extension-coverage.mjs` merges and reports it. **87.6% across all 43 compiled modules**, gated at `vsix.extension_threshold` and failing CI below it.

So the floor #440 asked for exists, and the migration is no longer about *getting* a number.

## Why migrate anyway

Two things the current gate cannot do, both real:

1. **It measures `out/**`, not `dist/extension.js`.** Same sources, different compiler. The shipped bundle is never the thing measured. The two copies cannot simply be merged — esbuild and `tsc` produce different statement maps, so istanbul would total tables that do not describe the same code.
2. **It under-reports.** Whatever only the bundle's activation path executes scores zero. That is the safe direction for a floor — it can never claim coverage no test produced — but it is still wrong, and it means E2E work is invisible to the number.

A Testing API run (`vscode.tests.createTestController` + `TestRunProfileKind.Coverage`) is instrumented by VS Code itself against the loaded extension, which is the bundle. That closes both.

## The blocker this migration must solve first

The suites currently pass **partly because the extension host and the unit tests load two different copies of the extension.** The host loads `dist/extension.js`; unit suites `import { … } from "../../extension"`, which resolves to `out/`. Two module instances, two sets of module-level state.

Collapsing them (verified by pointing `main` at `./out/extension.js` and running the suite) breaks tests that depend on the split:

- **`extension-glue.unit.test.ts` — `currentApi`.** ✅ **Already fixed.** It asserted the live handles were `undefined`, which was only true because *this* instance had never been activated. It now asserts the actual contract — every handle is a live read-through getter that reads without throwing — which holds in both layouts and covers one line more than the old assertion did.
- **`command-impls.unit.test.ts:170` — `activation keeps VSIX commands separate from namespaced LSP commands`.** ❌ **Open.** `assert.ok(commands.includes("deslop.lsp.refreshReport"))` fails under the shared-module layout. This is **not** obviously a bad assertion — it looks like real behavioural divergence (the namespaced LSP commands are never registered when the host loads the `out/` entry, suggesting the language client does not start). Diagnose before touching the test. **If the LSP genuinely fails to start from that entry point, the test is right and the migration is wrong until that is understood.**

Any further split-dependent tests surface the same way: flip `main`, run, read the failures.

## Checklist

### Phase 1 — Prove the ground is safe

- [ ] Diagnose `command-impls.unit.test.ts:170` under a shared module. Establish whether `deslop.lsp.*` registration genuinely breaks or the assertion encodes the split. **Do not modify the test until the cause is known.**
- [ ] Flip `main` to `./out/extension.js`, run the full suite, and record **every** failure — not just the first (`bail: true` hides the rest; drop it for this audit only).
- [ ] For each failure, classify: *assertion encodes the split* (fix the assertion, strictly stronger, never weaker) or *real divergence* (fix the product or stop the migration).
- [ ] Confirm no test depends on module-level state surviving between the host and a unit import.

### Phase 2 — Controller

- [ ] Add a `vscode.tests.createTestController` registration behind the test build only — it must never ship in the packaged VSIX ([VSIX-BUNDLE]).
- [ ] Enumerate the existing suites as `TestItem`s. Keep the current file layout; do not rewrite suite bodies in the same change.
- [ ] Add a `TestRunProfileKind.Run` profile and prove the full suite passes through it — **472 passing, matching the current count exactly.** A smaller number is a lost suite, not a passing migration.
- [ ] Add `TestRunProfileKind.Coverage` and confirm VS Code reports coverage for `dist/extension.js`.

### Phase 3 — Swap the gate

- [ ] Point `make _vsix-coverage` at the Testing API run.
- [ ] Map bundle coverage back to `src/**/*.ts` through the esbuild sourcemap; assert the module set matches the bundle's own sourcemap `sources`, so the denominator stays the whole shipped extension and an unloaded module scores 0% rather than vanishing.
- [ ] Re-measure. **Record the new number before setting the floor.** It will differ from 87.6% in both directions — E2E activation now counts, and bundling changes what a "line" is.
- [ ] Ratchet `vsix.extension_threshold` to the measured value. Never set a floor the code does not meet.
- [ ] Keep the fail-closed guards: no coverage written, an empty table, or a module-set mismatch must fail the run. A broken harness must never pass by default.

### Phase 4 — Retire the interim

- [ ] Delete `scripts/instrument-out.mjs`, `scripts/extension-coverage.mjs`, and `src/test/coverage-dump.ts` **only once the Testing API gate is green in CI** — not before.
- [ ] Drop `istanbul-lib-instrument` from `devDependencies` if nothing else uses it.
- [ ] Update [`vsix.md` §VSIX-TESTING-COVERAGE](../specs/vsix.md#vsix-testing-coverage) to describe the shipped-artifact measurement.
- [ ] Update [`release-audit.md`](../release-audit.md) with the new number.
- [ ] Report the outcome on #440 — including that the `NODE_V8_COVERAGE` premise was confirmed and the "unmeasurable" conclusion was not. **Do not close it; that is the maintainer's call.**

## Guardrails

- **Never lower a floor to make the migration green.** If the bundle measures lower than 87.6%, that is the honest number and the floor moves down only with the measurement written down and explained — the previous floor covered a different artifact, so it is not a ratchet violation, but it must be stated, not slipped in.
- **Never weaken an assertion to accommodate the harness.** Every test the migration touches must end up asserting more than it did, as `currentApi` now does.
- **A suite that did not run is not a pass** ([VSIX-SUITE-EXECUTES]). The Testing API run needs the same protection: assert the executed test count, not just the exit code.
