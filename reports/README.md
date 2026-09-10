# Reports — live findings

**This file is an index, not a measurement.** It carries forward only the findings from ten earlier reports that are still true against the working tree, each labelled with the report and the generator that produced it. Every number here was produced mechanically by the tool named beside it; nothing in this file was computed by hand. The ten source reports were deleted — their evidence directories under `target/` are gone, so their links were dead and their figures were no longer reproducible. Anything worth re-measuring must be re-run, not read out of here.

Verified against the tree on 2026-09-09 at `bd920260` (branch `fix/regression-rollbacks`).

**Correction (2026-09-10).** An earlier revision of this file listed the `calls` accuracy quarantine under §2 as discharged. That was wrong: the quarantine moved into the new `calls/` submodule rather than being lifted, and it is live. It is now item 0 above.

---

## 1. Act now

| # | Item | State | Evidence |
|---|---|---|---|
| 0 | **The `calls` accuracy quarantine is LIVE, and 103 `deslop-core` tests fail on it.** `is_statement_shape` is a mandated `panic!` at `cluster_filters/calls/statements.rs:119` — the deleted allowlist omitted Rust `let_declaration`, so a registry-and-registration run had no covered statements and escaped scaffolding suppression. Correct per AGENTS.md and **must not be worked around**; the exit is to restore the classifier and turn the pin green. | Red, by mandate | 23 lib + 80 suite failures, one panic site; pinned by `registry_call_payload_variation_keeps_only_authored_control` |
| 1 | **gh #520–#526 are fixed but still open.** All six verification checks exit 0. AGENTS.md forbids agents closing issues — a human closes these. | Ready to close | `scripts/repository/regression-verification.py`, 6 checks / 0 failing |
| 2 | **gh #369 — CRITICAL, 4 tests ignored.** Embedding-only false positives survive on MockOllama's length-residue cosine alone (structural 0, token_jaccard 0). The stated fix has an O(N²·D) cost. | Open, blocking | `grep -rn SKIP-UNFINISHED crates/` → 4 × GH #369 |
| 3 | **gh #356 — embeddings-on mutates buckets.** `ts-mixed-band` publishes a four-file clone with embeddings off and nothing with them on; ANN bridges mutate structural components before measurement (`session/render.rs`). `csharp-type3` follows the discovery route, not the code. Must stay red, not baselined. | Open | ignored-test reasons, commit-f92300e5 review |
| 4 | **`Vec<Signature>` still holds 1 KiB per fingerprint** (~3.5 M fingerprints on the Flutter clone ⇒ ~3.5 GB). Sharing via `Arc` from the existing signature memo is the value-preserving fix and is not done. | Not started | `crates/deslop-core/src/pipeline/session/store.rs:64`, `fpcache.rs:65` |
| 5 | **LSH bucket pair materialisation is still quadratic in occurrence count.** Buckets over duplicated framework boilerplate hold thousands of members; every within-bucket pair legitimately survives (jaccard 1.0), so no pre-filter can drop them. Fix is to collapse equal-signature bucket members into one group with multiplicity, or stream pair generation. | Not started | Flutter Windows profiling, 2026-08-30 |
| 6 | **Merge goldens are keyed on a cluster-hash suffix.** `MergedFromCluster_7023e4` / `mergedFromCluster_22921e` / `merged_from_cluster_ff0d17` are committed in fixtures. A tree-sitter comparison showed all remaining AST tokens equal — only the hash differs — so these three tests fail on cluster-id churn, not on merge behaviour. Normalise the helper name or derive the golden. | Live fragility | `crates/deslop-lsp/tests/fixtures/code_action/*.merged.*` |
| 7 | **`fsharp: ["memory"]` is the only entry left in the corpus ratchet.** Deleting it by fixing the defect is the only correct exit. | Open | `corpus/known-failures.json` |
| 8 | **Corpus manifests cannot express same-file pairs.** 17 of the 35 hand-verified Tornado verdicts — including the strongest Type-1 evidence — are unassertable and survive only as prose in `must_find_status`. Region-precise non-duplication claims are likewise inexpressible. A same-file assertion form is the fix. | Schema gap | `corpus/tornado.json` status prose |

## 2. Resolved — do not re-open

- **gh #526's 36 orphaned spec identifiers are all resolved.** `scripts/repository/spec-crossrefs.py` re-run over 40 requested identifiers reports **unresolved references: 0**; every one now links to a definition. The issue text is stale. (The identifiers were mostly renamed, not deleted — e.g. `[FUSED-RANK-MASS]` → `[RANK-MASS-SUM]`, `[VSIX-SETTINGS-RANKING]` → `[RANK-STRUCTURAL-ONLY]`, `[DESLOP-LIVE]` → `[LIVE-SCHEDULER]`.)
- **The Flutter performance gate passes.** `corpus/flutter.json` ceilings are now re-derived from a completed measured scan — **295 s wall / 7,947 MB peak RSS** against ceilings of 700 s / 9,000 MB — and Flutter carries no entry in `known-failures.json`. Three reports asserting the corpus could not finish are obsolete. Note the RSS headroom is thin (~12%); items 4 and 5 above are what protects it.

## 3. Ground truth worth keeping — Tornado, 35 hand-verified pairs

Curated 2026-09-03 against `tornadoweb/tornado` v6.5.8, comparing `f92300e5` (#368) against `b5273c16` (#501), each a clean-room release build with sha256-verified distinct binaries. Six cross-file verdicts are now enforced in `corpus/tornado.json`; the rest are prose ground truth. Verdict split: **24 real duplicates, 5 shape-only false positives (all old-build), 2 boilerplate suppressions, 6 real duplications only the new build finds.**

**The new build is unambiguously better on this corpus.** It publishes no confirmed false positive, and it finds two production-code duplications the old build missed — `locks.py:129-138` ↔ `queues.py:61-70` (the `on_timeout` closure) and `curl_httpclient.py:446-453` ↔ `simple_httpclient.py:412-419` (a body guard differing only by indent and receiver). The second is exactly the drift duplication detection exists to catch: two HTTP clients diverging from one copy of the same guard.

**Watch items — the only live precision risk on this corpus:**

| Pair | Code | Why it's a watch item |
|---|---|---|
| 13 | `auth.py:492-521` ↔ `iostream.py:294-314` | Abstract-stub idiom, docstring-heavy. Reported faint by the new build only. A fixture family already exists: `python_issue_69_abstract_method`. |
| 20 | `escape_test.py:293-297` ↔ `template_test.py:411-420` | Shape-only run of `assertEqual` (`json_decode` vs `render`). Published by **both** builds; old build as `structural_only`, new build faint at low rank. |
| 35 | `blog.py:189-191` ↔ `auth.py:384-386` | Shape-only `get_argument` runs. New build only, weakest finding in the set. |

The canonical old-build false positive, worth keeping as the reference case: `auth_test.py:501-505` ↔ `web_test.py:2314-2318` — unrelated OAuth tests versus handler methods, published `structural_only` at embedding 0.0, with token_jaccard reaching 1.0 only after identifiers were stripped. The starkest was `docs/conf.py:26-31` ↔ `auth.py:742-746` — Sphinx config against OAuth URL constants, published at fused 0.18.

## 4. Regression attribution — which change broke what

Measured against `main` at `77b4681b` by `scripts/repository/regression-attribution.py`; every row is a `git log -S` result re-checked by counting the anchor string at the commit and at its parent. Kept because it is the map from a defect back to the change that introduced it, and it cannot be re-derived cheaply.

| gh | Introduced by | Commit | What changed |
|---|---|---|---|
| #520 | #518 | `8324e479` | Literal-variation filter began bailing out on any body-carrying call; structural family split stage added to the cluster pipeline |
| #521 | #485 | `1ecfc997` | Cluster colour table rekeyed from clone kind to rank band; the assertion that a demoted family is not painted act-now was deleted |
| #522 | #36, #63, #392, #485 | `dcc12296`, `345c7d35`, `2dae2a5b`, `1ecfc997` | Four separate colour tables accreted — percentile ramp, chart palette, evidence-keyed paint, then the paint table fed from the rank band |
| #523 | #144, #485 | `9cec1e82`, `1ecfc997` | IPC report deserialised with no version negotiation; a wire field rename then fired the unversioned parse |
| #524 | #485 | `1ecfc997` | Compare-against-canonical removed (40 occurrences → 4); per-row two-step selection added |
| #525 | #420, #485 | `42b2c928`, `1ecfc997` | Two-decimal helper added, then integer mass routed through it and the wire field made an integer |

`1ecfc997` (#485) is implicated in five of the six. Treat it as the highest-risk change in recent history.

The measured detector behaviour across that range, one frozen copy of `site/tests` scanned by a release build of each commit, shows where accuracy moved:

```
f92300e5  mixed= 2/27  pct=15.22   fused confidence: honest cluster signals, content gate
42b2c928  mixed=13/39  pct=20.74   measure structural as subtree overlap
b235c1a5  mixed= 4/11  pct= 6.61   fused-score accuracy follow-ups, verbatim subgroups
1ecfc997  mixed= 2/5   pct= 4.44   render no pair-only evidence on cluster surfaces
8324e479  mixed= 9/30  pct=21.92   independent clone registers, corpus accuracy gate
77b4681b  mixed= 9/31  pct=22.01   find duplicated code inside a single file (#508)
```

Post-fix, the same fixture at `--min-nodes 30` reads **12.14% / 17 clusters** on this branch against **22.01% / 31 clusters** on main, and the pure statement-run fixture — where a mixed span count above zero *is* the #520 defect — publishes nothing at any floor (12/30/45), where main published 8/3/1 clusters with a mixed span in every case.

## 5. What was deleted, and why

| File | Verdict |
|---|---|
| `spec-crossrefs-2026-09-07.md` | Byte-level duplicate of the file below — same generator, same title, 36 of the same 40 identifiers. |
| `regression-restoration-2026-09-07.md` | Misnamed: it *is* the spec cross-reference audit, not a restoration report. All `target/deslop-astra-check/` and `target/deslop-compare-validation/` evidence is gone. Its one durable fact is §2. |
| `regression-attribution-2026-09-07.md` | Attribution table preserved in §4. Its 36-orphan table is superseded by the 0-unresolved re-run. |
| `regression-verification-2026-09-08.md` | Result preserved in §1 item 1 and §4. |
| `regression-revert-quarantine-2026-09-07.md` | The quarantine it documents is discharged; the panic is gone and the assertion stands. |
| `tornado-corpus-curation-2026-09-03.md` | Verdicts preserved in §3; six now enforced in `corpus/tornado.json`. |
| `commit-f92300e5-review.md` | Untracked; all `target/commit-f92300e5-review/` logs gone. Live content preserved as §1 items 2, 3 and 6. |
| `flutter-corpus-2026-08-23.md` | A run that timed out and emitted no metrics. Nothing to preserve. |
| `flutter-performance-fix-status-2026-08-23.md` | Asserted "not fixed"; superseded by a passing gate. Cited `docs/BRANCH_REVIEW.md`, which no longer exists. |
| `flutter-corpus-windows-2026-08-30.md` | Profiling narrative superseded by the passing gate. The two structural fixes it named and did not make are §1 items 4 and 5. |
