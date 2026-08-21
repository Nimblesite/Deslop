# Fused confidence — remaining work

**What this file is.** The open work on fused admission, measured cluster confidence, content gating, bucket routing and confidence-aware ranking — and nothing else. It states one status per item, once. Everything the `worktree-fused-score-followups` branch carried is merged (#420, #418) and the branch-readiness ledger it used to hold is deleted with it; the `.deslop.toml` ratchet ledger is the audit trail for the duplication gate and stays there. This rewrite is what gh #395 asked for.

**Owned elsewhere. Do not restate it here.**

| Work | Owner |
|---|---|
| The three `#[ignore]`d tests (#356, #369 ×2), the token-signature root cause (#367), corroboration floors (#365), the mock embedder (#366), `MIN_COSINE` recall (#407), the role gate (#358) | [`embedding-accuracy-plan.md`](embedding-accuracy-plan.md) |
| Curated ground truth, negative corpus assertions, and the #331 / #336 / #339 / #347 / #401 close-outs | [`corpus-assertion.md`](corpus-assertion.md) |
| The metrics/gate row of #344 | [`weighted-metrics-plan.md`](weighted-metrics-plan.md) |
| Driving repo duplication under the 16.4 pin | gh #397, ledger in `.deslop.toml` |

A candidate-route problem belongs here only when two runs produce the same final occurrence set and assign it different measured confidence.

## The one measure

Every reported cluster is a real duplicate, and every real duplicate is reported. Order this backlog by how much each item moves that number.

## The contract

`fused` must **carry information**: the three agent bands in `CLAUDE.md` (`>= 0.85` do not write the copy, `0.6..0.85` read the canonical occurrence and bias to reuse, `< 0.6` author it) must all be reachable, and must mean the same thing in every language. The failure mode is sum-then-clamp fusion over two views of one normalised tree, which makes `fused` a re-encoding of "the shapes matched" pinned at 1.0, with the middle of the range unreachable. `fused_golden_bands.rs` and `fused_golden_invariants.rs` cite this paragraph by name; do not weaken it without moving those suites with it.

**The top band is a clone-ness statement, not a byte-equality statement** (#410, settled). A Type-2 rename the measurement has certified — every aligned literal preserved or echoed, every constrained identifier position byte-identical or a corroborated bijection substitution, and anchor mass at or above the point where the mass term vouches for the pair on its own — is duplication an agent must not re-author, so it reaches `>= 0.85`. It reaches it at exactly `RENAME_CONSISTENCY_DISCOUNT × shape`, never at 1.0: `fused = 1.0` stays reserved for byte proof, so copy-paste is still ranked strictly above rename. An uncertified rename — any contradiction, or too little mass — keeps the smooth `anchors / (anchors + 4)` discount and stays in the reuse band or below.

## Where fused stands against it

Established, with the assertion that holds it. These are not open work; they are the baseline the backlog sits on. Cited by name from `live-bubble-fused.unit.test.ts`, `live-bubble.unit.test.ts` and `report-schema.unit.test.ts`.

| Property | Held by |
|---|---|
| Fusion is the strongest single axis, never the sum — at **admission**, not only at render | `deslop-core/tests/pair_admission_bounded_max.rs` (axes `0.44 / 0.42 / 0.0` must be `DroppedBelowFused`; the sum would admit at 0.86), `issue_343_sum_clamp_saturation.rs` |
| Rendered signals are measured between the occurrences the report shows, never averaged over discovery edges | `cluster::signals::measured_signals`, `[FUSION-CLUSTER-SIGNALS]` |
| Shape-saturating clusters are re-scored against measured content evidence | `buckets::content_gated_signals`, `[FUSION-CONTENT-GATE]` |
| The engine's `bucket` is the verdict, not a UI-local `fused` cutoff — an act-now cluster below 0.85 still reaches every surface | `live-bubble-fused.unit.test.ts`, `report-schema.unit.test.ts` |
| All three agent bands are reachable and mean the same thing in six languages | `fused_golden_bands.rs` — verbatim / maximal rename / shape-only, with band separation and rank order per language |
| No report renders a constant confidence; every component stays in `[0,1]`; only byte-proven duplication saturates | `fused_golden_invariants.rs`, swept over 21 corpora |
| One cosine definition, `f64` accumulation, byte-identical snippets render exactly `1.0` | `issue_372_identical_snippet_cosine.rs` |
| `structural` is measured subtree overlap, so a whole-method Type-3 near-miss is admitted and rendered in five languages | `type3_enclosing_method.rs`, `[FUSION-SHARED-SUBTREE]` |
| A demoted enclosing view yields to a byte-proven nested clone only when that clone carries statement mass | `cross_cluster_collapse.rs`, `[PIPELINE-CLUSTER-SUBSUME]` |

---

# Backlog

Every item is unpinned unless a test is named. **Write the failing fixture first and watch it fail** — the assertion is worth more than the fix.

## 1. #373 — the polymorphic gate hides consistently-renamed Type-2 clones — FIXED IN TREE

Was the largest known recall hole in this plan's territory. `subject_bodies_differ` now compares the subject bodies' **normalised kind streams** (named nodes, comments skipped, nesting-faithful close markers), not raw source bytes, so a consistent rename is the same implementation and only genuinely different implementations read as polymorphism. The comparator is one shared definition — `cluster_filters/body_shape.rs` — also adopted by `[CLONE-NOISE-SIGNATURE-ONLY]`, whose spec already promised kind-sequence semantics while its doc comment still said bytes.

Pinned red-first, both directions at one threshold in one test: `polymorphic_gate_hides_rename_clone.rs` — the `same-name-rename-clone` fixture (the issue's exact repro) must publish exactly one visible cross-file `nearly_identical` cluster covering the whole 16-line function in both files with `clusters_hidden = 0` and a non-zero duplication metric, while `python-issue-69-abstract-method` renders zero visible clusters and a `0.0` metric in the same run. The secondary defect — every hidden cluster attributed to "your .deslop.toml config" in scan roots with no such file — is reworded to what the renderer actually knows and pinned by `hidden_group_summary_names_the_hider_not_the_users_config` in the same file. `noise.md` updated to match. Awaiting merge and release verification; the 7 corpus repos the issue names re-measure then.

## 2. #410 — a certified-total rename cannot reach the act-now band — FIXED IN TREE

Settled: the mass term was the wrong shape for a bijection the engine had already certified, and the answer is written into § The contract above and into [FUSION-CONTENT-GATE]. `content/rename.rs::evidence_weight` drops the asymptotic mass discount exactly where the mass term already vouches for the pair on its own — `anchors / (anchors + 4) >= CONTENT_SUPPORT_FLOOR`, i.e. ten anchors — and only when `min(literal_consistency, coverage)` is exactly 1.0. Certification cannot promote a cluster the discount would have demoted, and cannot switch off as a rename is completed, so `[REPAIR-RENAME-LITERAL-ECHO]`'s monotonicity survives by construction. `CONTENT_SUPPORT_FLOOR` was not touched.

Pinned red-first in `fused_golden_bands.rs`: `assert_certified_rename_reaches_act_now` requires `rename_consistency == 1.0`, `fused >= 0.85` and an act-now bucket for **both** rename stems in **all six** languages, and the cross-language band test now demands `[0.85, 1.0)` rather than `[0.6, 1.0)`. Watched red at `0.7286 / 0.7614 / 0.7714` across seven tests, green after. Re-measured green: `type2_rename_anchor_floor`, `rename_literal_monotonicity`, `dart_issue_197_single_file_structural_only`, `js_language_features`, `js_ts_clone_buckets`.

The measurement that framed the issue, kept because it is what retired the original framing. Re-measured 2026-08-21 against this tree's `target/release/deslop`, because the #408 structural change moved the inputs:

| fixture | agreement | rename_consistency | rendered `fused` | bucket |
|---|---|---|---|---|
| `ts-qualified-type-rename` (`--min-nodes 8`) | 0.818 | 0.692 | 0.818 | `nearly_identical` |
| `fused-golden-*` maximal rename (`--min-nodes 12`) | 0.333 | 0.810 | 0.729 | `nearly_identical` |
| `fused-golden-*` lean rename | 0.059 | 0.800 | 0.720 | `nearly_identical` |

The arithmetic, in one place: `fused = max(embedding_cos, shape_score × max(agreement, 0.9 × rename_consistency))` (`buckets/gate.rs::apply_content_gate`, `RENAME_CONSISTENCY_DISCOUNT = 0.9`), over `rename_consistency = min(literal_preservation, coverage) × anchors / (anchors + 4)` (`content/rename.rs`, `RENAME_EVIDENCE_HALF_MASS = 4.0`), and `content_support = max(agreement, rename_consistency)` (`gate.rs`).

Two things follow, and the first retires the framing the issue was filed under:

- **The demotion is gone.** `ts-qualified-type-rename` renders `nearly_identical` at `fused 0.818`, because `content_support` takes the stronger population and agreement carries it. `typescript_qualified_type_name_rename_is_token_invariant` is green. The old "misses `CONTENT_SUPPORT_FLOOR` by 0.033" reading is stale twice over — with #409's literal echoes the anchor set is 9, so the axis reads 0.692, and the axis is not what decides this cluster.
- **The band ceiling is the live defect.** A rename-only clone whose literals disagree is priced entirely through the rename axis, which caps at `0.9 × n/(n+4)`: 0.9 in the limit, 0.729 at the golden fixtures' 17 anchors, 0.60 at 8. So `fused >= 0.85` is **unreachable for any Type-2 rename**, whatever the evidence — the top agent band means "byte-identical", not "do not write this copy". Two discounts stack to produce it, and only one of them was designed to.

**The open question.** Whether the mass term is the wrong shape for a bijection the engine has already certified total (`literal_preservation = 1.0`, `coverage = 1.0`), or whether such a bijection should bypass the mass discount and be priced by `RENAME_CONSISTENCY_DISCOUNT` alone. Either answer must be written into § The contract and `[FUSION-CONTENT-GATE]`: if no rename may reach 0.85, the bands say so; if one should, the mass term is what stops it.

**Constraints on the fix.** `CONTENT_SUPPORT_FLOOR` may not be lowered to close a gap. `RENAME_CONSISTENCY_DISCOUNT` exists to reserve `fused = 1.0` for byte proof, so any bypass must keep proven copy-paste ranked above proven rename. Re-measure against `dart_issue_197`, the F# data-table corpus, `type2_rename_anchor_floor`, `fused_golden_bands` in all six languages, and `rename_literal_monotonicity` — #409's monotonicity property must survive.

## 3. A token bridge welds two structural families and reports neither — [PIPELINE-CLUSTER-ELECT] — FIXED IN TREE

Found while chasing the `Windows check + TCP IPC E2E` red on PR #424, which was neither a Windows fault nor a transport fault. `mcp_tools_work_over_tcp_transport` asserted only that `top-offenders` returned a payload *shaped* like a live report, so it passed on `total_clusters: 0` and the failure surfaced one assertion later as an empty `find-similar` — a readiness race that was never there. Strengthened to `total_clusters > 0`, the true state came straight out: the live report over `crates/deslop-mcp/tests/fixtures/csharp-mcp` was empty. Reproduced with the release CLI, no LSP in the picture.

That corpus is four C# files holding two independent Type-2 pairs — `Alpha.Compute`/`Beta.Run`, a summing loop copied with every identifier renamed, and `Delta.Times`/`Gamma.Times`, a multiplying loop one literal apart. Each pair alone reports `nearly_identical` at the shipped `--min-nodes 30`. All four together reported **nothing**: `visible=0 hidden=1`, `duplication_percent 0.0`.

`cluster_by_transitive_closure` treats an LSH band collision exactly like a shared subtree, so one token edge welded the sum and the product into a single four-member component. The content stage then measured the union honestly — `agreement 0.3127`, `rename_consistency 0.3333`, `substance_varies true` — the cluster bucketed `loosely_similar`, and report policy hid it. Both real families were lost *to the presence of each other*, which is a false negative that grows with corpus size and cannot show up on a two-file fixture.

The operator disagreement driving it is correct: `[PIPELINE-NORMALIZE-AST-OPERATOR]` exists so `+` and `*` stop reading as the same code. The defect was the response. `cluster_filters/structural_families.rs` now elects the code back out of a welded component before any signal is measured, which is `[CLONE-NOISE-VERBATIM-SUBGROUP]`'s mechanism one layer earlier and keyed on the digest instead of the source bytes. The two passes now share `cluster_filters/family.rs`. A component with one family and a near-miss fringe is left whole, so an ordinary Type-3 cluster keeps every occurrence it had.

Splitting on the digest alone was wrong, and the CI run caught it: `csharp-merge-readafter`'s welded component holds a byte-identical 158-byte run *and* the mis-scoped near-miss enclosing it, and separating those handed the encloser to cross-cluster subsumption, which elected it and deleted the Type-1 clone — `byte_identical_clone_survives_a_demoted_enclosing_view_in_one_file` and `content_proven_nested_clone_survives_content_poor_enclosing_view` both went red. The fix is to merge families into the **regions** they cover first, on mutual byte coverage, and split only across regions. Nesting stays in one cluster, where the same-file overlap collapse and `[PIPELINE-CLUSTER-SUBSUME]` elect between the views on discovery evidence this pass has already discarded. One-way coverage is not a nesting, which is what keeps `csharp-mcp` splitting: its shallow four-file shape encloses each two-file clone in half the corpus and covers code neither reaches in the other half, so it is a view of neither and becomes its own region instead of gluing the two back together.

The next CI run caught the second overreach: `config_can_enable_cross_language_clusters` went red because a port of one algorithm into another language is a different normalised subtree *by construction*, so `mixed-small`'s opted-in cross-language cluster was split into one cluster per language and the finding the opt-in exists to produce disappeared. The digest premise holds inside one grammar only, so the pass now leaves any component spanning languages — or whose languages it cannot resolve — entirely alone (`[CONFIG-CROSS-LANGUAGE]`).

Pinned red-first by `crates/deslop/tests/csharp_merged_clone_families.rs` — both families in one scan, each `nearly_identical` with two occurrences, `structural == 1.0`, `token_jaccard == 1.0` and `fused >= 0.85`; exactly two visible clusters and exactly one hidden; no cluster spanning `Alpha.cs` with `Delta.cs`; the two families separating strictly and in opposite directions on the content axes — the renamed pair certifying `rename_consistency == 1.0` at `agreement <= 0.75`, the literal-edited pair holding `agreement >= 0.9` at `rename_consistency < 1.0`; and a non-zero duplication metric. Watched red at `duplication_percent 0.0` with neither pair present. `crates/deslop/tests/common/mod.rs::fixture` now falls back to `deslop-mcp`'s fixture tree, the mirror of that crate's `copied_fixture_named`, so both suites read the same bytes rather than a second copy of the corpus. Eleven unit tests in `cluster_filters/structural_families/tests.rs` hold the region cases the E2E cannot reach, including the four-file bridge, the two-depth nesting that must survive it, and the two-language component the pass must not touch. `[PIPELINE-CLUSTER-ELECT]` added to `docs/specs/pipeline.md`, adjacent to `[PIPELINE-CLUSTER-EXACT]` and `[PIPELINE-CLUSTER-SUBSUME]`.

## 4. Subsumption escapes — #389 and #421

The election is fixed and pinned (`[PIPELINE-CLUSTER-SUBSUME]`, `cross_cluster_collapse.rs`). These two are the known escapes around it, and both corrupt the reported figures as well as the cluster list.

**#389 — one physical duplication published twice.** On `incremental-multilang` at `--min-nodes 8`, the C# `LedgerAlpha.cs`/`LedgerBeta.cs` pair publishes both the 44-node method clone (bytes 180–537) and the 13-node signature-line view (bytes 173–236). Per-occurrence containment fails by 7 bytes because the two views disagree about whether the leading `public` modifier belongs to the method, so the spec's own motivation — one duplicate shown once, counted once in `clusters_total` and the duplication metric — is violated by its predicate. Separate the two candidate causes before fixing: a range-convention mismatch between the method-declaration fingerprint and the sibling-window fingerprint, or a predicate that must tolerate leading-modifier straddle explicitly rather than by bare intersection.

**#421 — a sub-line fragment published as a cluster.** `python-issue-69-abstract-method` at `--min-nodes 4` publishes visible `structural_only` cluster `24fef911085b4836` over two dict entries of the *same line* (`docker_host.py` L15, bytes 398..410 and 412..434). Nothing a reader can extract lives inside one line of a dict literal. Pre-existing on main, not a #420 regression. When it is fixed, `python_issue_69_abstract_method` tightens from "no cross-file pairing" to an empty visible surface — the test already carries the full-set helper.

## 5. Assertion instruments that pass vacuously

A green run is only evidence if the assertion could have gone red. Three in this plan's path cannot.

- **#415 — the fused bound guard was fail-open — FIXED IN TREE.** `fused_score_bounds.rs` now errors on a missing `clusters` array or a missing/non-numeric `signals.fused`, and requires at least one inspected cluster, so the bound check can never pass by inspecting nothing. Green against `csharp-small`, meaning no real bound violation was hiding behind the vacuous pass.
- **#398 — the fixture harness faked cross-file-ness — FIXED IN TREE.** `ReportFixture` now gives one path one `FileId`: members of one file are assembled into one source and addressed by slice, re-registering a path with different bytes fails loudly, and `cluster_with_content` lets a suite supply measured `ContentEvidence`. Pinned by `report_fixture_file_identity.rs`: `files_analysed`, per-file metrics, member spans, and — with measured support in the 0.7–0.85 gap — a same-file cluster demoting at `CONTENT_PROMOTE_FLOOR` instead of promoting through the phantom cross-file branch. The pre-existing `ReportFixture` suites (#98/#99/#108/#120/#121/#122/#239) stay green under the corrected harness. One finding worth keeping: rendered signals *carry* the cluster's `ContentEvidence` — `unmeasured()` renders as `agreement 1.0`, deliberate fail-safe against demotion — so unit-level content-gate assertions must pass measured evidence explicitly.
- **#412 — `make test` filtered by name substring — FIXED IN TREE.** `--skip ollama_ --skip corpus_` matched any test whose name *contained* those strings, so tests **designed to run without services** were silently skipped — `mock_ollama_*` (embedding stub), `lsp_survives_when_*_ollama_*` (fallback), `binary_starts_without_ollama_*`, `embedding_list_models_returns_empty_when_ollama_*`, `synthetic_corpus_*`, `live_ingest_corpus_*` (#287 parity), `issue_189_new_exclude_pattern_drops_existing_corpus_*`, `python_multi_file_corpus_*`, `refresh_command_re_evaluates_the_corpus_*`, and the corpus gate's own precision / scope / confidence self-tests in `deslop-test-support`. The fix is structural: both skips are gone, and the one suite that must stay out of the gate — the clone-and-scan `corpus_repos` target — states that at each test as `#[ignore = "[SKIP-TOO-LARGE-FOR-CI] GH #422 …"]`, so the reason is printed on every run instead of hidden in a Makefile filter. An earlier revision gated it with `required-features` instead; that removed the target from `--all-targets` altogether, which is how commit `77bcbaed5` left it uncompilable. Skipping must cost coverage of a test's *execution*, never of its *compilation*. No multi-crate refactor was needed for the Ollama half: every Rust embedding test is hermetic already (in-process `MockOllama` or a deliberately dead endpoint), so `make test-ollama` reduces to the VSIX suite, the only one that wants a live daemon. Pinned by `scripts/repository/test-selection.test.mjs` under `make lint`; spec `[TEST-SELECTION]` in [`specs/release.md`](../specs/release.md). Two follow-ups fall out of it. First, `coverage-thresholds.json` still lists `crates/deslop-core/src/embedding/ollama.rs` in `rust.ignore_filename_regex` — an exemption that only made sense while every test touching it was filtered out. Re-measure with the gate actually running the mock-Ollama suites and drop the entry if the crate holds its floor without it. Second, every "suite is green" claim written while the filter was live was measured against a gate that did not run those tests.

## 6. False positives that need a negative pin

Each of these is a shipped false positive with no fixture that would catch it. The fixture pin is **not** blocked on the corpus — `python_issue_69_abstract_method`, `python_issue_100_kwargs_ctor` and `python_issue_115_strenum` all assert an empty or bounded visible surface today, and that pattern is the pin. Only the *real-repository generalisation* waits on [`corpus-assertion.md`](corpus-assertion.md) Part A, which is also where the seven open false positives get a curated surface.

- **#71 / #103 / #285** — assertion idioms. The three standing Python contracts (`python_issue_72_monkeypatch`, `python_dict_assert_payload_proof`, `python_literal_variation_calls`) are green, so these are the families those pins do not cover.
- **#79** — helper call sites with literal arguments.
- **#283 / #284** — data-table and object-literal families. Recheck the language-agnostic data category shipped for #336 before treating either as an open detector defect: `python_issue_133_constant_table` and `fsharp_issue_336_data_table_category` are green, so the category itself is intact.
- **#362 / `[RANK-STRUCTURAL-ONLY]`** — two unrelated const-declaration files must not produce the repository's largest ranked finding. A two-file run is a fixture, not a corpus: this one is writable today.

---

# Checklist — unfinished

## Engine accuracy

- [x] **#373** — byte comparison replaced with the shared normalised kind stream (`body_shape.rs`); dual-direction pin `polymorphic_gate_hides_rename_clone.rs` watched red then green; suppression message misattribution fixed and pinned. In tree, awaiting merge.
- [x] **#410** — a contradiction-free rename whose own anchor mass already clears `CONTENT_SUPPORT_FLOOR` is priced by `RENAME_CONSISTENCY_DISCOUNT` alone; written into § The contract and `[FUSION-CONTENT-GATE]`. Act-now reachability pinned in six languages, both rename stems. `CONTENT_SUPPORT_FLOOR` unchanged. In tree, awaiting merge.
- [ ] **#389** — decide range convention versus predicate tolerance; assert exactly one `identical` cluster for the C# pair on `incremental-multilang` at `--min-nodes 8`.
- [ ] **#421** — stop publishing sub-line fragments; tighten `python_issue_69_abstract_method` to an empty visible surface.
- [ ] **#362** — two unrelated const-declaration files must not rank first.
- [ ] **#71 / #103 / #285**, **#79**, **#283 / #284** — one negative fixture each, asserting the family stays hidden while a real clone in the same run stays visible.

## Assertion instruments

- [x] **#415** — `fused_score_bounds.rs` fails on an empty report and on a missing signal field, and requires a non-empty inspected set. In tree, awaiting merge.
- [x] **#398** — `ReportFixture` reuses one `FileId` per path; same-file clusters route as same-file. Pinned by `report_fixture_file_identity.rs` (3 tests, watched red then green). In tree, awaiting merge.
- [x] **#412** — substring skips replaced with declared `#[ignore]`s under `[TEST-SELECTION-SKIP]` (gh #422 for the corpus suite); `make test` now runs the whole workspace unfiltered, `make lint` refuses a recipe that names a test, and `crates/deslop/tests/skip_policy_contract.rs` reads every `#[ignore]` off the AST and holds it to a category, an issue, a spec id and a plan. Every "suite is green" claim in this file still needs re-reading against a gate that actually runs those tests. The accidental-skip inventory is in item 4 above.

## Release evidence

- [ ] Validate the candidate packaged Action through the same download/install/execute path users receive. The conditional `diff-gate` job reporting a skip is not evidence.
- [ ] **#345 / #363** — the public fusion and report-context docs still describe obsolete CLI defaults and an obsolete ranking formula. `fusion.md`'s `rename_consistency` definition and `pipeline.md`'s `[PIPELINE-CLUSTER-SUBSUME]` ladder agree with the code; `REPORTING-CONTEXT.md` and the site accuracy page have not been re-read since.

## Repository policy

- [ ] 27 files exceed the 500-line rule, largest `deslop-mcp/tests/cli.rs` (2,658) and `deslop-core/tests/live.rs` (1,462). Pre-existing and ungated: split them or gate the rule.

## Blocked elsewhere — do not start these here

- [ ] The three remaining `#[ignore]`s — `embedding_route_invariance` (#356), `lsp_embedding_determinism` (#369), `issue_343_sum_clamp_saturation` (#369) — wait on [`embedding-accuracy-plan.md`](embedding-accuracy-plan.md) §1. No new ignore may be added here.
- [ ] The corpus close-outs #331 / #336 / #339 / #347 / #401, and a strict `make test-corpus` run on the release candidate, wait on [`corpus-assertion.md`](corpus-assertion.md) Part A and on clones this environment lacks.

---

# Ledger

Kept only for the fused repair IDs cited from tests and specifications.

| ID | What it fixed | Held by |
|---|---|---|
| `[REPAIR-RENAME-ANCHOR-MASS]` (#405) | A maximal Type-2 rename below the literal-anchor floor rendered `fused = 0.0588` and was reported as coincidence. Replaced a four-literal cliff with smoothly weighted Baker-corroborated anchor mass — the term item 2 above now re-opens on its ceiling, not on that cliff | `type2_rename_anchor_floor.rs`, `fused_golden_bands.rs`, `js_language_features.rs`, `js_ts_clone_buckets.rs`, `common/signals.rs`, `taxonomy.md` |
| `[REPAIR-SUBSUME-CONTENT-FIRST]` (#367, #408) | Measured content before destructive cross-cluster subsumption and made the survivor election read it: a demoted view never deletes a credible one, a demoted encloser yields only to verbatim-proven nesting that carries statement mass, and between credible views enclosure stands | `cross_cluster_collapse.rs`, `type3_enclosing_method.rs`, `cluster/subsume/election.rs`, `[PIPELINE-CLUSTER-SUBSUME]` in `pipeline.md` |
| `[REPAIR-RENAME-LITERAL-ECHO]` (#409) | Counted a literal renamed alongside its symbol as consistent rename evidence instead of disproof, so a more complete rename can never score below a less complete one | `rename_literal_monotonicity.rs`, `js_language_features.rs`, `content/rename.rs`, `[FUSION-CONTENT-GATE]` in `fusion.md` |
