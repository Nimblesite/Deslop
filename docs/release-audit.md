# Release audit — what blocks the release

Scoped to **regressions since `f92300e` (v0.32.0)**: behaviour that is worse at HEAD than in the shipped release, plus anything that stops the gate running at all. Measured on the `accuracy-ordered-overlap-bound` branch at HEAD (`5a0999e`) with `target/release/deslop`.

Everything else that was on this page — the fusion-arithmetic departures, the parked engine defects, the gate and coverage work, the documentation drift — is not a regression and now lives in [`plans/fused-score-followups.md`](plans/fused-score-followups.md). It is not restated here.

**Verdict: not ready.** What stands between this branch and the release: two #434 fixes half-landed (`release-gate`, in progress) and the packaged Action unvalidated. The gate compile (§ 1) is fixed and verified.

## 1. The gate does not compile — introduced on this branch (fixed)

`crates/deslop-core/Cargo.toml` declares two bench targets, `cluster_signals` and `shared_subtree_alignment`, without `required-features = ["benchmark"]`. Both sources import `cluster::benchmark` / `overlap::benchmark`, which are `#[cfg(feature = "benchmark")]`. The gate runs `--all-targets --features deslop-core/live,deslop-lsp/profiling`, and `deslop-lsp/profiling` pulls in `fxprof-processed-profile` and `pprof` only — it does not enable `deslop-core/benchmark`. So cargo builds the benches without the feature and dies.

Reproduced directly: `cargo check -p deslop-core --benches` fails `E0432`, "could not find `benchmark` in `cluster`" and "in `overlap`", with rustc naming both gated modules as configured out. `make test` and `make lint` both fail to compile at HEAD.

**Fix — applied by `release-gate` over TMC, verified here:** `required-features = ["benchmark"]` now sits on both `[[bench]]` sections. `scripts/benchmarks/cluster-signals.mjs` and `shared-subtree-alignment.mjs` already pass `--features benchmark` themselves, so nothing else moved. The exact gate command now compiles every target clean, and the full suite with the fix runs 1210 passed, 0 failed — this was the branch's only self-inflicted damage.

That count excludes the 28 rows of `CURATED_SKIPS` in `crates/deslop/tests/skip_policy_contract.rs`. Twelve of them are accuracy tests that are red for real reasons under `-- --ignored`: `operator_drift_is_not_duplication` ×2 and `report_golden` (#432), `incremental_multilang_golden` ×3 and `lsh_only_nearmiss_recall` (#433), the four #434 pins below, and `lsp_embedding_determinism` / `issue_343_sum_clamp_saturation` (#369). Read "1210 passed" next to that number, never on its own.

## 2. Four Python noise families publish that v0.32.0 suppressed — #434

Re-measured at HEAD with the release CLI, at each pin's own `--min-nodes`, on the checked-in fixture bytes:

| fixture | what publishes | dup% | `duplicated_loc` | hidden |
|---|---|---|---|---|
| `python-issue-71` | one `nearly_identical` family ×4, ranked **#1** above the real clone staged in the same run (weight 108 vs 44) | 70.18 | 40 | 0 |
| `python-issue-70` | one `structural_only` family ×4 | 57.14 | 28 | 0 |
| `python-issue-72` | one `structural_only` trio ×3, all intra-file in `test_fly_host.py`, `hidden=false` — its 15 lines are counted in `duplicated_loc` (31 of 46) | 67.39 | 31 | 3 |
| `python-issue-107` | three `identical` clusters, one same-file pair per file | 45.83 | 22 | 4 |

Each fixture also stages a real 16-line clone as a control. Every row counts the noise family on top of it, so `--fail-over` trips on scaffolding.

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

- **Spec-vs-test conflict (stop item).** `verbatim_subgroup_survives_noise.rs::two_identical_collection_cells` asserts an intra-file byte-identical pair **must publish** — the negation of the decided [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE]. Red at HEAD. `release-gate` will not touch a red test asserting a decided spec without the boss: the test must move with the spec, or the spec be reconsidered — sign-off needed.
- **Defect A — sub-gate clusters still bill the metric.** The #72 trio carries `meets_fused_gate=false` (fused 0.57 against the 0.85 gate) yet contributes 15 of 31 `duplicated_loc` and drives 67.39%. The tool says it is not confident, then bills the metric anyway.
- **Defect B — rank severity inverted at population one.** `rank_band(1,1) = faint` (pinned, `report_weight.rs:205`): once suppression leaves exactly one cluster, a byte-identical `fused=1.0` clone renders *faint*.
- **False-negative candidate on #71.** The suppressed family duplicates 24 of 30 lines with no pre-extracted helper, and was `nearly_identical`, weight 108, ranked #1 — it may be a real duplicate the fix now hides. Recorded on gh #434 explicitly; needs fixture adjudication, not silent absorption.
- **VSIX consumer-path blocker.** A VSIX-file install unpacks as `nimblesite.deslop-live-VERSION` with **no platform suffix**, so the documented absolute MCP path does not resolve — and a green contract test enforces the wrong path. The artifact itself validated: 13.31 MB, all three binaries execute from inside it, MCP initialize + 13 tools OK, cold CLI scan real, no version drift (every surface reads `0.0.0-dev`).
- **VSIX surfaces unverified on this host.** Cross-platform VSIXes (linux x64/arm64, darwin-x64, win32) cannot build here, and a Marketplace install needs credentials — both recorded unverified, not passed.
- **`deployment.md` contradicts itself.** `[EXTERNAL-MCP-CONSUMER]` allows PATH only via brew/scoop; `[DOCS-INSTALLER-FAILCLOSED]` blesses a curl install to `~/.local/bin`. That drift is what put a 0.27 on this machine's PATH.
- **`make vsix-package` scrubs the host.** It runs `_delete-path-binaries`: brew deslop/lsp/mcp 0.32 and `~/.local/bin/deslop` 0.27 are gone — nothing deslop is on PATH here. Validators must use the VSIX bundle by absolute path.

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
- [ ] **#434 — fix #70 / #71** — **owner `release-gate`, in progress.** Rule decided: every covered statement must carry a call, **except one lone call-free statement**, admitted only as a Python `assert_statement` whose subject identifiers are non-empty and all bound by the covered calls' assignment targets. Two or more call-free statements still block the filter — authored data handling is the extractable logic `rename_needs_an_anchor` protects, so the pin holds as is. Spec: [CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT] in `docs/specs/noise.md`. **Verifier, fresh:** #70 `16/49 = 32.65%`, hidden 1; #71 `16/57 = 28.07%`, hidden 1 — control clone only. Unticked until `release-gate` ticks; the #71 false-negative candidate (above) must be adjudicated first.
- [ ] **#434 — fix #72 / #107** — **owner `release-gate`, in progress**, implementing [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE] and [-EXACT-BYTES] exactly as `docs/specs/noise.md` states. **Verifier, fresh:** #107 `16/48 = 33.33%`, hidden 4 — green. **#72 unfixed and worse:** `31/46 = 67.39%`, hidden dropped 3 → 0 (filter inert — see the § 2 correction). Unticked.
- [ ] **Restate all four #434 pins and delete their `CURATED_SKIPS` rows** — **owner `release-gate`, blocked on the two fixes.** Surgery: 28 → 24 rows, `SKIPS_PER_ISSUE` loses `(434, 4)` so 7 → 6 entries.
- [ ] **Re-measure all four #434 fixtures** with the release CLI at each pin's own `--min-nodes` and rewrite the § 2 table with post-fix dup%, `duplicated_loc` and hidden — **owner `pi-audit`, after the fixes land.** No figure carries over unmeasured.
- [ ] **Recount the § 1 skip sentence after the pins un-ignore** — 24 rows, #434 out of the red-under-ignored list, `skip_policy_contract.rs` prose to "seven gh #432–#433" — **owner `pi-audit`, counted from the registry, nothing guessed.**
- [x] **Correct #459 on GitHub** — **owner `pi-audit`, done.** Sweep re-measured independently (base vs +copy × cold/warm × min-nodes 8/12/20/30/40 — coverage rises every time, cold=warm) and posted: [#459 (comment)](https://github.com/Nimblesite/Deslop/issues/459#issuecomment-5438230870). Issue left open for the author.
- [ ] **Re-bless the stale goldens once, last** — **owner `release-gate`**; an investigation is classifying each golden stale vs engine-defect vs restatement, and the findings land with `pi-audit` to write up. Blocked on #432/#433.
- [ ] **Strict `make test-corpus` — a signable either/or** — **owner `release-gate`.** `flutter/memory` and `fsharp/memory` are `corpus/known-failures.json` entries under #166, `corpus/flutter.json` needs `max_peak_rss_mb: 9000` against a 7 GB standard runner, and #426 keeps `corpus_manifest_contract` red. Either land those, or ship with this exact release-note sentence: *"This release ships without a strict corpus run: the Flutter and F# corpus checks remain known-failures under gh #166 (runner memory), and the corpus manifest scope contract (gh #426) is red; every other gate ran green on the candidate."*
- [ ] **Run the full gate on the candidate** — **after `release-gate`'s #72/#107 lands** (two agents mid-edit in `crates/**`; not now, confirmed).
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
