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
| All three agent bands are reachable and mean the same thing in six languages — **but the rename band is only pinned above the literal-anchor floor; see section 0** | `fused_golden_bands.rs` — verbatim / maximal rename / shape-only, with band separation and rank order per language |
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

## 0. 🛑 QUARANTINED — a maximal Type-2 rename was reported as coincidence

`content::pair_rename_consistency` is a `panic!`. Every scan that reaches a shape-saturating cluster whose
member pair carries fewer than four literal anchors now aborts with exit 101, by design: a false negative on
the textbook Type-2 clone is worse than a crash. **Nothing else may be built on this crate until the
replacement lands.**

`crates/deslop/tests/type2_rename_anchor_floor.rs::a_maximal_rename_with_few_literals_is_still_a_type2_clone`
was watched failing on the rendered verdict **before** the quarantine replaced the code. Two TypeScript
files, identical logic, every identifier renamed, one literal (`0`):

```
id=f461c761183864b0 bucket=structural_only category=logic size=2 weight=0.388
structural=1.0000 token_jaccard=1.0000 embedding_cos=0.0000 fused=0.0588
files={"charge.ts", "invoice.ts"}
```

`fused = 0.0588` is inside the `< 0.6` band in which `CLAUDE.md` instructs an agent to **write the copy
anyway**. This is a false negative on the clone class `fused_golden_bands.rs` calls "the load-bearing one …
every clone detector must report it".

**Mechanism**, and it is deliberate code, not a slip:
[`content::pair_rename_consistency`](../../crates/deslop-core/src/content.rs) returns `0.0` outright when a
member pair carries fewer than `RENAME_EVIDENCE_MIN_LITERALS = 4` literal positions. `content_gated_signals`
then scores the cluster `max(agreement, 0.9 × 0.0)`, and a maximal rename agrees on almost no raw identifier
bytes. The floor exists for a real reason — without anchors, a consistent identifier mapping cannot separate
a Type-2 rename from sibling scaffolding that also substitutes names consistently — so the fix is a
discriminator that does not depend on literal mass, not a lowered constant.

Why no suite caught it: every `fused-golden-<lang>` rename fixture keeps **identical literals** on both
sides, so the band is only ever exercised above the floor. #341 then softened 17 rename-showcase fixtures
from maximal to partial renames, which moved the shipped fixtures above the floor as well.

**Blast radius, measured.** `cargo test --workspace --all-targets --features deslop-core/live -- --skip
ollama_ --skip corpus_` is fail-fast and now stops at `deslop --test boilerplate`
(`import_boilerplate_is_suppressed_but_real_clones_still_report`,
`import_boilerplate_report_mode_emits_low_noise_hints`), exit 101, both on the quarantine panic. The same
command was **exit 0 across 170 binaries** immediately before the quarantine, so every casualty from here on
is this defect being made visible, not a new one. `cargo clippy --release --all-targets --workspace` is
clean; the `#[allow(clippy::panic)]` on the quarantined function is the sanctioned exception and must be
deleted with the panic.

- [ ] Fix the discriminator, not the constant, and keep `dart_issue_197_single_file_structural_only`,
      `declaration_family_plurality` and `declaration_family_mixed_component` green — those are the
      sibling-scaffolding side the floor was protecting.
- [ ] Extend `fused_golden_bands.rs` with a below-floor rename scenario per language so the band is pinned
      across the anchor count, not only above it.

## 1. Eight assertions are `#[ignore]`d — every one is a live accuracy defect

Nothing is deleted or weakened: each carries an `#[ignore = "…"]` naming its issue and runs under
`cargo test … -- --ignored`. This is the top of the list because **every embeddings-on assertion in the
workspace is currently switched off**, which is why gap 3 above cannot be measured, let alone closed.
Measured 18 Aug: `cargo test --workspace --all-targets --features deslop-core/live -- --skip ollama_
--skip corpus_` exits 0 across 170 test binaries with exactly these 8 ignored — the tree is otherwise
green, section 0 excepted.

- [ ] **[#369](https://github.com/Nimblesite/Deslop/issues/369)** — three ignores.
      `issue_343_sum_clamp_saturation::mid_band_cluster_confidence_never_exceeds_its_strongest_axis` renders
      two embedding-only false positives and hides the real clone; both false pairs carry `structural = 0`
      and `token_jaccard = 0` and survive on `MockOllama`'s length-residue cosine alone.
      `pair_size_coherence::an_embedding_only_pair_does_not_join_occurrences_of_different_size` and
      `lsp_embedding_determinism::lsp_embedding_refresh_is_bounded_and_reproducible` fail on the same
      mechanism. The known fix has an O(N²·D) blowup — that is the part to solve.
- [ ] **[#370](https://github.com/Nimblesite/Deslop/issues/370)** — `embedding_failure_progress` hangs
      indefinitely (14m41s locally, two whole CI Test budgets). The stall is in the unbounded
      `recv_response` read, upstream of the file's own 20s timeout: the server appears never to emit a
      terminal progress frame on the rejection path.
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
- [ ] Restore the 17 rename-showcase fixtures #341 softened from maximal to partial renames — **blocked on
      section 0**. Restoring them before the anchor floor is fixed converts 17 passing fixtures into 17
      false negatives.
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

The branch regression audit (RA-01…RA-09, REG-01…REG-11) is fully discharged: RA-06/#393 closed, RA-07/#394
and RA-08/#395 carried above, everything else fixed and pinned. The six once-skipped VSIX tests are restored
with zero skips left.
