# Fused confidence — open work and branch readiness

**Scope.** This file carries two things and nothing else:

1. the open engine work that changes fused admission, measured cluster confidence, content gating, bucket
   routing or confidence-aware ranking;
2. the readiness ledger for the `worktree-fused-score-followups` branch — merged in from
   `DIFF_RELEASE_READINESS_REPORT.md` and `docs/worktree-fused-score-followups-pr-readiness.md`, both now
   deleted.

Candidate generation, cache mechanics, watcher state, CI maintenance and repository-wide metrics have
their own plans. A candidate-route problem belongs here only when two runs produce the same final
occurrence set but assign it different measured confidence.

The shipped contract is `[FUSION-STRATEGY-BOUNDED-MAX]`, `[FUSION-CLUSTER-SIGNALS]` and
`[FUSION-CONTENT-GATE]` in [`fusion.md`](../specs/fusion.md). The real-repository precision gate is planned
separately in [`corpus-assertion.md`](corpus-assertion.md).

## The one measure

Every reported cluster is a real duplicate, and every real duplicate is reported. Order open work by how
much it moves that number.

## The contract

`fused` must **carry information**: the three agent bands in `CLAUDE.md` (`>= 0.85` do not write the copy,
`0.6..0.85` read the canonical occurrence and bias to reuse, `< 0.6` author it) must all be reachable, and
must mean the same thing in every language. `fused_golden_bands.rs` cites this paragraph; do not weaken it
without moving that suite with it.

## Where fused stands against it

Established, with the assertion that holds it. These are not open work; they are the baseline the open work
sits on top of. Cited by `live-bubble-fused.unit.test.ts`, `live-bubble.unit.test.ts` and
`report-schema.unit.test.ts`.

| Property | Held by |
|---|---|
| Fusion is the strongest single axis, never the sum — at **admission**, not only at render | `deslop-core/tests/pair_admission_bounded_max.rs` (axes `0.44 / 0.42 / 0.0` must be `DroppedBelowFused`; the sum would admit at 0.86), `issue_343_sum_clamp_saturation.rs` |
| Rendered signals are measured between the occurrences the report shows, never averaged over discovery edges | `cluster::signals::measured_signals`, `[FUSION-CLUSTER-SIGNALS]` |
| Shape-saturating clusters are re-scored against measured content evidence | `buckets::content_gated_signals`, `[FUSION-CONTENT-GATE]` |
| The engine's `bucket` is the verdict, not a UI-local `fused` cutoff — an act-now cluster below 0.85 still reaches every surface | `live-bubble-fused.unit.test.ts`, `report-schema.unit.test.ts` |
| All three agent bands are reachable and mean the same thing in six languages | `fused_golden_bands.rs` — verbatim / maximal rename / shape-only, with band separation and rank order per language |
| No report renders a constant confidence; every component stays in `[0,1]`; only byte-proven duplication saturates | `fused_golden_invariants.rs`, swept over 21 corpora |
| One cosine definition, `f64` accumulation, byte-identical snippets render exactly `1.0` | `issue_372_identical_snippet_cosine.rs` |

---

# Part 1 — Branch readiness

**Verdict: two hard blockers — the duplication gate, and the four deliberately red
`type3_enclosing_method` cases. Everything else is either closed with measured evidence, or is a
pre-existing defect this branch improved rather than caused.**

The second blocker was previously recorded here as a tolerated red pin. It is not tolerable at the
gate: `make test` is fail-fast, CI runs it, and `type3_enclosing_method.rs` does not exist at
`f92300e` — this branch introduced four failing tests, so CI on this branch cannot go green while
they stand. See § "#408 residue" for what closing them costs; the decision between closing them and
tracking them differently is the release owner's, not an agent's.

Base `f92300e5e`, head `8751e8bfb`. The duplication figures below were measured on 2026-08-20 against this
tree with the binary this tree builds. Every other figure is carried forward from the runs the two merged
audit documents recorded on 2026-08-19/20; re-run them against the exact release candidate before approval.

## The static audit's four P0s

| # | Defect the static audit found | Status |
|---|---|---|
| P0-1 | `ReportSignals` gained `agreement` / `rename_consistency` / `literal_fraction`; two literals still built the old four-field struct, so those targets could not compile | **Fixed.** Both sites carry all seven fields (`diff_scope/tag.rs:105`, `tests/diff_render_tags.rs:88`); `cargo clippy --release --all-targets --workspace -- -D warnings` clean, no suppressions |
| P0-2 | The bounded exact embedding-pair path was deleted, leaving `TOP_K = 5` ANN recall — admissible pairs vanish when six closer neighbours crowd both endpoints out | **Fixed.** `EXACT_PAIR_LIMIT = 256`, `exact_embedding_pairs` and the deterministic exact/ANN merge are restored in `embedding/pairs.rs:25-97` |
| P0-3 | All-providers-failed reached a production `panic!` carrying `#[allow(clippy::panic)]`, instead of a terminal failure that preserves the last good report | **Fixed.** Panic and suppression gone; `run_embedding_refresh` returns a typed `FailedEmbeddingRefresh`, the embeddingless report is never committed, and the failure path publishes `phase = "failed"`, `done = 0` with provider/model/counts |
| P0-4 | Five checked-in accuracy contracts recorded as red | **Four green, one red.** See below |

### P0-4 in detail

| contract | status |
|---|---|
| `typescript_qualified_type_name_rename_is_token_invariant` (#410) | green |
| `python_issue_72_monkeypatch::monkeypatch_setenv_setup_pattern_is_not_duplicate_code` | green |
| `python_dict_assert_payload_proof::a_call_inside_a_consumed_payload_value_is_not_excused` | green |
| `python_literal_variation_calls::rest_endpoint_family_with_fstring_paths_is_suppressed` | green |
| `type3_enclosing_method.rs` (#408 residue) | **red — 1 of 5 languages** |

The three Python suppression contracts went green with the `verbatim_dominated` repair: one
token-identical family — equal normalised-subtree digest *and* equal collapsed-leaf keys — must now hold a
strict majority before it can certify a cluster as verbatim. Previously it certified non-verbatim members
as verbatim and forced `agreement` to 1.0.

The Type-3 residue is analysed in Part 2. It is **not a regression**: at `f92300e` *no* language reports
the enclosing method pair; at head C# does. This range took #408 from 0 of 5 to 1 of 5.

## The duplication gate — the one hard blocker

Measured on this tree with the binary this tree builds, 2026-08-20:

| | value |
|---|---|
| duplicated LOC | 14,851 |
| analysed LOC | 116,139 |
| duplication | **12.787%** |
| `.deslop.toml` ceiling | **9.9%** |
| `make dup-gate` | exits **3** — `make ci` fails on this step |

Closing 2.89 points means removing roughly 3,350 duplicated LOC. Where the duplication lives, measured
over the head report's 1,054 clusters:

| where | removable? |
|---|---|
| inline fixture literals in test files (`CSHARP_ALPHA`/`CSHARP_BETA` in `tests/boilerplate.rs`, the generated-DTO pairs in `tests/defaults.rs`) | **no** — they exist *because* they are duplicates. `.deslop.toml` excludes `**/tests/fixtures/**`, but a fixture written as a `const … &str` has no path to exclude |
| test scaffolding and test code | yes — the bulk of the mass |
| production `src/` | yes |

The ceiling is reachable without touching a fixture; it is not reachable *quickly*. The distribution is a
flat tail of several hundred clusters averaging about eleven redundant lines each, so closing the gap means
hoisting shared scaffolding across several hundred test files, each change carrying its own risk of
weakening an assertion.

The branch has been paying this down rather than moving the number: the largest DRY-able cluster in the
repository was the pair of near-identical GH #119 role-gate suites, whose contract now lives once in
`tests/common/role_gate.rs` — which also strengthened both suites, since the Dart and Python same-role
tests inherited the embedding-support assertion they previously lacked.

**No threshold was ever raised to hide a regression.** The 12.5 → 14.5 → 11.3 → 9.9 history tracked a shift
in what the engine counts, then real removal; like-for-like on one binary this branch *removed*
duplication relative to base.

## Ignored tests — eight down to three

No new `#[ignore]` was introduced. All six JavaScript/TypeScript `.skip(...)` calls are gone (0 remain).
Two Rust ignores were removed by making the tests genuinely pass: `python_issue_119_embedding_role_mismatch`
(needed a real fix — see below) and `pair_size_coherence` (needed nothing but running).

The three that remain carry the same `#[ignore]` attributes verbatim at `f92300e`, so they are unchanged
pre-existing defects, not regressions in this range:

| still ignored | measured with `--ignored` |
|---|---|
| `crates/deslop/tests/embedding_route_invariance.rs` (#356) | fails — the `ledger_d`/`ledger_e` pair published with embeddings off is absorbed into a wider `a,b,d,e` cluster with them on, so the exact published file set disappears |
| `crates/deslop-lsp/tests/lsp_embedding_determinism.rs` (#369) | fails — the `ts-mixed-band` refresh has no stable second cluster to reproduce |
| `crates/deslop/tests/issue_343_sum_clamp_saturation.rs` (#369) | fails — `mid_band_cluster_confidence_never_exceeds_its_strongest_axis`; two embedding-only false positives survive on MockOllama's length-residue cosine and the real clone is hidden |

## Defects found while closing the audit — all fixed

Each was invisible to the static audit and is pinned by a test.

- **`diff_render_tags` goldens predated the content-evidence line** the text renderer emits for every
  cluster (#344). Fixing P0-1 made the file compile, which made two of its three tests fail. The goldens
  now carry that line across 8 cluster blocks — strictly more bytes asserted, not fewer.
- **Old-report replay would have been demoted.** `ReportSignals.agreement` now defaults to
  `report::unmeasured_agreement()` (1.0, matching `ContentEvidence::unmeasured`, so a replay never demotes
  what the original report vouched for); `rename_consistency` and `literal_fraction` default to 0.0; and
  `EmbeddingProvenance.succeeded_subtrees` is reconstructed from the `attempted = succeeded + failed`
  invariant. The defaults are declared in the typeDiagram config
  (`scripts/typediagram-gen/type-config-{report,core}.mjs`), so the generated wire model carries them.
  Pinned by `cli::from_report::from_report_replays_legacy_report_predating_content_signals`, which replays
  a four-field, provenance-without-`succeeded_subtrees` report and asserts the bucket, every signal value,
  the reconstructed count and the preserved metrics. The existing fixture was left untouched.
- **A vanished provider announced success.** `admit_refresh_report` treated a report with *no*
  `embedding_provenance` as a success. A refresh runs under `EmbeddingMode::Auto`, where
  `run_embedding_pass` deliberately swallows a provider error — so the LSP announced `phase = "complete"`
  over an embeddings-off snapshot: the GH #370 false negative through a different door. Model selection
  probes the provider, so an endpoint already down is refused with an error the user sees; the uncovered
  case is a provider that answers that probe and is gone when the background refresh runs. Pinned by
  `vanished_provider_refresh_reports_failure_and_preserves_last_good_report` against the real binary,
  driven by `MockBehavior::VanishAfterProviderHandshake` — deterministic, because it ends on the handshake
  rather than on a clock.
- **The failure announcement was not revision-guarded** while the success announcement was, so a superseded
  refresh could land a stale terminal `failed` after a newer one announced `complete` — and clients hold
  one embedding-progress signal, not one per revision. Both terminal announcements now go through
  `AnalysisSession::embedding_refresh_is_current`.
- **A real `ollama_*` regression inside the range.** `make test-ollama` reported 6 passed, 2 failed;
  both pass at `f92300e`. The `ollama_*` tests do not use a live provider despite their name — they run
  through `run_deslop`, which spawns `MockOllama`. At `f92300e` the mock was the GH #366 vector, whose two
  constant lanes floored *every* pair near cosine 1.0, so the Type-4 pair passed for a reason unrelated to
  its content. GH #369 replaced it with an honest content statistic, and a Type-4 clone is by definition one
  no statistic over the text can score. The fixture's behaviour-equivalence is now declared to the mock
  through `MockOllama::spawn_semantic`, so the mock stands in for a model that has read both files while
  every pair it does not name keeps its honest shingle cosine. No threshold moved and no assertion changed.
  Independently confirmed against the real model: `nomic-embed-text` scores this pair at cosine **0.974**
  and the CLI publishes the cross-file `same_behavior` cluster.

## Validation routes

| route | status |
|---|---|
| `make lint` | clean — `cargo clippy --release --all-targets --workspace -- -D warnings`, no suppressions |
| `cargo fmt --all -- --check` | clean |
| ordinary workspace suite | green apart from the four deliberately red `type3_enclosing_method` cases |
| `make test-ollama` | 8/8 against a real local `nomic-embed-text`, after the regression above |
| `make dup-gate` | **fails** — see above |
| `make test-corpus` | not runnable here; needs corpus clones this environment lacks |
| hosted action path | the branch-built proof `scripts/test-action-diff-gate.mjs` passes 2/2, but it tests the gate's logic, not the download/install path. The `diff-gate` job in `action-selftest.yml` runs only when the newest published version is ≥ `0.33.0`, so the hosted route can be skipped precisely before the first release that introduces the compatible flags |

## Repository-policy items

- The two 501-line Rust test files are split: `common/multilang.rs` → 336 + `common/multilang_warm.rs` 185;
  `diff_scoped_reporting.rs` → 312 + `diff_scoped_ingest.rs` 62 + `common/diff_scope.rs` 159. All 18
  affected tests green.
- **Twenty other files still exceed 500 lines**, largest `deslop-mcp/tests/cli.rs` at 2,658 and
  `deslop-core/tests/live.rs` at 1,462. Pre-existing, not introduced by this branch, and not covered by any
  gate.
- **Too Many Cooks configuration is intentional.** `.codex/mcp.json` sits beside the tracked `.codex/skills/*`
  set and `.mcp.json` is its Claude-runtime mirror; the two are byte-identical by design and CLAUDE.md
  documents TMC as a supported workflow.

---

# Part 2 — Open engine work

## #410 — anchor mass demotes a bijection the engine certifies as total

The only open engine defect in this plan, and unblocked.

`rename_consistency = min(literal_preservation, coverage) * anchor_weight(anchors)`.
[`ts-qualified-type-rename`](../../crates/deslop/tests/fixtures/ts-qualified-type-rename) measures
`literal_preservation 1.0` and `coverage 1.0` — the engine's own terms certify the bijection as **total** —
and demotes anyway, purely on `anchor_weight(8) = 8/(8+4) = 0.6667` against `CONTENT_SUPPORT_FLOOR = 0.7`.
It misses by 0.033.

`typescript_qualified_type_name_rename_is_token_invariant` is **green**: the whole-function pair survives
instead of being deleted in favour of its byte-identical tail fragment, because content evidence is now
attached before cross-cluster subsumption elects a survivor (`[REPAIR-SUBSUME-CONTENT-FIRST]`). The mass
question is therefore open on its own terms, not on a red pin.

#410 was blocked by #409 because #409 changes its only input
(`anchors = preserved_literal_count(literals) + mapping.explained`). That edge is discharged: re-measured
after #409 landed, #410 is unchanged, as predicted — the fixture has no substituted literals, so no echo
fires and the anchor set is identical.

**The open question.** Whether `RENAME_EVIDENCE_HALF_MASS` is the wrong shape — a mass term that can never
reach a floor above `n/(n+4)` for small-but-total bijections — or whether a certified-total bijection should
bypass the mass discount entirely.

**Constraints on the fix.** Re-measure against the same precision set #409 was measured against:
`dart_issue_197`, the F# data-table corpus, `type2_rename_anchor_floor`, `fused_golden_bands`.
`CONTENT_SUPPORT_FLOOR` may **not** be lowered to close the 0.033 gap.

## #408 residue — an admission defect, not a gate defect, and it is measured

#408 was filed as a five-language Type-3 recall hole and tracked here as a subsumption problem. `csharp-type3`
was, and is, fixed. The other four are not this plan's defect: their whole-method pairs are never
*admitted*. No subsumption order can recover a pair that was never built.

Pinned red by [`type3_enclosing_method.rs`](../../crates/deslop/tests/type3_enclosing_method.rs) — `dart`,
`go`, `python`, `ts-type3-stmt`. `ts-type3-stmt` is the sharpest: one inserted statement takes the visible
clone count from one to zero.

Exact k-gram Jaccard between the two whole methods, measured off the normalised token streams:

| fixture | method nodes | exact Jaccard | admitted? |
|---|---|---|---|
| `dart-type3` | 56 / 49 | 0.8431 | no — under `FUSED_THRESHOLD` 0.85 |
| `go-type3` | 53 / 48 | 0.7755 | no |
| `python-type3` | 37 / 31 | 0.7429 | no |
| `csharp-type3` | 58 / 52 | 0.8519 | yes — renders via the LSH-only near-miss route at 0.92 |

C# clears the bar only because its `namespace`/`class` scaffolding dilutes the one-statement delta. The
MinHash estimate is not the cause: it reads 0.80 against an exact 0.84 on Dart, and the exact value is
still short.

The evidence the pipeline discards is structural. `pair.rs` documents `structural_sim` as "the
best-achievable subtree overlap", but the code writes a literal `0.0` for every cross-bucket pair — while
the unchanged statements inside these methods are Merkle-identical, which is exactly why fragment views
survive. Maximal shared-subtree coverage over the larger method: dart 0.87, go 0.86, python 0.82,
csharp 0.84, ts 0.81.

Closing it means measuring that overlap at admission **and** at render, plus a routing row for "high
structural overlap, moderate token overlap". Rendered `structural` is currently binary Merkle equality and
the anchor-free near-miss route requires `structural <= 0.01`, so making it non-binary without a matching
routing row would hide `csharp-type3` — the one language that works today. That is a signal-semantics
change needing its own assertions.

Content evidence must **not** move into pair admission to close this: it is a cluster measurement, and the
cluster-level facts it depends on (the canonical-member mean, the verbatim-member share) would change
meaning as well as cost. Measured on the 2026-08-18 repository run, 123,663 fingerprints produced 595,609
candidate pairs of which 11,868 survived into 3,616 closure components; content attachment cost ≈134 ms on
the components and would be asked of ~596,000 pairs at admission.

Tracked here only until whichever plan owns candidate admission takes it. Keep `type3_enclosing_method.rs`
red until it lands.

## Fused false positives — blocked on the corpus

None is closeable until the corpus can express *"these two things are not duplicates"* — section A of
[`corpus-assertion.md`](corpus-assertion.md), the same gap #401 reports.

Re-measured after the `verbatim_dominated` repair: the three suppression pins that were red are green.
Each asserts a *suppression*, so green means those false positives are no longer live.

- **Assertion idioms** (#71, #103, #285) — `python_issue_72_monkeypatch.rs` and the `python_dict_assert_*`
  suites are green; the idiom families are suppressed.
- **Data-table / object-literal families** (#283, #284) — recheck the language-agnostic data category
  shipped for #336 before treating these as open detector defects. `python_issue_133_constant_table` and
  `fsharp_issue_336_data_table_category` are green, so the category itself is intact.
- **Helper call sites** (#79) — `python_literal_variation_calls.rs` is green; the f-string endpoint family
  is suppressed.
- **#362 / `[RANK-STRUCTURAL-ONLY]`** — two unrelated const-declaration files must not become the
  repository's largest ranked finding.

## Corpus assertion gaps

[`corpus-assertion.md`](corpus-assertion.md) records that the corpus gate cannot yet back an accuracy
claim: five of nine repositories assert nothing; six of eight languages have no curated ground truth; there
is no `files_analysed` assertion, so a zero-file scan can pass; only Rust and TypeScript get curated Type-2
enforcement; the curated precision check uses raw `text.contains`, contrary to the AST-only rule and unsound
in both directions; seven open false positives lack a curated corpus surface; `must_find` is weaker than the
Type-2 checks; determinism is checked for only two of nine repositories; and a scheduled slice can be
mistaken for complete coverage. That plan owns the repair; this plan's #331/#339/#336/#347 close-outs all
wait on it.

## Close-outs — evidence recorded, a human closes

Deslop's agents never close issues (`CLAUDE.md`), so an item here is done when its evidence is **recorded
and named**.

| issue | what remains |
|---|---|
| #343 | nothing — `pair_admission_bounded_max` 3/3, `fused_golden_invariants` 2/2, `issue_343_sum_clamp_saturation` 3 passed + 1 pre-existing ignore |
| #355 | nothing — `dart_issue_197_single_file_structural_only` 1 passed, 0 ignored, re-verified after the subsumption change that briefly broke it |
| #339 | the curated-corpus F# token re-measure. Local suites green — `fsharp_issue_339_sibling_window_rename` (2), `fsharp_issue_339_token_fallback_rename` (1) |
| #336 | the curated F# run. `fsharp_issue_336_data_table_category` 4/4 green |
| #345 | audit the rest of the public fusion doc set. `fusion.md`'s `rename_consistency` definition and `pipeline.md`'s `[PIPELINE-CLUSTER-SUBSUME]` ladder are back in agreement with the code |
| #331 | re-verify the real-repository claim through the repaired corpus assertion; reopen if it does not survive |
| #347 | three consecutive green corpus runs, named when closing |

#339, #336, #331 and #347 all need `make test-corpus` clones this environment lacks.

---

# Checklist

## Done

Items marked **(code-verified)** were re-checked against this tree on 2026-08-20 by reading the code that
holds them. The rest are carried forward from the runs the merged audit documents recorded, and must be
re-run against the exact release candidate.

- [x] **(code-verified)** Every `ReportSignals` initializer carries all seven fields after the wire-model
      expansion (P0-1).
- [x] **(code-verified)** Bounded complete recall restored for admissible embedding pairs —
      `EXACT_PAIR_LIMIT`, `exact_embedding_pairs`, deterministic exact/ANN merge (P0-2).
- [x] **(code-verified)** The embeddingless-refresh `panic!` and its `#[allow(clippy::panic)]` replaced
      with typed terminal failure that preserves the last good report (P0-3).
- [x] The vanished-provider hole closed and both terminal announcements revision-guarded.
- [x] Old-report replay preserved through wire-model defaults, pinned by a new legacy fixture.
- [x] `diff_render_tags` goldens carry the content-evidence line — more bytes asserted, not fewer.
- [x] The three standing Python false-positive contracts are green after the `verbatim_dominated` repair.
- [x] The #410 TypeScript rename pin is green.
- [x] `make test-ollama` 8/8, including the `MockOllama` Type-4 regression found inside this range.
- [x] **(code-verified)** Ignored tests 8 → 3; JS/TS `.skip(...)` 6 → 0. No new ignore, no test or
      assertion removed or weakened.
- [x] **(code-verified)** The two 501-line Rust test files split — 336/185 and 312/62/159 lines.
- [x] `make lint` and `cargo fmt --all -- --check` clean.
- [x] **The one-calculation cleanse.** Every figure a surface renders is now computed once, in the
      engine, and carried on the wire: `rank` and `rank_band` ([SEVERITY-BAND]), `shape`,
      `meets_fused_gate`, `evidence_verdict`, `occurrence_count`, `language`, and
      `EmbeddingProgress.percent`. The client copies were deleted — the two rank-percentile engines,
      the severity cut points, the shape-score reduction, the verdict engine, the fused-threshold
      constant, the duplicate occurrence-count formula, and the progress percentage. The boundary
      that says what a client may still do is written down as
      [PRINCIPLES-ONE-CALCULATION](../specs/principles.md#principles-one-calculation). Held by
      `report_weight::rank_band_cut_points`, `report_weight::stamp_ranks_numbers_the_whole_report`,
      `report_weight::rank_band_never_brightens_down_the_report`,
      `render::signals::verdict_reads_each_family`,
      `render::signals::shape_score_is_the_stronger_axis`,
      `report_golden::committed_golden_satisfies_report_contract`, and the VS Code suites
      `severity.unit.test.ts`, `signal-evidence.unit.test.ts` and `report-schema.unit.test.ts`.
- [x] **(code-verified)** Stale checked-in release claims reconciled — including this merge, which replaces
      `DIFF_RELEASE_READINESS_REPORT.md` and `docs/worktree-fused-score-followups-pr-readiness.md`, and the
      restored § “Where fused stands against it” that three VS Code unit-test files cite by name.

## Remaining — blocking the PR

- [ ] **Duplication gate.** Bring the tree from **12.787%** to the **9.9%** ceiling — about 3,350 duplicated
      LOC, all of it reachable in test scaffolding and `src/` without touching a fixture literal. Do not
      raise the ceiling.
- [ ] **The four red `type3_enclosing_method` cases.** `make test` is fail-fast and CI runs it, so the
      branch cannot go green while `dart`, `go`, `python` and `ts-type3-stmt` fail. The file is new on
      this branch, so this is a blocker this range introduced. Closing it is the #408-residue work
      below; weakening or deleting the assertions is prohibited.

## Remaining — engine accuracy

- [ ] **#408 residue** — four languages' whole-method Type-3 pairs are never admitted. Needs shared-subtree
      overlap measured at admission *and* at render, plus a routing row for "high structural overlap,
      moderate token overlap"; a non-binary `structural` without that row would hide `csharp-type3`. Hand to
      the plan that owns candidate admission; keep `type3_enclosing_method.rs` red until it lands.
- [ ] **#410** — decide `RENAME_EVIDENCE_HALF_MASS`'s shape versus a certified-total bypass. Re-measure
      against `dart_issue_197`, the F# data-table corpus, `type2_rename_anchor_floor`, `fused_golden_bands`.
      Do not lower `CONTENT_SUPPORT_FLOOR`.
- [ ] **#356** — unignore `embedding_route_invariance`: enabling embeddings absorbs a published
      `ledger_d`/`ledger_e` pair into a wider cluster and the exact file set disappears.
- [ ] **#369 (LSP)** — unignore `lsp_embedding_determinism`: the `ts-mixed-band` refresh loses its second
      correlated signal, so there is no stable second cluster to reproduce.
- [ ] **#369 (clamp)** — unignore `issue_343_sum_clamp_saturation`: two embedding-only false positives
      survive on cosine alone and the real clone is hidden.

Fix these with honest fixtures and unchanged behavioural assertions. Do not weaken a threshold or an
assertion to turn one green.

## Remaining — corpus, blocked on `corpus-assertion.md` section A

- [ ] Close the corpus assertion gaps: assert every entry analyses files; curated positive *and* negative
      ground truth for every supported language; replace raw-text precision matching with AST identity and
      provenance; make a full strict run unmistakable from a scheduled subset.
- [ ] **#71 / #103 / #285** — assertion idioms.
- [ ] **#79** — helper call sites.
- [ ] **#283 / #284** — data-table / object-literal families.
- [ ] **#362** — `[RANK-STRUCTURAL-ONLY]`; unrelated const declarations as the largest ranked finding.
- [ ] **#339** — curated-corpus F# token re-measure.
- [ ] **#336** — curated F# run.
- [ ] **#331** — re-verify the real-repository claim through the repaired corpus assertion.
- [ ] **#347** — three consecutive green corpus runs.
- [ ] Run `make test-corpus` strict on the release candidate in an environment that has the clones, and
      record the result separately from the ordinary test target.

## Remaining — release evidence

- [ ] Validate the candidate packaged action through the same download/install/execute path users receive.
      The conditional `diff-gate` job reporting a skip is not evidence.
- [ ] **#345** — audit the remaining public fusion docs.

## Remaining — repository policy

- [ ] Twenty Rust files exceed the 500-line rule, largest `deslop-mcp/tests/cli.rs` (2,658) and
      `deslop-core/tests/live.rs` (1,462). Pre-existing and ungated; split them or gate the rule.

---

# Ledger

Kept only for fused repair IDs cited from tests or specifications.

| ID | What it fixed | Held by |
|---|---|---|
| `[REPAIR-RENAME-ANCHOR-MASS]` (#405) | Replaced a four-literal cliff with smoothly weighted Baker-corroborated anchor mass | `type2_rename_anchor_floor.rs`, `fused_golden_bands.rs`, `js_language_features.rs`, `js_ts_clone_buckets.rs`, `common/signals.rs`, `taxonomy.md` |
| `[REPAIR-SUBSUME-CONTENT-FIRST]` (#367, #408) | Measured content before destructive cross-cluster subsumption, and made the survivor election read it: a demoted view never deletes a credible one, a demoted encloser yields only to verbatim-proven nesting, and between credible views enclosure stands | `cross_cluster_collapse.rs`, `type3_enclosing_method.rs`, `cluster/subsume.rs`, `[PIPELINE-CLUSTER-SUBSUME]` in `pipeline.md` |
| `[REPAIR-RENAME-LITERAL-ECHO]` (#409) | Counted a literal renamed alongside its symbol as consistent rename evidence instead of disproof, so a more complete rename can never score below a less complete one | `rename_literal_monotonicity.rs`, `js_language_features.rs`, `content/rename.rs`, `[FUSION-CONTENT-GATE]` in `fusion.md` |
