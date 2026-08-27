# Release audit — what blocks the release

Scoped to **regressions since `f92300e` (v0.32.0)**: behaviour that is worse at HEAD than in the shipped release, plus anything that stops the gate running at all. Measured on the `accuracy-ordered-overlap-bound` branch at HEAD (`16e3d91`) with `target/release/deslop`; every figure below was measured at that commit.

Everything else that was on this page — the fusion-arithmetic departures, the parked engine defects, the gate and coverage work, the documentation drift — is not a regression and now lives in [`plans/fused-score-followups.md`](plans/fused-score-followups.md). It is not restated here.

**Verdict: not ready.** What stands between this branch and the release: **one red assertion — `render_noise_totals_observability` Interaction 2 (gh #478)**, awaiting a load-bearing threshold decision from `release-gate`, and the packaged Action unvalidated. The `#460` quarantine crash is **fixed (measured: self-scan exits 0)**; all four #434 fixes landed; **gh #467 landed** (`pi-audit`: two-member literal-variation families publish iff the differing argument is an authored interpolation; split defers pairs to render); the gate compile (§ 1) is fixed and verified. Everything else in the repo is green.

## 1. The gate does not compile — introduced on this branch (fixed)

`crates/deslop-core/Cargo.toml` declares two bench targets, `cluster_signals` and `shared_subtree_alignment`, without `required-features = ["benchmark"]`. Both sources import `cluster::benchmark` / `overlap::benchmark`, which are `#[cfg(feature = "benchmark")]`. The gate runs `--all-targets --features deslop-core/live,deslop-lsp/profiling`, and `deslop-lsp/profiling` pulls in `fxprof-processed-profile` and `pprof` only — it does not enable `deslop-core/benchmark`. So cargo builds the benches without the feature and dies.

Reproduced directly: `cargo check -p deslop-core --benches` fails `E0432`, "could not find `benchmark` in `cluster`" and "in `overlap`", with rustc naming both gated modules as configured out. `make test` and `make lint` both fail to compile at HEAD.

**Fix — applied by `release-gate` over TMC, verified here:** `required-features = ["benchmark"]` now sits on both `[[bench]]` sections. `scripts/benchmarks/cluster-signals.mjs` and `shared-subtree-alignment.mjs` already pass `--features benchmark` themselves, so nothing else moved. The exact gate command now compiles every target clean, and the full suite with the fix runs 1210 passed, 0 failed — this was the branch's only self-inflicted damage.

That count excludes the **20 rows** of `CURATED_SKIPS` in `crates/deslop/tests/skip_policy_contract.rs` — recount verified at `16e3d91` from the registry itself. Eight are accuracy tests red for real reasons under `-- --ignored`: `operator_drift_is_not_duplication` ×2 (#432), `lsh_only_nearmiss_recall` (#433), `lsp_embedding_determinism` / `issue_343_sum_clamp_saturation` (#369), and the `type2_recall` trio (#439, red on purpose per the curated-recall bargain). The other twelve are infrastructure: eleven `corpus_repos` entries blocked on #166 (gh #422) and `corpus_manifest_contract` (#426). The #434 pins are un-ignored and green; the retired `report_golden`/`incremental_multilang` ignores proved green before deletion. Read “1210 passed” next to that number, never on its own.

## 2. Four Python noise families v0.32.0 suppressed — #434, all four fixed at `16e3d91`

Re-measured at HEAD with the release CLI, at each pin's own `--min-nodes`, on the checked-in fixture bytes. Before the fix campaign the four fixtures published their scaffolding on top of the staged 16-line control clone; after it, every fixture reports **the control clone only** and its family hidden:

| fixture | pin `--min-nodes` | pre-fix | post-fix (measured) | hidden |
|---|---|---|---|---|
| `python-issue-71` | 4 | 70.18%, `duplicated_loc` 40 | **28.07%, 16/57 — control only** | 1 |
| `python-issue-70` | 8 | 57.14%, `duplicated_loc` 28 | **32.65%, 16/49 — control only** | 1 |
| `python-issue-72` | 4 | 67.39%, `duplicated_loc` 31 | **34.78%, 16/46 — control only** | 1 |
| `python-issue-107` | 4 | 45.83%, `duplicated_loc` 22 | **33.33%, 16/48 — control only** | 4 |

Each fixture also stages a real 16-line clone as a control, so “control only” is an assertion the detector still sees — not blindness. The #72 trio publishes via the `structural_only` route no more; the pins for all four are un-ignored and green.

### All four are regressions, not two

The previous revision of this page recorded #70 and #71 as new pins on previously-untested behaviour. That is wrong, and the correction matters because it doubles the blocking surface. Measured against `f92300e`:

- **The fixture bytes are unchanged.** `test_write_file_calls.py` (#70), `test_endpoints.py` (#71) and `test_fly_host.py` (#72) all hash identically to their v0.32.0 contents. What this branch added to those directories is the `control_clone_a.py` / `control_clone_b.py` pair and the test — not the noise family. The input the detector sees is the same input v0.32.0 saw.
- **Both root causes are code that did not exist in v0.32.0.** `cluster_filters/verbatim_subgroup.rs` is absent at `f92300e`. So is `every_covered_statement_has_call` in `cluster_filters/calls.rs` — the file existed, that precondition did not.

So the same bytes now take a code path that v0.32.0 had no way to take, in both halves of the defect. For #72 and #107 this is confirmed from the other direction: their tests were green at `f92300e`, unskipped, asserting zero clusters. For #70 and #71 it is an inference from unchanged input plus new blocking code — running the v0.32.0 binary would need a checkout, which the repository rules forbid, so nobody has measured it directly. Treat them as regressions and let a measurement demote them, not the other way round.

### Two defects, not one — corrected mid-campaign

- **#70 / #71 — the filter never fires** (`hidden == 0`). `[CLONE-NOISE-LITERAL-VARIATION-CALLS]` should suppress these. `every_covered_statement_has_call` returned false first, because each body ends in a call-free `assert`. The arbitration is now decided and written — [CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT] in `docs/specs/noise.md` — and the constraint held: `rename_needs_an_anchor` moved with the fix, not around it.
- **#72 — the filter is inert here, not leaking.** Measured during the fix campaign: every noise filter reads `fired=0`, `hidden=0` on this fixture — nothing suppressed anything, and the trio publishes because **no filter engaged**. The earlier framing on this page ("suppression counted, 3 hidden, cores escape") attributed hides to this family that were not its; the hatch is not what publishes the trio.
- **#107 — the verbatim-subgroup escape, the real one.** Suppression counted (4 hidden) and same-file cores published; the pairs are not byte-identical, so [CLONE-NOISE-VERBATIM-SUBGROUP-EXACT-BYTES] closes it.

Stale alongside: all four #434 `#[ignore]` reasons still read "spec arbitration pending" — the arbitration is decided; the reasons must name the decided spec when the pins are restated (`release-gate`'s, with the pin surgery).

The `duplicated_loc` rule is settled and ticked below: metrics fold visible clusters only — black-box verified again by the #107 min-nodes sweep (hidden 5→4→3 with `duplicated_loc` steady at 16).

This branch neither caused nor can close these — `b235c1a5` (#424) and `c3ce7882` brought them to `main`. It is the release vehicle, so it ships them. At this HEAD the branch touches both files mechanically (`Rc`→`Arc`, sharding) with the decision logic unchanged.

Already fixed and green: the #69 hidden-group summary wording, un-skipped, `CURATED_SKIPS` row deleted.

## New findings from the fix campaign — all open

Measured by `release-gate`'s workers during the #434 fixes; recorded here unabsorbed, none ticked.

- **gh #462 — closed at HEAD: the sibling-cell route was a false negative, not a price.** The original contradiction (two pins over byte-identical `ledger_rows.py` demanding opposite outcomes) was **dissolved by restructure**: the solo corpus nests under `collection-cells/cells/`, the hide-side moved to `idiom-price/` — no bytes are pinned twice. The A/B pin (`adding_a_differing_sibling_never_deletes_a_visible_copy`) proved the sibling-cell suppression **deleted a visible copy when a stranger cell arrived** — a same-single-literal family can never span files, so the [CROSS-FILE] price is unpayable there and the suppression was a pure false negative. Decided architecture, landed: **sibling-cell route publishes the byte-identical pair** ([CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE-SAME-LITERAL]); **spannable routes keep the price** — `verbatim_subgroup_idiom_price.rs` pays it. **Measured here: 6/6 `verbatim_subgroup` pins green** (an earlier intermediate working tree measured `duplication_percent 0.0` over the solo corpus — the narrowing had not landed then). pi-audit's initial “flip the pin to hidden” ruling is **retracted, in writing**: a pin is never ruled from bytes alone.
- **gh #464 — sub-gate clusters billing the metric: mooted by the #72 fix, kept open for the class.** Pre-fix, the #72 trio carried `meets_fused_gate=false` (fused 0.57 against the 0.85 gate) yet contributed 15 of 31 `duplicated_loc` and drove 67.39%. Post-fix (measured at HEAD): the trio is hidden, `16/46 = 34.78%`, control clone only — nothing sub-gate bills anything on this fixture. The class question — whether any **visible** cluster below the fused gate should bill `duplicated_loc` — stays with #464.
- **gh #466 — rank severity inverted at population one.** `rank_band(1,1) = faint` (pinned, `report_weight.rs:205`): with exactly one cluster left, a byte-identical `fused=1.0` clone renders *faint*. Caveat measured: `[LSP-SEVERITY-PERCENTILE]` is Planned (#177) and severity floors default to 0, so nothing is suppressed by severity today — the damage is glyph density on the VSIX surface only; HTML and text reports never render `rank_band`.
- **gh #460 — quarantine abort fixed; the FP-rate claim refuted.** Two things, both measured. First — **fixed**: `deslop .` over this repo **exited 101** when the #460 quarantine panic fired mid-scan; at HEAD it **exits 0** (measured here, self-scan over the full repo). Second: the earlier “systemic false positive” reading does not survive hand-judgement — **26 clusters judged: 17 real, 3 borderline, 6 FP**, and support does **not** separate them (real 0.00–0.68, FP 0.00–0.31, overlapping); 504 of 1316 visible clusters (38%) sit below the 0.7 support floor and embedding support is vacuous as evidence because embeddings were off by config in that run. Acting on the old reading would delete 504 act-now clusters including rank #1. What remains true: the 0.7-floor population deserves eyes, not deletion.
- **False-negative candidate on #71.** The suppressed family duplicates 24 of 30 lines with no pre-extracted helper, and was `nearly_identical`, weight 108, ranked #1 — it may be a real duplicate the fix now hides. Recorded on gh #434 explicitly; needs fixture adjudication, not silent absorption.
- **VSIX consumer-path blocker — gh #468.** A VSIX-file install unpacks as `nimblesite.deslop-live-VERSION` with **no platform suffix**, so the documented absolute MCP path does not resolve — and a green contract test enforces the wrong path. The artifact itself validated: 13.31 MB, all three binaries execute from inside it, MCP initialize + 13 tools OK, cold CLI scan real, no version drift (every surface reads `0.0.0-dev`).
- **VSIX surfaces unverified on this host.** Cross-platform VSIXes (linux x64/arm64, darwin-x64, win32) cannot build here, and a Marketplace install needs credentials — both recorded unverified, not passed.
- **gh #471 — a test artifact ships inside the `.vsix`.**
- **`deployment.md` contradicts itself — see gh #470.** `[EXTERNAL-MCP-CONSUMER]` allows PATH only via brew/scoop; `[DOCS-INSTALLER-FAILCLOSED]` blesses a curl install to `~/.local/bin`. Same manifest-vs-spec divergence family as `shipwright.json`, which sanctions what `AGENTS.md` forbids.
- **gh #412 (back) — fixed at HEAD, then re-caught by its own contract.** `Makefile` now uses bare names with a zero-selection guard (`Makefile:414–420`), `corpus.yml` defers to the Makefile lists, and `crates/deslop/tests/corpus_selection_contract.rs` holds every name to a test that exists. That contract itself was red at `8c85cda`: its Makefile parser read one physical line, so `CORPUS_TESTS_FULL`'s backslash continuation became a phantom test name — the `full` dispatch would have selected nothing. **Fixed (`pi-audit`, under lock): the parser joins make continuations; 3/3 contract tests green, no assertion touched.**
- **`make vsix-package` scrubs the host.** It runs `_delete-path-binaries` and can uninstall every deslop from PATH — validate from the VSIX bundle by absolute path.

## Checked and cleared — not blockers

Recorded so neither gets re-raised against this release.

- **#458 — rendered cluster signals average over pairs the detector never admitted.** Confirmed at HEAD: two byte-identical TypeScript files render `identical` at `structural / token_jaccard / fused = 1.0 / 1.0 / 1.0` when scanned alone, and `nearly_identical` at `0.9982 / 0.8313 / 0.7953` when the same pair sits inside a six-member cluster. Byte proof loses its bucket to averaging. Real, and critical — but `cluster/signals.rs` is present at `f92300e` and the mean predates it, so it is not a regression. Tracked in the plan.
- **#459 — "adding one duplicated file deletes existing findings" does not reproduce.** Filed against `ts-mixed-band` on the claim that adding a byte-identical `ledger_a_copy.ts` drops the report from 2 clusters / 5 files / 100% to 1 cluster / 2 files / 11.11%. Re-measured cold, warm, and across `--min-nodes` 8/12/20/30/40: adding the copy *increases* coverage every time — 5 files → 6, clusters 2 → 2 or 3, duplication stays 100%, `duplicated_loc` rises 15 → 18, and the cold and warm-cache reports agree exactly. The original figures came from a contaminated scratch directory. The issue is wrong and needs a correction comment.

## 3. Packaged Action — download / install / execute, measured

The exact runner path, driven locally with this branch's resolver scripts against the **latest published release, `v0.32.0`** (the candidate `v0.33.0` asset does not exist until `release.yml` publishes it — see the post-tag box):

```sh
# 1. Resolve — exactly what the action's first step runs
GITHUB_OUTPUT=out node scripts/actions/action-resolve-artifact.mjs macOS ARM64 "" v0.32.0
#    -> url=https://github.com/Nimblesite/Deslop/releases/download/v0.32.0/deslop-0.32.0-macos-arm64.tar.gz
# 2. Download (curl --fail --location --retry 3)      -> 12,614,314 bytes + .sha256
# 3. Verify   node scripts/actions/action-verify-checksum.mjs <archive> <archive>.sha256
#    -> "Verified sha256 95d9f0a35a4330097a009997baacd65474f8b0c789ff0e84595c5376a7721445"
# 4. Install  tar -xf -> deslop-0.32.0-macos-arm64/{deslop,deslop-mcp,deslop-lsp} ; mv <stage> bin
#    -> bare `deslop` on PATH answers `deslop 0.32.0` (layout assertion passes)
# 5. Execute  deslop . --min-nodes 30 --no-incremental   (real repo: git archive HEAD of this tree)
#    -> exit 0, files 1316, clusters 1045, dup% 8.55, duplicated_loc 14559
```

**Version drift, measured.** The Action itself has none: a `v0.32.0` ref pin installs 0.32.0, the latest release; and the VSIX sweep found no 0.27 anywhere in the shipped surfaces — all eleven read `0.0.0-dev`. The only version pin in this repo's workflows is `action-selftest.yml:108` → `@v0.30.0`, a deliberate self-test fixture. What remains is consumer-side: an explicit `version: 0.27.0` input (the `osprey` repo's CI pin, per `Osprey2`) resolves 0.27.0 by design — an explicit pin is the user's contract, worth a nudge at release time, not an Action change. Candidate probe: the would-be `v0.33.0` asset URL answers **HTTP 404** today, as expected pre-release.

## To finish this release

- [x] **Gate the two benches** — `required-features = ["benchmark"]` on both `[[bench]]` sections (§1). Fixed by `release-gate` over TMC; verified here against the exact gate command.
- [x] **#434 — `duplicated_loc` must not count a suppressed family.** No separate fix exists: metrics already fold only visible clusters, pinned green by `metric_excludes_hidden_clusters` (re-run here, passing). This item collapses into the two fix items below — what § 2 still shows counting is the *published* trio, which goes hidden only when #72 lands.
- [x] **#434 — decide the `[CLONE-NOISE-VERBATIM-SUBGROUP]` arbitration** and write it into `docs/specs/noise.md`. Decided: the hatch is **cross-file only** ([CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE]) and byte-identity means **exact source bytes** ([CLONE-NOISE-VERBATIM-SUBGROUP-EXACT-BYTES]). #72/#107 unblocked.
- [x] **#434 — fix #70 / #71 — landed, verified here.** Rule decided and shipped: every covered statement must carry a call, **except one lone call-free statement**, admitted only as a Python `assert_statement` whose subject identifiers are non-empty and all bound by the covered calls' assignment targets. Spec: [CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT] in `docs/specs/noise.md`. **Measured at HEAD:** #70 `16/49 = 32.65%`, hidden 1; #71 `16/57 = 28.07%`, hidden 1 — control clone only in both.
- [x] **#434 — fix #72 / #107 — landed, verified here.** [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE] and [-EXACT-BYTES] implemented per `docs/specs/noise.md`; the sibling-cell route publishes per [CROSS-FILE-SAME-LITERAL] (see the #462 finding). **Measured at HEAD:** #72 `16/46 = 34.78%`, hidden 1, the `structural_only` trio gone; #107 `16/48 = 33.33%`, hidden 4 — control clone only in both. All four #434 pins un-ignored and green.
- [x] **Restate all four #434 pins and delete their `CURATED_SKIPS` rows — landed (`release-gate`).** Registry read from the file: `CURATED_SKIPS` **20 rows**, `SKIPS_PER_ISSUE` **6 entries** — `(369,2) (422,11) (426,1) (432,2) (433,1) (439,3)`; no live `#[ignore]` cites 434. The retired `report_golden`/`incremental_multilang` ignores proved green before deletion.
- [x] **Re-measure all four #434 fixtures** with the release CLI at each pin's own `--min-nodes` and rewrite the § 2 table with post-fix dup%, `duplicated_loc` and hidden — **owner `pi-audit`, done** (figures in § 2, measured at HEAD after the fixes landed; no figure carried over).
- [x] **Recount the § 1 skip sentence after the pins un-ignore — owner `pi-audit`, done from the registry.** 20 rows (was 28): eight red-for-real (432 ×2, 433 ×1, 369 ×2, 439 ×3), twelve infra (#422 ×11 on #166, #426 ×1); #434 absent. § 1 prose now carries these numbers.
- [x] **Correct #459 on GitHub** — **owner `pi-audit`, done.** Sweep re-measured independently (base vs +copy × cold/warm × min-nodes 8/12/20/30/40 — coverage rises every time, cold=warm) and posted: [#459 (comment)](https://github.com/Nimblesite/Deslop/issues/459#issuecomment-5438230870). Issue left open for the author.
- [ ] **Re-bless the stale goldens once, last** — **owner `release-gate`**; an investigation is classifying each golden stale vs engine-defect vs restatement, and the findings land with `pi-audit` to write up. Blocked on #432/#433.
- [ ] **Strict `make test-corpus` — a signable either/or, corrected** — **owner `release-gate`.** The gh #412 selector bug is **fixed at HEAD** (bare names, zero-selection guard, name contract — verified by `pi-audit`; see the findings list), so the gate can run real tests again. What remains: #428 leaves an **un-baselined false negative on the tokio corpus** open, and #166 (runner memory) + #426 (manifest contract) are **infrastructure**, shippable-around. Either run the strict corpus on the candidate, or ship with this exact release-note sentence: *"This release ships without a corpus run: gh #428 leaves an un-baselined false negative on the tokio corpus open; gh #166 and gh #426 are infrastructure defects this release ships around."* (An earlier draft claiming "every other gate ran green" was false twice and must not ship.)
- [x] **Fix gh #467 without regressing the four suppression pins — landed (`pi-audit`, under TMC lock).** The agent's family-size floor (≥3) broke all four suppression pins — every one stages a **two-member family**; the #467 pair is two-member too, so size can never be the rule (proof: gh #467 comment 5439912364). Landed rule: **a two-member literal-variation family publishes iff its differing string argument is an authored interpolation** (f-string route, template substitution); plain-literal pairs stay suppressed. **Measured: deslop suite 462/0 — the #467 pin and all four suppression pins green.** The change moves plain-literal pair convictions from the render stage to the split stage — same final state — which exposes a pre-existing stage-distribution defect in `render_noise_totals_observability` (gh #478, next box).
- [ ] **Re-derive `render_noise_totals_observability` Interaction 2 for the corrected engine — gh #478, owner `release-gate` (threshold decision).** **Interaction 1 is fixed** (`pi-audit`): a `NoiseStage` (`Split`/`Render`) now threads `is_noise_pattern`, and the split stage defers two-member literal-variation families to render — measured, split `language_specific fired=5` vs cumulative `fired=6`; the delta is back. **Interaction 2 remains, pre-existing**: with [EXACT-BYTES], any byte-identical statement pair (the fixture's shared `import` lines) is split-eligible at any min_nodes, so “split runs no filter at mn=15” cannot be staged by fixture or min_nodes (five iterations proved it). Remaining option: a `node_count` floor on `splittable_families` — load-bearing threshold, must exclude ~3-node imports yet stay below the ~7-node collection-cell pair the [#462] publish path routes through the split. Dossiers: gh #478 comments 5440425362, 5441254004.
- [ ] **Run the full gate on the candidate — owner `release-gate`.** Current measured state: **462/0 (deslop suite), 197/0, 180/1 — `render_noise_totals_observability` is the only red in the repo.**
- [x] **Validate the candidate packaged Action** — **owner `pi-audit`, done — see § 3.** The full download/install/execute path measured against the latest published release (`v0.32.0`): resolve → curl download → checksum verify (`95d9f0a3…`) → versioned-layout extract → bare `deslop` on PATH → real-repo scan exit 0 (1316 files, 1045 clusters, 8.55%). Version drift measured: none in the Action; consumer-side 0.27.0 pins drift by choice. A skipped conditional `diff-gate` job was not counted as evidence.
- [ ] **Re-run the § 3 five-step path against the candidate asset** (`v0.33.0`) once `release.yml` publishes it — **owner `pi-audit`, post-tag.** The asset 404s until then; this release must not ship on a validation that predates its own binary.

## Retired findings — citation index

Eight source files cite this table by title; it stays.

| finding | pinned by |
|---|---|
| streamed LSH construction | `lsh/banding.rs` |
| admission parity | `pair/gate_parity_tests.rs` |
| first-seen pair deduplication drops stronger evidence | `pair/candidates/builder.rs`, `tests/pair_evidence_merge.rs` |
| parallel rescue | `overlap/rescue.rs::shard_equivalence_tests` |
| mixed-size overlap fallback | `overlap/tests.rs` |
| segmented-store remove/upsert logic has no changed test | `pipeline/session/store.rs` |
| Large parallel paths lack black-box parity coverage | `pipeline/corpus/tests.rs` |
| Removed signature-construction performance assertions | `pipeline/signatures/tests/canary.rs` |
