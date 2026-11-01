# Release review — v0.37.0 to HEAD

Scope: `git log v0.37.0..HEAD` and `git diff v0.37.0 HEAD`, from `adf93d9606fc26bf301dc7385ed89311bfa8af64` to `45a70433d169fb1bccf1c5ad420e6e5add76ea97`. I read the changed code at both revisions and traced each finding from a production entry point. Per the review constraints, I did not edit source, commit, build, or run the corpus. Findings below are static, reachable behavior changes, not performance observations or feature requests.

## Findings

### 1. High — Dispatcher rescue now admits changed operations as clones when only the type contract renamed

**HEAD:** `crates/deslop-core/src/content/call_targets.rs:359-371`, especially `:365`; contract collection is in `crates/deslop-core/src/content/call_targets/contracts.rs:63-80`.

**v0.37.0:** `selector_renames` excused a changed external selector only when the actual receiver at the same frontier position had a corroborated rename. A cast type was not a receiver. Its own comment explicitly used `client.startJob` versus `client.cancelJob` as a changed operation that must contradict clone admission.

**HEAD:** `receiver_site` appends the cast or type-guard contract to `receivers`. `selector_renames` now returns true when **any** receiver position has corroboration, so a repeated `Before` → `After` type rename excuses `startJob` → `cancelJob` even though the runtime receiver is unchanged. This is not limited to lifecycle method affixes or to a semantic-equivalence proof.

**Concrete trigger:** use the existing `hook_module!` fixture in `crates/deslop/tests/clone_extent_recovery/shutdown_hooks.rs` with the two existing type contracts `BeforeShutdown` / `AfterShutdown`, but replace the two selectors with `startJob` / `cancelJob`. The type rename appears repeatedly in the predicate and casts, so `Corroboration::admits` accepts it. Calls such as `.map(async instance => (instance as any as BeforeShutdown).startJob(signal))` and its `AfterShutdown` / `cancelJob` twin have the same runtime `instance`, but HEAD finds no `CallTargetChange::Other`. At v0.37.0 the cast supplied no receiver rename and the changed selector was a hard contradiction. A minimal pair is two otherwise identical functions containing repeated `(value as Before).startJob()` versus `(value as After).cancelJob()` calls inside corresponding `hasBefore` / `hasAfter` type guards.

**Reachability:** CLI/LSP/MCP analysis reaches `content::pair_contradiction` at `crates/deslop-core/src/content.rs:307-318`; `CallTargetSubstitution` is a hard admission contradiction at `:179-190`. Removing that contradiction allows the full copied dispatcher, rather than only its genuinely shared body, into the rendered cluster.

**Suggested fix:** do not let a type-contract rename alone excuse a selector change. Restore the actual-runtime-receiver requirement in `selector_renames`; keep contract metadata for extent recovery and the independent changed-operation checks, not as wildcard rename evidence. Add an end-to-end fixture based on the existing dispatcher test with `startJob` / `cancelJob`, asserting that no whole-module cluster is reported and only the unchanged shared body can survive.

### 2. High — Any malformed `osprey.toml` aborts the whole scan, even with no Osprey source

**HEAD:** `crates/deslop-core/src/lang/osprey_selection.rs:63-74` and `:100-113`; propagated by `crates/deslop-core/src/pipeline/session/initialise.rs:70`.

**v0.37.0:** discovery did not read `osprey.toml`. The file had no effect on a repository, and a malformed, non-UTF-8, or unreadable file by that name did not prevent analysis of ordinary Rust/Python/etc. sources.

**HEAD:** `OspreyProjects::load` walks the repository, reads every non-excluded file named `osprey.toml`, deserializes it, and propagates the first I/O or TOML error. `PipelineSession::initialise_from_sources` uses `?` on discovery, so no report is produced.

**Concrete trigger:** a non-Osprey repository containing valid `src/lib.rs` plus a root `osprey.toml` whose contents are `[project` previously scanned normally; `deslop .` now exits with `ConfigParse` before fingerprinting `lib.rs`. The same whole-run failure occurs for invalid UTF-8 or an I/O error reading that manifest.

**Reachability:** the batch CLI enters `pipeline::run` → `PipelineSession::initialise` → `initialise_from_sources` → `discover_files` → `OspreyProjects::load`. LSP startup and MCP-backed live scans use the same initializer. The new test `malformed_project_manifest_returns_a_visible_error` proves the failure is intentional and reachable; intent does not preserve the old input contract.

**Suggested fix:** ignore an invalid Osprey manifest with a structured warning, or quarantine only its candidate project, while allowing unrelated source analysis to continue. Do not parse unrelated manifests as fatal workspace configuration. Add a black-box test with no `.osp`/`.ospml` files and ordinary source beside a malformed root `osprey.toml`, asserting a successful report containing the ordinary duplicates.

### 3. High — A comment in an ordinary Python file can now abort the entire repository scan

**HEAD:** `crates/deslop-core/src/lang/osprey_selection.rs:77-96`, `:149-176`, and `:186-205`; source reading and propagation are in `crates/deslop-core/src/discover.rs:350-368`.

**v0.37.0:** a leading Python comment such as `# osprey: flavor=default` was just a comment. Python discovery and parsing continued normally.

**HEAD:** every discovered `.py` file is read and parsed during discovery for flavour selection. A marker selecting a flavour other than `osprey-python` produces `CoreError::LanguageSelection` (`flavor marker contradicts the extension`), which discovery propagates and the initializer turns into a whole-run failure. An unknown flavour also errors at `flavor_language` (`:215-223`).

**Concrete trigger:** an otherwise valid Python repository with `alpha.py` beginning `# osprey: flavor=default`—for example, a documentation comment copied from Osprey documentation—scanned at v0.37.0 but now exits before reporting any Python or other-language duplicates. The new `conflicting_flavor_marker_returns_a_visible_error` test reaches exactly this failure path.

**Reachability:** `eligible_file` calls `projects.language(...)?` before corpus construction for every Python file; both CLI and live initialization propagate the error. This requires no Osprey project or Osprey source file.

**Suggested fix:** treat a conflicting or unknown marker as inapplicable metadata and use the extension's ordinary parser, optionally with a warning; at worst skip only the offending file rather than aborting the workspace. Add a test showing ordinary Python plus another language still completes and reports its duplicates in the presence of this comment.

### 4. High — Extract code actions combine unsaved-report ranges with stale on-disk bytes

**HEAD:** `crates/deslop-lsp/src/backend.rs:393-416`, especially `:403`; unsaved reports enter through `didOpen` / `didChange` at `:333-353`. The mismatched plan is computed in `crates/deslop-lsp/src/code_action.rs:45-78` and `crates/deslop-core/src/refactor/mod.rs:56-88`.

**v0.37.0:** reports were disk-derived, and `textDocument/codeAction` read the same disk bytes. The cluster ranges, requested-range conversion, AST, precondition checks, and emitted edit positions therefore described one source version.

**HEAD:** `didOpen` and `didChange` replace the analysed source with the editor buffer, but `code_action` still calls `std::fs::read(&path)` and converts the editor's current LSP range using that old text. It then applies unsaved occurrence byte ranges to the old AST. This can suppress a valid action or generate an extract plan and `WorkspaceEdit` for the wrong statements.

**Concrete trigger:** save a C# file containing two identical methods, then insert a line or a multibyte character before them without saving. The new report has shifted occurrence offsets. Request `textDocument/codeAction` on either current method. HEAD maps that current range through the old disk string and parses old bytes using the shifted ranges; v0.37.0 used one consistent disk version. If old bytes at those offsets happen to form equivalent statement runs, the direct extract action can be offered with edits addressed to the wrong current-buffer locations.

**Reachability:** this is the production LSP `textDocument/codeAction` handler. The direct `refactor.extract` path is affected; lazy merge/consolidation resolution instead reads the pipeline's in-memory buffer and is not the basis of this finding.

**Suggested fix:** compute both the requested byte range and the refactor plan from the exact open-buffer snapshot used for the report. If that snapshot is unavailable or versions differ, return no action rather than an edit for another source version. Add a real-LSP test with an unsaved insertion before an identical pair, asserting both the offered plan and every resulting `TextEdit` against the unsaved document.

### 5. Medium — Cache-seeded startup acknowledges edits before analysing them, exposing the stale cached report

**HEAD:** `crates/deslop-lsp/src/backend_changes.rs:58-96`, especially `:86`; deferred behavior is in `crates/deslop-core/src/live/session.rs:203-222`; dirty-state acknowledgement is in `clients/vscode/src/reportStore.ts:299-310` and consumed by `clients/vscode/src/notifications.ts:106-128`.

**v0.37.0:** an edit during the cache-seed window was queued and produced an empty delta. `changed_paths_notification` returned `None` for every empty delta, so no report acknowledgement cleared the VSIX dirty guard. The stale occurrences stayed hidden until cold-pass replay produced a real report.

**HEAD:** `apply_buffer_change` always supplies `analysed_buffer`. The empty-delta suppression now applies only when that field is absent, so a deferred seed-only edit still emits `deslop/reportChanged` with the current editor version. `applyDelta` / `setSnapshot` calls `acknowledgeBuffer`, which sees the matching version and clears the dirty marker even though the report is still the old cache seed.

**Concrete trigger:** start the LSP with a cached report containing a clone, then edit that clone while the background cold pass is still building the pipeline. `AnalysisSession::apply_changes` sees `pipeline.is_none()`, queues the path, and returns an empty delta without analysing the buffer. The backend nevertheless publishes `analysed_buffer = { path, version }`; the VSIX clears its dirty projection and displays stale cached clone rows and offsets until deferred replay eventually finishes.

**Reachability:** this is the normal `deslop-lsp` cache-seeded startup path used by the VSIX. The later cold-pass notification in `crates/deslop-lsp/src/cache_seed.rs:173-179` has `analysed_buffer: None`, so it cannot undo the premature acknowledgement.

**Suggested fix:** distinguish deferred/no-op passes from analysed no-change passes. Never attach an analysed version while `is_seed_only()` or when the change was only queued. Retain buffer versions in the session and attach the correct acknowledgement only after `install_pipeline` has replayed that buffer. Add a cache-seeded integration test that edits during the cold pass and asserts the dirty projection cannot clear before the replayed report arrives.

### 6. Medium — The corpus gate now accepts 11 known false positives that v0.37.0 failed

**HEAD:** `corpus/register/outcomes.json` (for example the Composer `clearly_out` failure at `:1636-1652` and the React failure ending at `:3599`), interpreted by `crates/deslop-test-support/src/corpus_score/outcomes.rs:141-160` and `crates/deslop-test-support/src/corpus_score/verdict.rs:39-42`.

**v0.37.0:** `corpus/register/score-thresholds.json` had `maximum_false_negatives = 0` and `maximum_false_positives = 0`, with no per-repository exemptions. `Verdict::passed` required an empty failure list.

**HEAD:** `Outcome::Fail` plus a previous `Outcome::Fail` becomes `Standing::StillFailing`, which does not fail the run. The new ledger contains 324 pairs, 11 recorded as `fail`; all 11 are `clearly_out` pairs, i.e. known false positives: Composer 3, Dio 1, Giraffe 1, Hugo 5, and React 1. These are no longer gate failures.

**Concrete trigger:** run the full judged scorer on reports whose only wrong judgments are those recorded pairs, for example Hugo's `tpl/collections/init.go:151-164` paired with `tpl/urls/init.go:47-58`. At v0.37.0 this false positive breached the zero threshold. At HEAD it is `StillFailing`, so `Verdict::passed` accepts it and the gate can exit successfully without fixing the wrong answer.

**Reachability:** `corpus-score score --gate` builds pair standings and the verdict on every full corpus/comparison run. The default CI slice does not include these repositories, so its green status does not disprove this weakening; `make score-corpus` and full version comparisons use the affected records.

**Suggested fix:** restore zero tolerance for `clearly_out` false positives, and keep known wrong answers red until fixed. If a historical ledger is retained for reporting, do not let `StillFailing` make `Verdict::passed` true. Remove these fail records as exemptions and add a gate test proving each currently recorded `clearly_out` failure still fails the run.

## Area-by-area attack results

- **Detection, normalisation, clustering, clone kinds, and percentages:** I traced the changed call-target aligned and unaligned paths and found finding 1. I checked per-kind line-set union, total/per-file/folder sharing, legacy `total_includes` defaults, per-kind denominators, and gate verdicts. I found no reachable case where a new numerator exceeds its denominator. The lower default headline for excluded kinds is explicitly represented by `total_includes` and per-kind rows, so I did not relabel that policy change as a calculation bug.
- **Hard failures and unusual inputs:** I found findings 2 and 3 for bad manifests and marker conflicts. I did not count unreadable ordinary source as new: v0.37.0 already propagated source-read failures during corpus construction. I found no newly introduced failure for empty files, unknown extensions, non-followed symlinks, or odd source encoding beyond the new manifest/marker paths.
- **Osprey flavour selection on non-Osprey repositories:** both findings 2 and 3 require no Osprey source. I also checked contextual parser stamping and Python-flavour reuse; ordinary Python without a project or leading marker retains its old parser.
- **Unsaved buffers and the live loop:** I attacked incremental UTF-16 edits, multibyte offsets, buffer identity, deleted/closed buffers, cache-seed replay, version acknowledgements, and refactor source selection. Findings 4 and 5 are the demonstrated regressions. I did not claim a general out-of-order `didChange` race without a concrete scheduler interleaving.
- **MCP frames and long-path IPC:** I checked the 200 KiB boundary including the newline and actual request id, escaped and multibyte identifiers, split input through `BufRead::read_line`, nested rescan cursors after byte trimming, and Unix socket fallback to authenticated TCP for overlong paths. I found no boundary rejection or cursor corruption to report. Oversized tool responses are deliberately truncated with recovery metadata; non-tool success frames remain unbounded rather than newly rejected.
- **Tests and corpus records:** finding 6 is the material weakening. No test files were deleted in this range, and I found no new test skip or `#[ignore]` in the changed product tests. Metric assertions changed to account for the new include set; I did not count those coordinated changes as independent regressions.
- **Release and CI:** `.github/workflows/release.yml` and `scripts/release` did not change. The new required-`CI` merge gate runs under `always()` and rejects failed/cancelled or improperly skipped required jobs. I found no silently skipped publish step or newly pass-by-skipping gate in the changed workflow.
- **UI and report parity:** I checked restored hover occurrence links against command signatures, total-include presentation, generated wire defaults, and CLI/webview metric sourcing. Finding 4 is the demonstrated UI contract break. I found no additional CLI-versus-webview percentage disagreement attributable to this range.

## GO / NO-GO

**NO-GO.** HEAD introduces a concrete false-positive admission path, two whole-scan hard failures on inputs v0.37.0 analysed, an unsafe unsaved-buffer code-action path, a premature live-report acknowledgement, and a corpus gate that now tolerates 11 known false positives. Findings 1–4 should block release; findings 5–6 also require correction before the accuracy and live-loop contracts are restored.

## Verification against the source

The review above was written by a pi agent (`kimi-coding/k3.1-pro`). Each finding was then checked against the code, and findings 1–4 were reproduced by a test that failed before the fix.

| # | Verdict | Outcome | Pinned by |
| --- | --- | --- | --- |
| 1 | Confirmed | A renamed contract now excuses a changed method only when the contract is that method's own name. | `a_renamed_contract_cannot_excuse_an_unrelated_operation` |
| 2 | Confirmed | A broken `osprey.toml` now fails only the `.osp` and `.ospml` files under its directory. Everything else is scanned. | `broken_project_manifests_leave_ordinary_python_analysable`, `broken_nested_manifests_leave_sources_outside_them_analysable` |
| 3 | Confirmed | A marker other than `flavor=python` in an unclaimed `.py` file is now an ordinary comment. A conflicting marker in an `.osp` file is still an error. | `foreign_markers_leave_ordinary_python_analysable`, `conflicting_flavor_marker_returns_a_visible_error` |
| 4 | Confirmed | Code actions now read the text the current report analysed, from a lock-free snapshot published with that report. | `unsaved_lines_above_a_clone_shift_the_extract_edit_with_them` |
| 5 | Confirmed by reading the code | An edit in the seed window is no longer acknowledged until the cold pass has replayed it. | `a_seed_window_edit_is_acknowledged_only_by_the_pass_that_replays_it` |
| 6 | Not a code regression | No change. See below. | n/a |

**Two tests changed meaning.** `malformed_project_manifest_returns_a_visible_error` and `conflicting_flavor_marker_returns_a_visible_error` used Python fixtures, which pinned findings 2 and 3 as intended behaviour. They now use an Osprey fixture and keep every assertion, and the Python cases have new tests asserting that the scan succeeds.

**Finding 5's test works at session level.** It calls the session's acknowledgement methods directly. It does not drive a real LSP, because the seed window can't be held open deterministically from outside. The test was written with the fix, so it was never seen failing, and the notifications in `cache_seed.rs` and `backend_changes.rs` are not covered by it.

**Finding 4 leaves one gap.** A code action requested while the analysis of a newer edit is still running gets the previous report and its matching text, so its edits describe the previous version. v0.37.0 had the same exposure for every unsaved buffer, so this is not a regression.

**Finding 6 is a policy question, and nothing changed.** The 11 recorded `fail` outcomes are false positives the gate tolerates by design since #594, which fails a run only on regressions. They were recorded in `13fca3f9`. Making them red again means fixing those 11 pairs in the detector or reversing #594, and neither is a regression fix.

**Corpus comparison.** `scripts/compare-versions.sh v0.37.0 HEAD`, run on `cc9b3e98`, scored both builds at 96.6% (313/324), with 0 false negatives, 11 false positives and 43215 clusters. The 11 false positives therefore already existed at v0.37.0. The dispatcher fix changed no judged pair and no cluster count on this corpus, so `a_renamed_contract_cannot_excuse_an_unrelated_operation` is the only evidence for it.
