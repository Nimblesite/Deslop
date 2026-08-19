# Release Readiness Audit

## Compared endpoints

- Base: `f92300e5e1004ef6c53a94174a0d7e842232ec80`
- Head: `6b8cb396d034876f1ba0cae02fb545d85f4d872a`
- Review method: one static, aggregate endpoint comparison only. No individual commits were inspected.
- Validation boundary: no tests, builds, linters, or coverage jobs were run for this audit. Runtime status is therefore not claimed.

## Verdict: DO NOT RELEASE

The head contains three independently release-blocking defects visible directly in the endpoint diff:

1. Test targets construct an expanded generated wire-model struct without its new required fields, so the affected Rust test targets cannot compile.
2. The bounded exact embedding-pair path was deleted, making admissible pairs disappear whenever more than five closer neighbours crowd both endpoints out of the ANN result.
3. An ordinary all-embedding-provider-failures condition now reaches an intentional production `panic!` instead of a terminal failure state that preserves the last good report.

The head also explicitly documents unresolved red accuracy contracts, retains five ignored regression tests, weakens the duplication gate, and does not provide complete release evidence for the corpus, Ollama, or hosted-action paths.

## Scope summary

The aggregate endpoint comparison contains 353 changed paths:

| Change | Count |
|---|---:|
| Added | 141 |
| Modified | 210 |
| Deleted | 2 |

Neither deleted path is a test file. Static syntax-tree counts show fewer explicit skips and more assertion sites overall:

| Signal | Base | Head | Change |
|---|---:|---:|---:|
| Rust `#[ignore]` attributes | 8 | 5 | -3 |
| JavaScript/TypeScript `.skip(...)` calls | 6 | 0 | -6 |
| Rust assertion sites | 1,717 | 2,304 | +587 |
| JavaScript/TypeScript assertion sites | 1,390 | 1,665 | +275 |

These counts rule out a broad removal of tests, but they do not make the release safe: the blockers below are semantic and release-gate failures.

## Release blockers

### P0-1 — Expanded `ReportSignals` breaks affected test targets

`docs/models/live-ipc.td` adds three required fields to `ReportSignals`:

- `agreement`
- `rename_consistency`
- `literal_fraction`

Two head literals still initialize only the previous four fields:

- `crates/deslop-core/tests/diff_render_tags.rs:83`
- `crates/deslop-core/src/diff_scope/tag.rs:100`

Rust struct literals must initialize every required field. These test targets are therefore statically inconsistent with the generated model and cannot compile once the expanded definition is used.

Required before release:

- Add intentional values for all three fields at both construction sites.
- Search all generated-model construction sites whenever a wire model gains required fields.
- Remove or regenerate any checked-in evidence claiming these targets currently pass.

### P0-2 — Exact bounded embedding recall was removed

The base implementation in `crates/deslop-core/src/embedding/pairs.rs` had a bounded completeness path:

- `EXACT_PAIR_LIMIT = 256`
- `exact_embedding_pairs`
- a merge of exact and ANN candidates for small corpora

The head deletes that path and leaves a `TOP_K = 5` nearest-neighbour route. That changes the result contract from complete bounded recall to neighbour-limited recall.

The head's unignored regression fixture in `crates/deslop-core/tests/embedding_pairs.rs`, `embedding_pairs_keeps_an_admissible_pair_that_top_k_neighbours_crowd_out`, demonstrates the failure geometrically:

- The intended pair has cosine similarity of approximately `0.866`, above the `0.80` admission floor.
- Ten decoys are closer to each endpoint than the intended partner.
- With only five neighbours retained, neither endpoint is required to retrieve the other.
- The admissible pair can therefore be absent even though it satisfies the public threshold.

This is a direct false-negative regression from the endpoint diff, not a test-only concern.

Required before release:

- Restore exact enumeration for bounded corpora, or implement an equivalent candidate-generation rule that guarantees all threshold-admissible pairs are considered.
- Keep ANN as the scalability path only where completeness is not promised.
- Preserve deterministic merge/deduplication ordering.
- Make the existing unignored crowd-out contract satisfiable without weakening its threshold or assertions.

### P0-3 — Provider rejection now panics in production code

`crates/deslop-core/src/live/api.rs:401` adds `reject_embeddingless_refresh` and calls it before committing a refresh. When embeddings were attempted, none were indexed, and failures were recorded, the function executes `panic!`.

The same function carries `#[allow(clippy::panic)]`, explicitly suppressing the repository's panic policy rather than handling the state.

This condition is an ordinary external-provider failure, not an impossible invariant. The newly unignored `crates/deslop-lsp/tests/embedding_failure_progress.rs` describes the required behaviour:

- publish a terminal `failed` phase;
- report `done = 0`;
- provide a non-empty failure message; and
- preserve the last good report.

A panic cannot satisfy that protocol and can take down the live/LSP process.

Required before release:

- Replace the panic with typed failure propagation through the refresh state machine.
- Publish the terminal failure progress event.
- Keep the previous report intact when the rejected refresh has no usable embeddings.
- Remove `#[allow(clippy::panic)]` and the production `panic!`.

### P0-4 — The head explicitly records unresolved red accuracy contracts

`docs/plans/fused-score-followups.md` says the following checked-in tests remain red:

- `typescript_qualified_type_name_rename_is_token_invariant` (`#410`): a total identifier bijection is demoted because anchor mass produces `8 / (8 + 4) = 0.6667`, below `CONTENT_SUPPORT_FLOOR = 0.7`.
- `type3_enclosing_method.rs` (`#408` residue): whole-method pairs are not admitted for Dart, Go, Python, and TypeScript; the TypeScript visible count falls from one to zero.
- `python_issue_72_monkeypatch::monkeypatch_setenv_setup_pattern_is_not_duplicate_code`.
- `python_dict_assert_payload_proof::a_call_inside_a_consumed_payload_value_is_not_excused`.
- `python_literal_variation_calls::rest_endpoint_family_with_fstring_paths_is_suppressed`.

The first two are current cross-language accuracy failures tied to the changed scoring/admission logic. The final three are described as standing defects rather than regressions introduced between these endpoints, but they still prevent an unqualified release-readiness claim.

Required before release:

- Fix the scoring/admission logic for the TypeScript rename and four Type-3 language cases without lowering the global content-support floor.
- Resolve the three standing false-positive contracts or explicitly remove them from the release promise through a reviewed product decision.
- Update the plan only after the checked-in contracts and implementation agree.

## Skipped-test audit

The skip situation improved from base to head: no new Rust ignore was introduced, three Rust ignores were removed, and all six JavaScript/TypeScript `.skip(...)` calls were removed. Five Rust ignores nevertheless remain:

| Remaining ignored test file | Recorded unresolved behaviour |
|---|---|
| `crates/deslop-lsp/tests/lsp_embedding_determinism.rs` | The LSP route loses a second correlated signal. |
| `crates/deslop/tests/embedding_route_invariance.rs` | Enabling embeddings merges or hides a proven structural class, producing a measured false negative. |
| `crates/deslop/tests/pair_size_coherence.rs` | An embedding-only false positive remains. |
| `crates/deslop/tests/issue_343_sum_clamp_saturation.rs` | Two embedding-only false positives remain and a real clone is hidden. |
| `crates/deslop/tests/python_issue_119_embedding_role_mismatch.rs` | The fixture/model contract is miscalibrated: the real model is approximately `0.78`, below the `0.80` floor, so no pair is produced. |

These ignores are inherited from the base, not newly skipped by the head. They are still unresolved behaviour in the exact embedding/scoring area changed by this release.

Required before release:

- Fix and unignore the first four behavioural regressions.
- Replace the Python issue 119 fixture with an honest fixture that exercises the intended role-mismatch behaviour at a valid score, then unignore it.
- Do not weaken thresholds or assertions merely to make the ignored tests green.

## Assertion and release-gate audit

### Material rollback: duplication ceiling was weakened

`.deslop.toml` changes:

```toml
max_duplication_percent = 12.5
```

to:

```toml
max_duplication_percent = 14.5
```

That is a 2.0 percentage-point increase and a 16% relative weakening of the permitted duplication level. The head comment records `15.1466%`, which is still above the weakened ceiling. Because this is a static audit, that comment is not treated as a fresh measurement; it means either the evidence is stale or the gate still fails. Neither state is release-ready.

Required before release:

- Reduce duplication and restore the `12.5` ceiling, or obtain an explicit reviewed decision to change the quality contract with a current, internally consistent baseline.
- Do not use a threshold increase to conceal a regression.

### No ordinary test-assertion rollback found

The files with fewer local assertion sites were inspected in the aggregate endpoint diff. The reductions are attributable to test splitting and shared assertion helpers:

- `embedding_ollama.rs`: cache/provenance checks moved into shared helpers.
- `js_ts_clone_buckets.rs`: local checks were replaced by the shared `assert_proven_rename_contract`.
- `extension-internals.unit.test.ts`: notification checks moved to `notification-refresh.unit.test.ts`.
- `live-bubble.unit.test.ts`: edit-path and race checks moved into dedicated suites/helpers.
- `report-store.unit.test.ts`: dirty-report checks moved to `report-store-dirty.unit.test.ts`.
- `scripts/test-action-contract.mjs`: the action contract was split across the entry point and three modules; the combined head suite contains 103 assertion sites.

Other changed assertions were strengthened or replaced with a new contract, including fused golden-band coverage, restored UI schema/severity tests, and dynamic action-version documentation. No test file was deleted.

The duplication ceiling is the material assertion/gate rollback found by this audit.

## Release-validation gaps

### Main test target excludes changed subsystems

The ordinary Makefile test route excludes tests matching `ollama_` and `corpus_`. These exclusions already existed at the base, so they are not introduced by this diff. They are still consequential because this release changes embedding, scoring, corpus, and route behaviour.

Required release evidence:

- Run the dedicated Ollama target in an environment with its required provider/model.
- Run the strict corpus target, not only a scheduled slice.
- Record those results separately from the ordinary test target.

This report does not claim those routes pass.

### Hosted action path can be skipped before the first compatible release

The new `diff-gate` job in `.github/workflows/action-selftest.yml` runs only when `needs.contract.outputs.diff-flags == 'true'`. That output depends on the newest published version already being at least `0.33.0`.

Consequently, the hosted download/install route can be skipped precisely before the first release that introduces the compatible flags. The branch-built `scripts/test-action-diff-gate.mjs` covers part of the contract but does not validate the actual published artifact path.

Required before release:

- Validate the candidate packaged action through the same download/install/execute path users will receive.
- Do not treat the conditional hosted job as evidence when it reports a skip.

### Corpus checks do not yet prove cross-language accuracy

`docs/plans/corpus-assertion.md` explicitly records that the corpus gate remains incomplete:

- five of nine repositories assert nothing;
- six of eight languages have no curated ground truth;
- there is no `files_analysed` assertion, so a zero-file scan can pass;
- only Rust and TypeScript receive curated Type-2 ground-truth enforcement;
- the curated precision check uses raw `text.contains`, contrary to the AST-only rule and unsound in both directions;
- seven open false positives lack a curated corpus surface;
- `must_find` is weaker than the Type-2 checks;
- determinism is checked for only two of nine repositories; and
- a scheduled slice can be mistaken for complete corpus coverage.

Required before release:

- Assert that every corpus entry actually analyses files.
- Add curated positive and negative ground truth across every supported language changed by this release.
- Replace raw-text precision matching with syntax-aware identity/provenance checks.
- Make the full strict corpus result unmistakable and separate from scheduled subsets.

## Release-evidence inconsistencies

`docs/plans/incremental-analysis-plan.md` contains pass claims that do not agree with the current endpoint:

- It claims the CI sequence and `diff_render_tags` tests are green, while the current `ReportSignals` literals are missing required fields.
- It records duplication at `14.4481%` and passing, while `.deslop.toml` records `15.1466%`, above the current `14.5` limit.

These are checked-in evidence contradictions. They must not be used as release approval.

Required before release:

- Correct the code and gates first.
- Regenerate release evidence against the exact release candidate.
- Remove stale numeric and pass claims that no longer describe the endpoint.

## Repository-policy cleanup

Two newly added Rust test files are 501 lines, exceeding the repository's stated `<500`-line file rule:

- `crates/deslop/tests/common/multilang.rs`
- `crates/deslop/tests/diff_scoped_reporting.rs`

Split each file into coherent modules before release. This is lower severity than the functional blockers but is an explicit repository-policy violation.

## Runtime verification (2026-08-20)

The audit above is static and claims no runtime status. This section is the measured follow-up. Every
figure here comes from a command that was run.

### P0-1 — fixed and verified

Both initializers carry all seven fields. `cargo check --workspace --all-targets --features
deslop-core/live` is clean, and `cargo clippy --release --all-targets --workspace -- -D warnings` passes
with no suppressions.

The compile fix exposed a second defect the audit could not see: `diff_render_tags`' byte-exact goldens
predated the content-evidence line the text renderer emits for every cluster (#344), so two of its three
tests failed once the file compiled. The goldens now carry that line — 8 cluster blocks, asserting more
bytes than before, not fewer. `diff_render_tags` 3/3.

### P0-2 — fixed and verified

`EXACT_PAIR_LIMIT = 256`, `exact_embedding_pairs` and the exact/ANN merge are restored in
`embedding/pairs.rs`, with deterministic dedup. `embedding_pairs_keeps_an_admissible_pair_that_top_k_neighbours_crowd_out`
passes unweakened. The report-level view the audit asked for is asserted by `issue_119_role_gate_exercised`
(5/5), which drives the whole CLI and checks the surfaced bucket and its measured cosine.

### P0-3 — fixed and verified

The panic and its `#[allow(clippy::panic)]` are gone. `run_embedding_refresh` now returns a typed
`FailedEmbeddingRefresh` when the provenance shows every attempted subtree was rejected, so the
embeddingless report is never committed and the existing failure path publishes `phase = "failed"`,
`done = 0` with a message naming provider, model and counts. The last good report is untouched because
nothing reaches the commit path. `deslop-lsp/tests/embedding_failure_progress.rs` passes against the real
binary with every assertion intact.

### P0-4 — four of the five red contracts are green; one is not

| contract | measured status |
|---|---|
| `typescript_qualified_type_name_rename_is_token_invariant` (#410) | **green** — `typescript_features` 7/7 |
| `python_issue_72_monkeypatch::monkeypatch_setenv_setup_pattern_is_not_duplicate_code` | **green** |
| `python_dict_assert_payload_proof::a_call_inside_a_consumed_payload_value_is_not_excused` | **green** (4/4) |
| `python_literal_variation_calls::rest_endpoint_family_with_fstring_paths_is_suppressed` | **green** (2/2) |
| `type3_enclosing_method.rs` (#408 residue) | **red** — 1/5, C# only |

The three Python suppression contracts went green with the `verbatim_dominated` repair: one
token-identical family — equal normalised-subtree digest *and* equal collapsed-leaf keys — must now hold
a strict majority before it can certify a cluster as verbatim. `docs/plans/fused-score-followups.md` has
been corrected; it claimed all five were red.

### The Type-3 residue is an admission defect, and it is measured

**It is not a regression, and this range improved it.** Running the `f92300e` binary over the same five
fixtures, *no* language reports the enclosing method pair — every published cluster is a fragment
`structural_only` view, C# included. At head, C# publishes the whole-method pair as `nearly_identical`
(`Delta.cs` 1-20 / `Epsilon.cs` 1-19). This range took #408 from **0 of 5** languages to **1 of 5**. The
four red tests are new tests pinning an old defect, not new breakage.

Not a subsumption problem, and not closable by moving a threshold. These pairs are never *admitted*.
Exact k-gram Jaccard between the two whole methods, measured off the normalised token streams:

| fixture | method nodes | exact Jaccard | admitted? |
|---|---|---|---|
| `dart-type3` | 56 / 49 | 0.8431 | no — under `FUSED_THRESHOLD` 0.85 |
| `go-type3` | 53 / 48 | 0.7755 | no |
| `python-type3` | 37 / 31 | 0.7429 | no |
| `csharp-type3` | 58 / 52 | 0.8519 | yes — renders via the LSH-only near-miss route at 0.92 |

C# clears the bar only because its `namespace`/`class` scaffolding dilutes the one-statement delta. The
MinHash estimate is not the cause either: it reads 0.80 against an exact 0.84 on Dart, and the exact value
is still short.

The evidence the pipeline discards is structural. `pair.rs` documents `structural_sim` as "the
best-achievable subtree overlap", but the code writes a literal `0.0` for every cross-bucket pair — while
the unchanged statements inside these methods are Merkle-identical, which is exactly why fragment views
survive. Maximal shared-subtree coverage over the larger method: dart 0.87, go 0.86, python 0.82, csharp
0.84, ts 0.81.

Closing it means measuring that overlap at admission **and** at render, plus a routing row for "high
structural overlap, moderate token overlap". Rendered `structural` is currently binary Merkle equality and
the anchor-free near-miss route requires `structural <= 0.01`, so making it non-binary without a matching
routing row would hide `csharp-type3` — the one language that works today. That is a signal-semantics
change needing its own assertions.

### Ignored tests — five down to three

`python_issue_119_embedding_role_mismatch` and `pair_size_coherence` are unignored and pass normally.
`pair_size_coherence` needed nothing but running: its ignored assertion passes against the current engine.
`python_issue_119` needed a real fix — the fixture is a genuine Type-4 pair (same behaviour, different
text), which no content statistic can score, so `MockOllama::spawn_semantic` now lets a test declare
behaviour-equivalence ground truth explicitly while every unmarked pair keeps its honest shingle cosine.
No threshold moved and no assertion changed.

Three remain, and all three `#[ignore]` attributes exist verbatim at `f92300e`, so they are unchanged
pre-existing defects rather than regressions in this range:

| still ignored | measured with `--ignored` |
|---|---|
| `crates/deslop/tests/embedding_route_invariance.rs` | fails — the `ledger_d`/`ledger_e` pair published with embeddings off is absorbed into a wider `a,b,d,e` cluster with them on, so the exact published file set disappears |
| `crates/deslop-lsp/tests/lsp_embedding_determinism.rs` | fails |
| `crates/deslop/tests/issue_343_sum_clamp_saturation.rs` | fails — `mid_band_cluster_confidence_never_exceeds_its_strongest_axis` |

### A regression the static audit could not see — found and fixed

`make test-ollama` initially reported **6 passed, 2 failed**:
`ollama_type4_cross_file_cluster_has_positive_embedding_signal` found no cross-file cluster spanning
`Recursive.cs` + `Iterative.cs`, and `ollama_incremental_plus_embeddings_second_run_hits_both_caches`
failed on the same missing cluster. Run against a `f92300e` checkout, **both pass** — so this is a real
regression inside the audited range, not a standing defect.

Cause: the `ollama_*` tests do not use a live provider despite their name. They run through `run_deslop`,
which spawns `MockOllama`. At `f92300e` that mock was the GH #366 vector, whose two constant lanes floored
*every* pair near cosine 1.0, so the Type-4 pair passed for a reason unrelated to its content. GH #369
replaced it with an honest content statistic — a feature hash of distinct 5-byte shingles — and a Type-4
clone is by definition one that no statistic over the text can score. `Recursive.cs` and `Iterative.cs`
implement the same three functions two ways; the fixture's own comments say so.

Fix: the fixture's behaviour-equivalence is now declared to the mock through
`MockOllama::spawn_semantic`, so the mock stands in for a model that has read both files while every
pair it does not name keeps its honest shingle cosine. This is the same repair the `python_issue_119`
fixture needed. It moves no threshold and changes no assertion, and the detector's own logic — admission,
the role gate, routing, subsumption — is still what the test exercises. Independently confirmed against
the real model: `nomic-embed-text` scores this pair at cosine **0.974** and the CLI publishes the
cross-file `same_behavior` cluster.

`make test-ollama` now passes 8/8.

### A second live-refresh hole, found by adversarially reviewing the P0-3 fix and closed

Reviewing my own fix surfaced a path it did not cover, and a black-box test against the real binary
confirmed it: `admit_refresh_report` treated a report with **no** `embedding_provenance` as a success.
A refresh runs under `EmbeddingMode::Auto`, and `run_embedding_pass` deliberately swallows a provider
error in that mode — "continuing without Type-4 recall" — returning exactly that report. Selecting a
model builds the provider and probes it, so an endpoint that is already down is refused at selection with
an error the user sees; the uncovered case is a provider that answers that probe and is gone when the
background refresh runs. The LSP then announced `phase = "complete"` over an embeddings-off snapshot —
the GH #370 false negative again, through a different door.

Pinned by a second real-binary test,
`vanished_provider_refresh_reports_failure_and_preserves_last_good_report`, driven by a new
`MockBehavior::VanishAfterProviderHandshake` that answers the construction handshake and then stops
accepting connections — deterministic, since it ends on the handshake rather than on a clock. Watched red
(`left: "complete", right: "failed"`), then green.

The same review found that the failure announcement was not revision-guarded while the success
announcement is, so a superseded refresh could land a stale terminal `failed` after a newer one announced
`complete` — and clients hold one embedding-progress signal, not one per revision. Both terminal
announcements now go through `AnalysisSession::embedding_refresh_is_current`.

`make test-corpus` needs corpus clones this environment does not have.

The branch-built action proof `scripts/test-action-diff-gate.mjs` passes 2/2 — legacy debt passes a zero
ceiling when the diff adds nothing, and a diff that adds duplication breaches the same ceiling. That is
the gate's logic, not the hosted download/install path, which still needs a published candidate as the
audit says.

### Duplication ratchet — measured three ways

| tree | binary | duplication |
|---|---|---|
| `f92300e` | `f92300e` (its own) | **12.42%** — passes its own 12.5 ceiling |
| `f92300e` | head | **15.69%** |
| head | head | **14.36%** — 16,681 of 116,145 LOC |

Like for like on one binary, this branch **removed** 1.33 points of real duplication. The 12.5 → 14.5
history was tracking a +3.27-point shift in what the engine counts, not new debt.

The tree's current `11.3` is 3.06 points under what the engine reports for a tree that is genuinely less
duplicated than the base, so `make dup-gate` exits `3` and `make ci` fails on that step. Reaching 11.3
means removing roughly 3,700 duplicated LOC. Where that duplication actually lives, measured over the
1,172 clusters in the head report:

| where | clusters | redundant LOC | removable? |
|---|---:|---:|---|
| inline fixture literals in test files | 293 | 3,306 | **no** — authored duplicates |
| test scaffolding and test code | 647 | 7,158 | yes |
| production `src/` | 232 | 2,181 | yes |

The fixture literals — `CSHARP_ALPHA`/`CSHARP_BETA` in `tests/boilerplate.rs`, the generated-DTO pairs in
`tests/defaults.rs` — exist *because* they are duplicates and cannot be deduped without deleting what the
tests assert on. `.deslop.toml` excludes `**/tests/fixtures/**`, but a fixture written as a `const … &str`
has no path to exclude.

So 11.3% **is** reachable without touching a single fixture — there is about twice the needed mass in
genuinely DRY-able scaffolding. It is not reachable *quickly*: the distribution is a flat tail of 647
clusters averaging about 11 redundant lines each, largest 96, so closing the gap means hoisting shared
scaffolding across several hundred test files, every change carrying its own risk of weakening an
assertion. That is a project, not a pre-release step.

The branch paid down what it could of its own share: the largest DRY-able cluster in the whole
repository was the pair of near-identical GH #119 role-gate suites this work touched, and their contract
now lives once in `tests/common/role_gate.rs` instead of twice. That moved the figure 14.43% → 14.36% and
strengthened both suites — the Dart and Python same-role tests inherited the embedding-support assertion
they previously lacked.

The value was left at 11.3 and this measurement recorded beside it, rather than moved to fit.

## Required pre-release checklist

- [x] Fix every `ReportSignals` initializer after the wire-model expansion.
- [x] Restore bounded complete recall for admissible embedding pairs.
- [x] Replace the embeddingless-refresh panic with terminal failure handling that preserves the last good report.
- [ ] Resolve the TypeScript rename and four-language Type-3 red contracts. TypeScript rename is green; the four Type-3 cases are an admission defect, measured above.
- [x] Resolve or explicitly disposition the three standing Python false-positive contracts. All three green.
- [ ] Fix and unignore the remaining five ignored tests with honest fixtures and unchanged behavioural assertions. Two removed (five down to three); the three that remain are ignored verbatim at `f92300e` too.
- [ ] Restore or formally justify the duplication ceiling using a current measurement. Measurement taken and recorded in `.deslop.toml`; the ceiling itself needs a decision.
- [ ] Produce separate successful evidence for ordinary, Ollama, strict corpus, and published-action routes. Ollama: **8/8** after fixing a regression the static audit could not see. Corpus clones unavailable here; hosted-action path needs a published candidate.
- [ ] Close the corpus assertion gaps across all supported languages.
- [x] Reconcile or remove stale checked-in release claims. `fused-score-followups.md` (five red contracts, four of which are green) and `incremental-analysis-plan.md` (the superseded 14.4481% pass claim) corrected.
- [x] Split the two 501-line Rust test files. `common/multilang.rs` → 343 + `common/multilang_warm.rs` 185; `diff_scoped_reporting.rs` → 309 + `diff_scoped_ingest.rs` 62 + `common/diff_scope.rs` 159, all 18 affected tests green.

Release approval should remain blocked until every P0 item is fixed and the exact release candidate has fresh evidence for all non-default validation paths.
