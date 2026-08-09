# Open bugs

Triage snapshot of every open issue typed **Bug** — 28 of the 45 open issues.

**Showstopper** means do not release without fixing. **Critical** means it corrupts results, blocks a supported platform, or breaks the agent-facing gate, but a release could ship around it.

Sorted worst first. Counts: 7 showstopper, 19 critical, 2 normal.

---

## Showstoppers

| # | Issue | Area | Summary |
|---|---|---|---|
| [#309](https://github.com/Nimblesite/Deslop/issues/309) | `find-similar` returns empty for a verbatim copy of unclustered corpus code | MCP / recall | The prevention gate misses the exact case it exists for. A byte-for-byte copy of code that currently appears **once** in the corpus returns zero clusters, because snippet search only matches already-clustered code. Every agent following rule zero is told "no duplicate found" at the precise moment it is about to create the first copy. |
| [#301](https://github.com/Nimblesite/Deslop/issues/301) | `duplication_percent` is nondeterministic across runs | Engine / determinism | The same byte-identical tree yields a different duplication figure on every run. Originally 0.191pp of spread over 9 runs; corpus testing since measured cluster-count drift in **8 of 9 languages** and a 1.8-point swing on Flutter. `--fail-over` gates CI on this number, so identical code can pass or fail at random, and no snapshot test can exist. |
| [#276](https://github.com/Nimblesite/Deslop/issues/276) | CLI and LSP/MCP run different engine builds | Architecture | The CI gate and the editor panel are built and versioned independently and can carry different language support, so they analyse different corpora on the same working tree. The panel shows green while CI exits `3`, or the reverse. A gate that contradicts the product's own UI is worse than no gate. |
| [#331](https://github.com/Nimblesite/Deslop/issues/331) | Flutter widget declaration boilerplate ranks #1 on flutter/flutter | Precision / Dart | The single worst offender reported across all of Flutter is the `StatefulWidget` declaration the framework *requires* every widget to write. It cannot be extracted. Ranks 0, 1 and 4 are all mandatory scaffolding, pushing every genuine duplicate below unactionable noise. |
| [#336](https://github.com/Nimblesite/Deslop/issues/336) | Numeric array literals rank #1 on dotnet/fsharp | Precision / all non-Dart | The top F# cluster is an integer array literal with 3,544 occurrences, categorised `logic`. `CloneCategory::data` exists but its only classifier is Dart-only, so **7 of the top 10** F# clusters are data tables at full logic weight. F# reports 42.5% duplication. |
| [#166](https://github.com/Nimblesite/Deslop/issues/166) | CLI is single-threaded and holds the whole corpus in RAM | Performance | Cold analysis peaked at **13.8 GB / 425 s** on Flutter and 13.4 GB on the F# compiler. Standard GitHub runners have 7 GB, so the GitHub Action this project ships would be OOM-killed on either. Memory has grown ~4 GB since the issue was filed. |
| [#173](https://github.com/Nimblesite/Deslop/issues/173) | "Open Cluster Details" renders a blank panel on large/active repos | VSIX | On any large repository under live analysis, the command almost always opens an empty "No cluster selected" panel instead of the clicked cluster, because the selection is invalidated by a concurrent rescan. The primary way to inspect a duplicate is unusable exactly where duplicates matter most. |

---

## Critical

| # | Issue | Area | Summary |
|---|---|---|---|
| [#264](https://github.com/Nimblesite/Deslop/issues/264) | `find-similar` snippet query misses a tracked structural cluster | MCP / recall | A snippet copied near-verbatim from both occurrences of a tracked 457-node cluster returned zero results, while `report-for-file` returned the cluster correctly. Corpus state is healthy — only the snippet-input path fails, so the agent-facing gate is less reliable than the file-based one. |
| [#263](https://github.com/Nimblesite/Deslop/issues/263) | `find-similar` language enum omits TypeScript/JavaScript | MCP | The tool schema restricts `language` to csharp/rust/python/dart even though the engine has supported JS and TS for many releases and path-based queries over `.ts` files work. Snippet-mode gating is simply unavailable for two shipped languages. |
| [#262](https://github.com/Nimblesite/Deslop/issues/262) | MCP cannot gate TypeScript/JavaScript code units in this repo | MCP / dogfooding | The MCP bundled in the installed VSIX predates JS/TS support, so agents working on `clients/vscode/` cannot run rule zero on the extension's own TypeScript and must fall back to grep — the exact failure mode the rule exists to prevent. |
| [#252](https://github.com/Nimblesite/Deslop/issues/252) | MCP transport closes on Codex tool calls | MCP / IPC | The Deslop MCP appears as a reachable tool namespace but every call — including `schema-doc` and `session-config` — fails before returning data. The gate is fully unavailable to that client, not degraded. |
| [#228](https://github.com/Nimblesite/Deslop/issues/228) | VSIX DUPLICATION panel out of sync with live engine | VSIX / reactivity | The tree showed a stale cluster ranked #1 that the live engine no longer reports at all. The watcher → scheduler → session → broadcast → UI loop is not converging, so the panel shows a ranking that no longer exists. |
| [#314](https://github.com/Nimblesite/Deslop/issues/314) | LSP re-renders unchanged files and floods no-op notifications | LSP / performance | A production workspace logged 53 whole-corpus renders in about two hours, including 24 consecutive renders in five minutes, each re-flattening the corpus with nothing changed. Wasted CPU plus notification churn in the editor. |
| [#289](https://github.com/Nimblesite/Deslop/issues/289) | Ignored-path event storms still schedule a full refresh | LSP / performance | Admission control is correct (no ignored file is analysed) but every storm — even one containing only deletions of ignored paths — still triggers a ~770 MB, ~40 s transient refresh. A batch in which every path is excluded should be a no-op. |
| [#292](https://github.com/Nimblesite/Deslop/issues/292) | `top-offenders` transiently indexes ignored Playwright report assets | Discovery | Generated Playwright HTML and trace assets surfaced as the highest-weight clusters, displacing real TypeScript source from the results entirely. Generated output is being ranked above hand-written code. |
| [#298](https://github.com/Nimblesite/Deslop/issues/298) | `out/` missing from built-in default excludes | Discovery | `out/` is the standard TypeScript build directory for VS Code extensions. Without it, every `.ts` source double-counts against its transpiled `.js`, and compiled tests cross-duplicate heavily, inflating the metric on any extension repo out of the box. |
| [#283](https://github.com/Nimblesite/Deslop/issues/283) | Type-3 cluster conflates unrelated object-literal tables | Precision / TS | Unrelated object literals are grouped as one duplicate purely on shape after identifier and literal normalisation. The data they carry is different; only the syntax matches. |
| [#284](https://github.com/Nimblesite/Deslop/issues/284) | Type-3 cluster groups unrelated TDBIN test scenarios | Precision / TS | Distinct test scenarios are merged into a single cluster because their scaffolding is structurally identical, despite exercising unrelated behaviour. |
| [#285](https://github.com/Nimblesite/Deslop/issues/285) | Type-3 cluster groups unrelated TDBIN diagnostic tests by assertion idiom | Precision / TS | Tests sharing only a common assertion idiom cluster at `structural=1.00, token_jaccard=1.00, fused=1.00`. The signals cannot distinguish "same idiom" from "same code". |
| [#103](https://github.com/Nimblesite/Deslop/issues/103) | pytest idioms cluster as extractable duplicates | Precision / Python | After removing genuinely extractable clones, ~85% of remaining high-weight clusters were pytest patterns — `monkeypatch.setenv` chains, dict-access assertions, fixture call sites — that carry no shared logic and cannot be extracted. |
| [#79](https://github.com/Nimblesite/Deslop/issues/79) | Helper-call sites with literal arguments flagged as Type-2 clones | Precision | Repeated one-line calls to an already-extracted helper, differing only in their literal arguments, are reported as duplication. The extraction the tool would recommend has already been done. |
| [#71](https://github.com/Nimblesite/Deslop/issues/71) | Same HTTP verb + status assertion across different endpoints | Precision / Python | Eight tests hitting different endpoints cluster because they share a DELETE-plus-204 assertion shape. Same idiom, different subject under test. |
| [#167](https://github.com/Nimblesite/Deslop/issues/167) | Dart grammar produces ERROR nodes on declarative-constructor syntax | Parsing / Dart | The pinned `tree-sitter-dart` fork cannot parse experimental `new name(...) : init` syntax. Clean-parse rate is 99.59% over 2,372 real files and every failure traces to this one construct. No crashes — error recovery degrades to ERROR nodes that flow through harmlessly. |
| [#290](https://github.com/Nimblesite/Deslop/issues/290) | `wire_edit::file_uri` renders malformed URIs for Windows paths | Windows / autofix | `C:\work\lib.rs` becomes `file://C%3A%5Cwork%5Clib.rs` — colon and backslashes percent-encoded, drive letter in the authority position, missing third slash. Windows LSP and MCP clients cannot resolve these, so multi-file consolidation edits fail. Unix URIs are correct. |
| [#316](https://github.com/Nimblesite/Deslop/issues/316) | `make deployment-verify` unrunnable on Windows checkouts | Windows / release | Three of four contract suites fail on Windows for environment reasons alone — CRLF endings and NTFS exec bits — with no source defect. They pass on CI, so the release gate is unavailable on the primary development platform. |
| [#312](https://github.com/Nimblesite/Deslop/issues/312) | Flaky: `refresh_command_re_evaluates_the_corpus_after_an_edit` | Tests | The test races the live watcher: it passes locally and fails on CI at the identical commit. A flaky test in the reactive path erodes confidence in exactly the loop hardest to verify. |

---

## Normal

| # | Issue | Area | Summary |
|---|---|---|---|
| [#171](https://github.com/Nimblesite/Deslop/issues/171) | Duplicate hover card missing while a file is being edited | VSIX | During active editing the LSP duplicate *diagnostic* still appears but the Deslop hover card with Compare / View cluster / Copy for AI does not, so the actions are unreachable mid-edit. Language-agnostic. |
| [#250](https://github.com/Nimblesite/Deslop/issues/250) | JetBrains Gradle Java target does not match the 2024.3 platform spec | JetBrains / spec | Both Gradle modules pin `jvmToolchain(17)` while `docs/specs/jetbrains.md` sets the baseline at the 2024.3 platform line. Tagged `spec-violation`: code and approved spec disagree. |

---

## Coverage by the corpus suite

`make test-corpus` now fails on four of these against real pinned repositories, so they cannot regress unnoticed:

| Issue | Gate |
|---|---|
| [#331](https://github.com/Nimblesite/Deslop/issues/331) | `corpus_flutter_dart` — fails when mandated framework boilerplate ranks in the top 5 |
| [#336](https://github.com/Nimblesite/Deslop/issues/336) | `corpus_fsharp` — fails when a top-10 cluster is ≥60% numeric characters yet categorised `logic` |
| [#166](https://github.com/Nimblesite/Deslop/issues/166) | every `corpus_*` test — fails when peak RSS exceeds the 7 GB runner budget |
| [#301](https://github.com/Nimblesite/Deslop/issues/301) | `corpus_determinism_*` — scans twice, fails when cluster count, `duplication_percent`, or cluster ids differ |

### How this runs in CI without blocking merges

The `Corpus accuracy` workflow is scheduled + manual only — it has **no `pull_request` trigger**, so it can never sit in the merge path while these defects are open.

It runs in *baseline mode*: the checks listed in `corpus/known-failures.json` are printed to the job summary and pass; anything **not** listed fails the run. That is the regression gate — today's defects are visible without blocking, and a new one is loud.

Scheduled runs scan **two repositories, not nine** — `tokio` (fastest, and the only corpus ever stable across runs, so it acts as the control) and `nest` (cheapest repository that still reproduces #301). Clone plus scan plus build measures **~34 s**.

- `make test-corpus` locally ignores the baseline entirely, scans all nine, and fails on everything. Local runs stay honest.
- Baseline keys are rank-independent (`memory`, `boilerplate_rank`, …) because #301 moves cluster ranks between runs.
- A baseline entry that stops firing is reported as `[FIXED?]` but never fails a run — with #301 open, one lucky pass is not proof. Reconciliation is scoped to the checks a given gate actually evaluates, so the determinism gate cannot declare the memory gate fixed.
- **The scheduled slice does not gate precision.** #331 (Dart) and #336 (F#) live in `flutter` and `fsharp`, which peak above 13 GB (#166) and take minutes each. Run the full corpus via `workflow_dispatch` with `full`, or locally.

The remaining 24 bugs have no corpus-level gate. Seven corpus repositories still print `ACCURACY UNASSERTED`: they check resource ceilings only, and a pass there is not evidence that detection on them is correct.
