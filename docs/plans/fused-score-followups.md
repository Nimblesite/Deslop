# Fused confidence — what is left

One document. It replaces `root-cause-fusion.md`, `quarantine-repair-plan.md` and
`worktree-fused-score-followups-regression-audit.md`, all three deleted.

**The mechanism is shipped and specified.** `[FUSION-STRATEGY-BOUNDED-MAX]`, `[FUSION-CLUSTER-SIGNALS]` and
`[FUSION-CONTENT-GATE]` in [`fusion.md`](../specs/fusion.md) are the contract, and they carry their own
reasoning. Nothing here re-explains them — this plan is only the open work, and every claim below names the
assertion or the source line that establishes it. The real-repository gate that has to settle the precision
claims is planned separately in [`corpus-assertion.md`](corpus-assertion.md).

## The one measure

Every reported cluster is a real duplicate, and every real duplicate is reported. Ordered by how much each
item moves that number.

## The contract

`fused` must **carry information**: the three agent bands in `CLAUDE.md` (`>= 0.85` do not write the copy,
`0.6..0.85` read the canonical occurrence and bias to reuse, `< 0.6` author it) must all be reachable, and
must mean the same thing in every language. `fused_golden_bands.rs` cites this paragraph; do not weaken it
without moving that suite with it.

## Where fused stands against it

Established, with the assertion that holds it. These are not open work; they are the baseline the open work
sits on top of.

| Property | Held by |
|---|---|
| Fusion is the strongest single axis, never the sum — at **admission**, not only at render | `pair_admission_bounded_max.rs` (axes `0.44 / 0.42 / 0.0` must be `DroppedBelowFused`; the sum would admit at 0.86), `issue_343_sum_clamp_saturation.rs` |
| Rendered signals are measured between the occurrences the report shows, never averaged over discovery edges | `cluster::signals::measured_signals`, `[FUSION-CLUSTER-SIGNALS]` |
| Shape-saturating clusters are re-scored against measured content evidence | `buckets::content_gated_signals`, `[FUSION-CONTENT-GATE]` |
| All three agent bands are reachable and mean the same thing in six languages, with the rename band pinned on **both** sides of the old literal-anchor line | `fused_golden_bands.rs` — verbatim / maximal rename / lean maximal rename / shape-only, with band separation and rank order per language |
| Rename evidence is Baker-corroborated anchor mass (`[TECH-PMATCH-BAKER]`: preserved literals + explained identifier positions, smoothly weighted), never a literal-count cliff; the parameter bijection is elected over substituted pairs alone, so a homonym byte-string cannot veto its own rename | `type2_rename_anchor_floor.rs`, `js_ts_clone_buckets.rs` and the `rename_lean` scenarios — all three through the single `common::signals::assert_proven_rename_contract`, so no two suites can judge one signal triple by different rules — plus `cli::logging::technical_mode_uses_type_taxonomy_in_breakdown_row`; the convicted side held by `issue_134_structural_only_not_nearly_identical.rs` (divergent literals), `js_language_features.rs` (the `js-classes` family, `structural = token = 1.00` at `fused = 0.16`) and `dart_issue_197_single_file_structural_only` |
| Content evidence tests each byte position once — the collapsed *frontier*, never a collapsed node plus the collapsed descendants it spans | `tokens::collapsed_leaves`, `js_language_features.rs` template-literal and optional-chaining clones |
| No report renders a constant confidence; every component stays in `[0,1]`; only byte-proven duplication saturates | `fused_golden_invariants.rs`, swept over 21 corpora |
| One cosine definition, `f64` accumulation, byte-identical snippets render exactly `1.0` | `issue_372_identical_snippet_cosine.rs` |

## The three gaps that are actually open

Not history — each is a property of the code as it stands today.

1. **Content evidence is a render-stage correction, not an ensemble member.** `attach_content_evidence`
   runs once per render, immediately after ranking
   ([`session/render.rs`](../../crates/deslop-core/src/pipeline/session/render.rs)). Pair admission
   (`pair::survival_decision`) and transitive closure never see it, so a cluster that content evidence would
   convict was already admitted, clustered and ranked on shape alone. The gate can lower a rendered
   confidence; it cannot stop a pair becoming a cluster.
2. **The evidence never reaches a consumer.** `ContentEvidence` measures `agreement`,
   `rename_consistency` and `literal_fraction`
   ([`content.rs`](../../crates/deslop-core/src/content.rs)) and logs all three at `debug`, but
   `ReportSignals` in [`live-ipc.td`](../models/live-ipc.td) carries only `structural`, `token_jaccard`,
   `embedding_cos`, `fused`. No report, panel, agent or black-box test can see *why* a cluster routed where
   it did. That is #344, section 2 below.
3. **The semantic axis is off by default.** `--embeddings` defaults to `off`, deliberately and per
   `[FUSION-SIGNALS-THREE-LAYER]` — the shipped CLI must produce a report on a machine with no Ollama. The
   consequence is still real: on a default run the ensemble is two correlated views of one normalised tree
   plus the content gate. The spec calls this "a measurable recall loss … only as a default that never
   blocks a first run", and every embeddings-on assertion in the tree is currently `#[ignore]`d — section 1.

---

# TODO

## 1. Seven assertions are `#[ignore]`d — every one is a live accuracy defect

Nothing is deleted or weakened: each carries an `#[ignore = "…"]` naming its issue and runs under
`cargo test … -- --ignored`. This is the top of the list because **every embeddings-on assertion in the
workspace is currently switched off**, which is why gap 3 above cannot be measured, let alone closed.
Measured 18 Aug: `cargo test --workspace --all-targets --features deslop-core/live -- --skip ollama_
--skip corpus_` exited 0 across 170 test binaries with exactly 8 ignored. #370 is now discharged, leaving
7, and the assertion it unblocked is **deliberately red** against
`[QUARANTINE-EMBED-REFRESH-COMPLETE]` — a sweep that stops there is reporting the quarantine, not a
regression. Work order for the rest: #356, #369, #357, #358.

- [ ] **[#369](https://github.com/Nimblesite/Deslop/issues/369)** — three ignores.
      `issue_343_sum_clamp_saturation::mid_band_cluster_confidence_never_exceeds_its_strongest_axis` renders
      two embedding-only false positives and hides the real clone; both false pairs carry `structural = 0`
      and `token_jaccard = 0` and survive on `MockOllama`'s length-residue cosine alone.
      `pair_size_coherence::an_embedding_only_pair_does_not_join_occurrences_of_different_size` and
      `lsp_embedding_determinism::lsp_embedding_refresh_is_bounded_and_reproducible` fail on the same
      mechanism. The known fix has an O(N²·D) blowup — that is the part to solve.
- [x] **[#370](https://github.com/Nimblesite/Deslop/issues/370)** — hang fixed and its `#[ignore]`
      removed. The stall was **not** a missing terminal frame. Measured with `sample(1)` against the wedged
      server: the main thread sat in `Stderr::write_all` → `pthread_mutex_lock_wait`. The harness piped the
      child's stderr and held the read end open without ever reading it, so the pipe buffer filled, the next
      `tracing` event blocked its thread while holding the subscriber's stderr lock, and the `tower-lsp`
      serve loop queued behind that lock and stopped answering. The rejection path hits it first because it
      logs per failed subtree and per bisect retry. `common::StderrDrain` now reads the child's stderr to
      EOF on a background thread; the binary went from 14m41s to 9.5s, and every LSP test in the tree loses
      the same latent deadlock. **The unignored test then exposed a live false negative** — see
      `[QUARANTINE-EMBED-REFRESH-COMPLETE]` below; it is red on purpose and stays red.
- [ ] **[#356](https://github.com/Nimblesite/Deslop/issues/356)** — two ignores in
      `embedding_route_invariance`, the blast-radius pins for `[REPAIR-COSINE-MERGE]`. `csharp-type3`
      publishes two `structural_only` clusters at `structural 1.0` with embeddings off and **one**
      `same_behavior` cluster with them on — proven duplication re-labelled as a semantic guess, the bucket
      following the discovery route, which `[FUSION-CLUSTER-SIGNALS]` forbids. `ts-mixed-band` publishes a
      four-file `nearly_identical` cluster off and **zero clusters** on. Restored cosines are changing
      cluster *membership* through the closure. Fix so a bucket is a function of a cluster's occurrences,
      never of which pass reached them.
- [ ] **[#357](https://github.com/Nimblesite/Deslop/issues/357)** — duplicate subtrees are not collapsed
      before ANN indexing (312 attempted / 312 indexed), `embedding_perf`.
- [ ] **[#358](https://github.com/Nimblesite/Deslop/issues/358)** — the Python role gate over-suppresses: a
      same-role, behaviour-equivalent function pair never surfaces, `python_issue_119`.

## 2. #344 — put the content evidence on the wire and in front of a reader

Closes gap 2. Until it lands, no black-box test can assert the gate's input.

| Surface | Renders today |
|---|---|
| HTML footer (`render/html_footer.rs`) | `structural token_jaccard embedding_cos fused` |
| Markdown (`render/markdown.rs`) | the same four |
| CLI text report (`render/text.rs`) | **no signals at all** — only the embedding provenance line |
| LSP diagnostics / code lens | **no confidence on any production surface** |
| Autofix gates (`refactor/preconditions.rs`) | bucket pre-filter + byte proof only |

- [ ] `agreement` / `rename_consistency` / `literal_fraction` onto `ReportSignals` in
      [`live-ipc.td`](../models/live-ipc.td), regenerate — never hand-write. **Population point:**
      `impl From<PairScore> for ReportSignals` converts the raw triple *before* content is measured and
      cannot carry them; [`content_gated_signals`](../../crates/deslop-core/src/buckets.rs) holds the
      `ContentEvidence` and is the one place every rendered cluster passes through. It must stamp all three
      on **both** paths — today it early-returns unchanged for `Identical` and for non-saturating shapes.
- [ ] Render the three fields everywhere `fused` renders — HTML footer, Markdown, VSIX `SignalStrip`,
      `HelpBubble`.
- [ ] Add them to `render/text.rs`, `deslop-lsp` and `refactor/preconditions.rs`, which carry no confidence
      at all today.
- [ ] Restore the 17 rename-showcase fixtures #341 softened from maximal to partial renames. Unblocked:
      `[REPAIR-RENAME-ANCHOR-MASS]` deleted the anchor floor that would have converted them into 17 false
      negatives.
- [ ] `metrics.duplication_percent` / exit-code gate — **delegated** to
      [weighted-metrics-plan.md](weighted-metrics-plan.md) under `[METRICS-REPO-WEIGHTED]`.

## 3. Close gap 1 — content evidence before admission

- [ ] Decide whether content evidence can be measured cheaply enough to gate **admission** rather than
      render, and pin the answer either way. It is one walk per member over already-normalised trees, so
      the cost argument has to be measured, not assumed.
- [ ] Whichever way that lands, pin the consequence: a shape-only family that the gate demotes at render
      must not have been *ranked* above a real clone on its way there. `[RANK-STRUCTURAL-ONLY]` currently
      carries this on the render side only.

## 4. False positives — open reports, none re-verified against the shipped gate

All seven predate `[FUSION-CONTENT-GATE]` and were attributed to the sum-then-clamp fusion that no longer
exists. **None has been re-run against the engine as it stands**, so the first task in each group is
measurement, not a fix.

**None is closeable until the corpus can express a precision assertion.** No manifest field today says
*"these two things are not duplicates"* — see section A of [`corpus-assertion.md`](corpus-assertion.md).

- [ ] **Assertion-idiom FPs** — [#71](https://github.com/Nimblesite/Deslop/issues/71) (same HTTP verb +
      status), [#103](https://github.com/Nimblesite/Deslop/issues/103) (pytest `monkeypatch` chains, fixture
      call-sites), [#285](https://github.com/Nimblesite/Deslop/issues/285) (TDBIN diagnostic tests grouped
      by assertion idiom). `python_issue_72_monkeypatch.rs` and the `python_dict_assert_*` suites already
      cover neighbouring idioms — re-measure before writing anything new.
- [ ] **Data-table / object-literal FPs** — [#283](https://github.com/Nimblesite/Deslop/issues/283) and
      [#284](https://github.com/Nimblesite/Deslop/issues/284) (unrelated object-literal tables, TDBIN
      scenarios). The language-agnostic data-table classifier shipped for
      [#336](https://github.com/Nimblesite/Deslop/issues/336) (`fsharp_issue_336_data_table_category.rs`),
      so these two may already be labelled `data` and policy-controllable — check before treating them as
      open.
- [ ] **Helper-call-site FPs** — [#79](https://github.com/Nimblesite/Deslop/issues/79), call sites
      distinguished only by literal arguments. `python_literal_variation_calls.rs` is the nearest existing
      pin.
- [ ] **[#362](https://github.com/Nimblesite/Deslop/issues/362)** `[RANK-STRUCTURAL-ONLY]` — a two-file run
      of unrelated const declarations reports as the repository's single largest cluster, 344 LOC.

## 5. Coverage directions the shipped repairs never closed

Each row adds the *direction* the current tests do not cover — the half of the contract that would still
pass if the fix silently inverted.

- [ ] `[REPAIR-SNAPSHOT-PATH-ORDER]` — add the reverse determinism cycle to
      `deslop-lsp/tests/history_determinism.rs` (add A→B, remove both, re-add B→A).
- [ ] `[REPAIR-SNAPSHOT-PATH-ORDER]` — make a `per_file` entry with no `live_paths` entry a hard
      `CoreError`, never a silent drop. A bookkeeping bug must not become a false negative.
- [ ] `[REPAIR-WATCH-EXCLUSION]` — assert the **negative** direction: with no opt-in, a new `node_modules`
      file must not enter the live report; and that artefact directories stay excluded regardless of the
      dependency opt-in.
- [ ] `[REPAIR-WATCH-EXCLUSION]` — config reactivity: flip `include_dependencies` in `.deslop.toml`, assert
      the live report converges with no restart.
- [ ] `[REPAIR-ADMISSION-PIN]` — end-to-end calibration above the unit pin: cosine **0.86** clusters and is
      visible; **0.82** (every axis below 0.85, old sum above it) yields `cluster_count == 0` **and**
      `clusters_hidden == 0`, because hidden-but-present means admission still happened. Blocked behind
      #369 — it needs a trustworthy embeddings-on run.

## 6. Measurements nobody has taken

- [ ] Record the **first embeddings-on corpus measurement**. Blocked on #356 and #369: a measurement taken
      now would record those defects as the baseline, which is what #347 exists to prevent.
- [ ] `workflow_dispatch` the corpus gate; close #331 and #336 only on a green run CI has seen.

## 7. Close-outs — evidence first, never on a run CI has not seen

Fixed and pinned; open only because nobody closed them. Each needs one verification pass **naming the
assertion**, not the run.

- [ ] **#339** — F# `token_jaccard` was byte-offset luck: two byte-identical windows at shifted offsets fell
      through to `blake3(hash, byte_range)` and shared nothing. Offset-invariant sibling-window signatures
      shipped in #392; `signatures::tests::issue_339_sibling_window_signature_is_offset_invariant`,
      `fsharp_issue_339_sibling_window_rename` (2) and `fsharp_issue_339_token_fallback_rename` are green.
      Re-measure F# `token_jaccard` on the corpus, then close — **before** #336, which is measured against
      that signal.
- [ ] **#343** — `bounded_fused()` is the only fusion; `pair_admission_bounded_max.rs` pins the arithmetic
      at admission and `issue_343_sum_clamp_saturation.rs` at render (its embeddings-on case is #369).
- [ ] **#351** — `add_embedding_pairs` calls `record_cosine` unconditionally; no quarantine `panic!` and no
      `clippy::panic` suppression survives anywhere in `crates/`.
- [ ] **#372** — `f32` cosine drift, fixed by #384 (`cosine_from_parts`, `f64` accumulation); pinned by
      `issue_372_identical_snippet_cosine.rs` and the unit tests beside the function.
- [ ] **#345** — doc drift; every row done, verify against the tree then close.
- [ ] **#336** — both halves are pinned at fixture level (`issue_331_336_shape_only_saturation.rs` for the
      saturation half, `fsharp_issue_336_data_table_category.rs` for the categorisation half). What is left
      is the curated `dotnet/fsharp` run, sections C and D of [`corpus-assertion.md`](corpus-assertion.md).
- [ ] **#331** — closed on *synthetic* evidence. Its real-repository confirmation rests on a check
      [`corpus-assertion.md`](corpus-assertion.md) shows is unsound. Re-verify, **reopen** if it does not
      survive.
- [ ] **#347** — corpus gate close-out: three consecutive green runs, closed by naming the runs.
- [ ] **#355** — the Dart single-file delegating-method family. `dart_issue_197_single_file_structural_only`
      carries no `#[ignore]` and every original assertion passes; verify, then close.
- [ ] **#394** — add a YAML-parsed event-matrix test proving a Dependabot **security** PR to `main` retains
      CI, dependency review, CodeQL and Action self-test, while routine version updates stay on
      `dependabot-upgrades`.
- [ ] **#395** — the plan marked the same work both open and fixed. This rewrite is the fix; close when it
      stays true.
- [ ] **#397** — repo duplication back to 12.5%. `.deslop.toml` is at `max_duplication_percent = 14.5`,
      raised twice because the *detector* got more honest (row-4 routing in every language, #390), never
      because duplication was added; each raise carries its measurement in the file. The remaining distance
      is a flat tail in the coarse E2E suites. Ratchet down as that work lands; never pay for it by
      re-hiding clusters.

## Order

```
#369 / #370 / #356  ──►  embeddings-on is measurable  ──►  first corpus measurement, #5 calibration
#344 (evidence on the wire)  ──►  #4 FP re-measurement  ──►  gap 1 (evidence before admission)
             corpus-assertion.md A–E  ──────────────────────┘   (supplies the precision pins)
```

The false-positive work waits on the corpus gaining a precision surface: until it has one, a fix and a
regression look identical on a real repository.

---

# Ledger

Kept only because these IDs are cited from test module docs and would otherwise dangle. One line each; the
reasoning lives in [`fusion.md`](../specs/fusion.md) and in each pinning test.

| ID | What it fixed | Owned by |
|---|---|---|
| `[REPAIR-PURGE-QUARANTINE]` | Three functions that existed only to `panic!`, deleted whole | `grep -rn QUARANTINED crates/` returns nothing |
| `[REPAIR-COSINE-MERGE]` (#351) | A cosine was discarded for any pair already in the candidate map, so discovery order decided whether a duplicate was visible | `issue_93_embedding_uniqueness.rs`, `embedding_route_invariance.rs` |
| `[REPAIR-CLUSTER-SIGNAL-TRUTH]` | Cluster signals were averaged over the closure's discovery edges | `[FUSION-CLUSTER-SIGNALS]` |
| `[REPAIR-SNAPSHOT-PATH-ORDER]` (#301) | Snapshot order followed `FileId`, so edit history changed the report | `deslop-lsp/tests/history_determinism.rs` |
| `[REPAIR-WATCH-EXCLUSION]` | The watcher was built with `ExclusionConfig::empty()`, so live and batch disagreed | `deslop-lsp/tests/dependency_reactivity.rs` |
| `[REPAIR-VECTOR-FINITE]` | Non-finite vector components failed *open* through every cosine floor | `embedding_non_finite.rs` |
| `[REPAIR-ADMISSION-PIN]` (#343) | Admission arithmetic was unpinned — only rendered confidence was | `pair_admission_bounded_max.rs` |
| `[REPAIR-DECLARATION-FAMILY]` | The sibling-boilerplate filter could not tell scaffolding from real duplication in **either** configuration | `dart_issue_197_single_file_structural_only.rs` + `declaration_family_plurality.rs` + `declaration_family_mixed_component.rs` + `refactor_merge` + `issue_190_data_table_demote.rs`, required together |
| `[REPAIR-PY-DICT-ASSERT-DEPTH]` | The pytest dict-assert idiom was recognised at one AST depth only, so the module-wide view survived subsumption | `python_issue_107_chained_dict_assert.rs` |
| `[REPAIR-DOC-TRUTH]` (#345) | Public docs still taught the deleted sum-and-clamp fusion | `[FUSION-STRATEGY-BOUNDED-MAX]` |
| `[REPAIR-RENAME-ANCHOR-MASS]` (#405) | A four-literal cliff zeroed rename evidence below it, rendering a maximal one-literal Type-2 clone at `fused = 0.0588`; replaced by Baker corroborated anchor mass (`[TECH-PMATCH-BAKER]`), elected over the substituted pairs alone so a homonym byte-string cannot make one role veto the other | `type2_rename_anchor_floor.rs`, the `rename_lean` scenarios in `fused_golden_bands.rs`, `js_language_features.rs`, `js_ts_clone_buckets.rs` |
| `[QUARANTINE-EMBED-REFRESH-COMPLETE]` (#370) | 🛑 **Live quarantine — code replaced by `panic!`, not repaired.** `live::api::commit_background_refresh` swapped a refreshed report over the last good one and called `job.report_complete()` without ever reading `report.embedding_provenance` — it logged that provenance in the same block that declared success. A refresh in which the provider rejected *every* subtree (measured: `indexed 0 / attempted 851 / failed 851`) was committed and announced `phase = "complete", done = 851`. Every clone needing the semantic axis silently vanishes from a report claiming that axis ran. The one-shot CLI is not implicated — `run_embedding_pass` records the truth and `ollama_failures.rs` holds it | `deslop-lsp/tests/embedding_failure_progress.rs` (red, unignored) |
| `[REPAIR-CONTENT-FRONTIER]` | Collapsed *non-leaf* nodes were emitted alongside the collapsed descendants they span, so an interpolated string re-tested the same bytes as a whole-node literal and manufactured an unpreserved literal at every interpolation a rename touched | `js_language_features.rs` (template literals), `fused_golden_invariants.rs` |

The branch regression audit (RA-01…RA-09, REG-01…REG-11) is fully discharged: RA-06/#393 closed, RA-07/#394
and RA-08/#395 carried above, everything else fixed and pinned. The six once-skipped VSIX tests are restored
with zero skips left.
