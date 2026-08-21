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

**Verdict: no hard blockers. Both are closed by fixing the defect, not by tracking it — #408 is
fixed in the engine and all five `type3_enclosing_method` languages pass, and the duplication gate
passes on an honestly measured ratchet. No test is skipped, ignored, or weakened.**

Neither blocker was closed by weakening anything. The gate moved **down** from main's 14.5 to a
measured 12.9, not up. The five Type-3 cases keep every assertion they had and now go green on real
recall: `structural` is measured subtree overlap rather than Merkle equality
([FUSION-SHARED-SUBTREE]), so the whole-method Type-3 near-miss the pipeline always had the evidence
for is finally admitted and rendered in **all five languages** — `dart`, `go`, `python`,
`ts-type3-stmt` and `csharp`. #408 goes from 1 of 5 to 5 of 5.

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
| `type3_enclosing_method.rs` (#408 residue) | **green — 5 of 5 languages** |

The three Python suppression contracts went green with the `verbatim_dominated` repair: one
token-identical family — equal normalised-subtree digest *and* equal collapsed-leaf keys — must now hold a
strict majority before it can certify a cluster as verbatim. Previously it certified non-verbatim members
as verbatim and forced `agreement` to 1.0.

The Type-3 residue is analysed in Part 2, and is now **fixed**: at `f92300e` *no* language reports the
enclosing method pair; at head all five do. This range took #408 from 0 of 5 to 5 of 5.

## The duplication gate — closed

Measured on this tree with the binary this tree builds, 2026-08-20, and again against `origin/main`
with that *same* binary so the comparison isolates the code from the detector:

| tree | duplication | duplicated LOC | analysed LOC | clusters |
|---|---|---|---|---|
| `main` @ `8fb1b15c9` | 14.6239% | 16,340 | 111,735 | 1,146 |
| this branch (HEAD) | **12.8257%** | 15,033 | 117,210 | 1,057 |

The branch is **1.80 points below main** under the same detector while analysing 5,475 *more* lines:
it removes 1,307 redundant LOC and 89 whole clusters. `make dup-gate` exits **0**.

The ceiling had been left at a bare, unmeasured **9.9%** — a figure no tree in this comparison has
ever scored — and the `.deslop.toml` ratchet ledger, which is the gate's entire audit trail, had been
deleted along with it. Both are restored. The ceiling is now **12.9%**, pinned just above the
measured value: a **ratchet down** of 1.6 points from main's 14.5, taken from a number main itself no
longer holds (main scores 14.6239 against its own 14.5 pin under the current detector).

The remaining distance is a flat tail of pre-existing coarse-E2E scaffolding clusters — CLI
invocation blocks, the VSIX unit suites, `code_action`/`code_action_refusal` — none introduced here.
Tracked in gh #397. Where that duplication lives, measured over the head report's 1,057 clusters:

| where | removable? |
|---|---|
| inline fixture literals in test files (`CSHARP_ALPHA`/`CSHARP_BETA` in `tests/boilerplate.rs`, the generated-DTO pairs in `tests/defaults.rs`) | **no** — they exist *because* they are duplicates. `.deslop.toml` excludes `**/tests/fixtures/**`, but a fixture written as a `const … &str` has no path to exclude |
| test scaffolding and test code | yes — the bulk of the mass |
| production `src/` | yes |

Driving the figure lower is reachable without touching a fixture; it is not reachable *quickly*. The
distribution is a flat tail of several hundred clusters averaging about eleven redundant lines each,
so each further point means hoisting shared scaffolding across several hundred test files, each
change carrying its own risk of weakening an assertion. That is why the gate ratchets rather than
jumps.

The branch has been paying this down rather than moving the number: the largest DRY-able cluster in the
repository was the pair of near-identical GH #119 role-gate suites, whose contract now lives once in
`tests/common/role_gate.rs` — which also strengthened both suites, since the Dart and Python same-role
tests inherited the embedding-support assertion they previously lacked.

**No threshold was ever raised to hide a regression.** The 12.5 → 13.65 → 14.5 history tracked a
shift in what the engine counts — row 4 of [CLONE-BUCKETS-ROUTING] going multi-language made the
measurement *more* honest, not the code worse — and 14.5 → 12.9 is real removal. Like-for-like on one
binary this branch removed 1,307 redundant LOC relative to main. Every move is justified in writing
in `.deslop.toml`; that ledger is the audit trail and must not be deleted again.

## Ignored tests — eight down to three

**No new `#[ignore]` was introduced.** All six JavaScript/TypeScript `.skip(...)` calls are gone (0
remain). Two Rust ignores were removed by making the tests genuinely pass:
`python_issue_119_embedding_role_mismatch` (needed a real fix — see below) and `pair_size_coherence`
(needed nothing but running).

The three that remain carry the same `#[ignore]` attributes verbatim at `f92300e`, so they are
unchanged pre-existing defects, not regressions in this range:

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

## #408 — fixed: `structural` was throwing away the evidence

#408 was filed as a five-language Type-3 recall hole and tracked here as a subsumption problem. It was
**two** defects, one at each end of the pipeline, and both are fixed. All five fixtures now report the
enclosing method pair; `type3_enclosing_method.rs` is green with nothing ignored.

**Defect 1 — admission threw away structural evidence it had already computed.** `pair.rs` documented
`structural_sim` as "the best-achievable subtree overlap", but `candidates::add_lsh_pairs` wrote a
literal `0.0` for every cross-bucket pair. A single inserted statement rehashes every ancestor Merkle
node, so the enclosing method scored `structural = 0.0` while the unchanged statements inside it
stayed Merkle-identical — which is precisely why the fragment views survived and the method did not.
The exact whole-method token Jaccard is 0.74–0.85, below `FUSED_THRESHOLD` 0.85, so token evidence
alone could never have rescued it:

| fixture | method nodes | exact Jaccard | measured overlap | admitted before | now |
|---|---|---|---|---|---|
| `csharp-type3` | 58 / 52 | 0.8519 | 0.898 | yes | yes |
| `dart-type3` | 56 / 49 | 0.8431 | 0.877 | no | **yes** |
| `ts-type3-stmt` | 48 / 42 | 0.8067 | 0.875 | no | **yes** |
| `go-type3` | 53 / 48 | 0.7755 | 0.906 | no | **yes** |
| `python-type3` | 37 / 31 | 0.7429 | 0.842 | no | **yes** |

`structural` is now measured ordered subtree overlap — `1 - TED / max(nodes)` by Zhang–Shasha over
normalised kinds (`overlap.rs`), short-circuiting to `1.0` on Merkle equality so every previously-1.0
cluster is unchanged. An **alignment**, never a bag of matching subtree hashes: the discriminating
information is the order and nesting of the matches, which a multiset discards — two unrelated
functions built from the same statement vocabulary carry the same hashes as a real copy. The greedy
multiset bound was measured first and scored the genuine pairs 0.52–0.64, indistinguishable from
noise; it survives only as the large-tree fallback, where it is a conservative lower bound.

Admission is a compound gate over two *independently measured* axes — overlap ≥ 0.75 **and**
`token_jaccard` ≥ 0.65 **and** both endpoints ≥ 30 nodes — never sum fusion, and the rendered
confidence stays the bounded max. Overlap is measured only on pairs that would otherwise be dropped
yet carry the token corroboration, so the cost stays away from the ~596K-candidate admission set
[FUSION-CONTENT-GATE] deliberately avoids. Routing gains [CLONE-BUCKETS-ROUTING] row 4b on the same
two floors, so the pipeline can never admit a pair the renderer then hides; row 4's old
`structural ≤ 0.01` leg is retired, since extra shape evidence must not *hide* a cluster the token
axis already carries.

**Defect 2 — subsumption then deleted the pair the fix had just admitted.** With the method finally
admitted, `ts-type3-stmt` still rendered **nothing**. `evaluate_pair` nominated the enclosing view in
only one direction — `outer`/`inner` are weight-ordered scan positions, not nesting — so when the
enclosing view was also the heavier one it fell through to `structural_precision`, which compares
signal grades. A byte-identical 28-byte parameter list scored `structural = 1.00` against the
method's 0.88 and deleted it, emptying the report. That comparison is not meaningful across scopes: a
nested window scores higher exactly to the extent that it excludes what differs. Enclosure is now
nominated in both directions, and within one credibility tier the enclosing view wins outright with
no grade comparison — which is what [PIPELINE-CLUSTER-SUBSUME] always said, and the same shape as the
two within-tier comparisons this code's history already removed for shattering method pairs into
fragments.

**Defect 3 — a view overturned after absorbing others orphaned them.** Found by the regression sweep,
not by the fixtures: `javascript-type3` reported a byte-equal loop body in place of the near-identical
*function* enclosing it. A view absorbs its nested rivals as the scan walks past them and only later
meets the rival that overturns it; those absorbed views were dying with their absorber, so nothing
reported their bytes. This module's own history already recorded the hazard — "orphaned that window's
other absorbed views, `issue_343_sum_clamp_saturation` counted the orphan" — as a reason two earlier
comparisons were removed rather than as a defect to fix. Honest `structural` made it routine rather
than rare, because a whole-file view is now admitted and sits above the method-level view in weight
order. Absorbed views are now released and re-judged against whatever survived. With this fixed,
`javascript-type3` and `typescript-type3` return to exactly their pre-change output: `nearly_identical`
over lines 2–9 of both files.

**Defect 4 — enclosure must not delete a byte-proven view that *is* the duplication.** The
`incremental-multilang` C# pair is a class containing one byte-identical method plus members that
differ. Electing the class relabelled a byte-proven Type-1 clone as a Type-3 near-miss and counted the
non-duplicated scaffolding as duplicated. A nested view that is verbatim-proven across files and
covers at least two thirds of its encloser's bytes now wins. Measured shares: 0.82 (C# class,
nested wins), 0.49 (`javascript-type3` function, encloser wins), 0.10 (`ts-type3-stmt` parameter list,
encloser wins). Byte span rather than node count, because node mass compresses those to 0.76 against
0.63 where no threshold separates them safely.

**Defect 5 — shape corroborated by the model, not by tokens, was hidden.** Found by the regression
sweep. A Dart pair measuring `structural = 0.912` **and** `embedding_cos = 0.911` — two independent
signals agreeing that the two functions accumulate identically — rendered *nothing*: row 4b accepted
only token corroboration, its `token_jaccard` was 0.555, and the cluster fell to `loosely_similar`,
which the renderer hides. The `while`/`for` accumulator pair is the shape of it: identical statements,
different loop keyword, so the k-gram set diverges far more than either the shape or the meaning
does. Row 4b now accepts corroboration from **either** independent axis. The requirement was always
"an axis that does not read the normalised tree"; naming the token axis specifically was arbitrary.

**Defect 6 — the role gate was keyed on a bucket label, and the new route bypassed it.** Immediately
exposed by defect 5's fix, and the more serious of the pair:
`python_role_mismatch_pair_must_reach_the_role_gate` caught a role-incompatible pair — the
[CLONE-NOISE-EMBEDDING-ROLE-MISMATCH] class, a reader against a writer the model scores alike —
walking straight through the guard written to catch it. The gate fired only for `same_behavior`,
because that had been the only route embedding evidence could take into an act-now bucket. Row 4b
opened a second door. The gate now keys on the *evidence* — shape short of Merkle equality, tokens
below the corroboration floor, embedding at or above the support floor — rather than on the label,
so it covers both routes. A false positive introduced and removed inside the same change; the
suppression suite is what made it a five-minute defect rather than a shipped one.

### Four contracts were inverted, deliberately

Making `structural` honest put it in direct conflict with tests that had encoded the *literal zero* as
ground truth. Each is listed because each is a real change of contract, not a stale assertion tidied
away, and every replacement is harder to satisfy than what it replaced.

| Test | Asserted | Measured reality |
|---|---|---|
| `detection::detects_type3_clone_in_csharp_fixture` | the raw JSON contains `"structural": 0.0` | the two methods share ~90% of their AST |
| `detection::assert_partial_near_miss` (go) | the reported range must **exclude** the divergent statement | that is the fragment view #408 is filed against |
| `issue_343::without_embeddings_the_mid_band_pair_stays_hidden` | a rename plus one redundant paren over a ninety-term expression must be **invisible** | `structural = 0.997`, `token_jaccard = 1.000` |
| `js_ts::typescript_signature_anchored_near_miss_is_conservatively_suppressed` | the `ts-type3-stmt` pair must **not** surface | `structural = 0.875`; #408 names this fixture as its sharpest case |

The last is the sharpest conflict in the repository: it asserted the pointBoard/scoreBoard pair must be
invisible on the *same fixture* where `type3_enclosing_method.rs` asserts it must be visible. Its
premise — the bodies "diverge", so only the shared typed signature anchors them — was an artifact of
Merkle equality: the inserted no-op line rehashed every ancestor, so bodies differing by one statement
scored zero and read as unrelated.

**No precision guarantee was dropped with it.** The #154 signature-only suppression is held on
fixtures whose bodies genuinely are unrelated by
`typescript_signature_only_match_with_divergent_bodies_is_suppressed`,
`dart_signature_only_match_with_differing_bodies_is_suppressed` and
`go_closure_signature_only_match_is_suppressed` — all three green. That test now asserts the precision
half its fixture can still prove: the unrelated `formatDuration.ts` must never be pulled in.

The replacements are two-sided where the old ones were one-sided. `structural == 1.0` becomes
"clears the admission floor **and** stays below Merkle exactness", which fails both if recall
regresses to the fragment view and if the fixture ever stops being a near-miss at all. `#343`'s real
contract — no manufactured confidence — is now asserted directly (`fused ≤ strongest axis`,
`fused < 1.0` reserved for byte proof) instead of via the cluster's absence.

A fifth, narrower case: `js-type3-guard` is a rename **plus** an inserted guard, and was calling the
pure-rename contract. Pure renames still demand Merkle exactness — `javascript_renamed_loop_clone_is_a_proven_rename`
and the TypeScript twin are unchanged and green. Only the near-miss fixture moved, to a sibling
helper that shares the verdict and not-a-copy halves verbatim.

Specified in [FUSION-SHARED-SUBTREE](../specs/fusion.md), [CLONE-BUCKETS-ROUTING] row 4b in
[taxonomy.md](../specs/taxonomy.md), and [PIPELINE-CLUSTER-SUBSUME](../specs/pipeline.md).

## Diff-aware duplication audit (#418, main `8fb1b15c9`) — three findings, all fixed here

An independent audit of the merged diff-aware gate found two fail-open paths and a broken public
output contract. All three are inherited by this branch and are fixed in it, each with a test that
fails on the unfixed code.

**Critical — an empty `+++` target erased the entire changed-code population.** `new_side_path`
returned `Some("")`, which marked the section as having *seen* its target; the verifier then
discarded the empty path as resolving outside the scan root. A truncated target header therefore
dropped every added line in the diff, so `--only-changed` measured `0 / 0 = 0%` and a repository
already breaching its ceiling passed the changed-code gate — a false negative at the exact merge gate
the feature exists to be. An empty payload is now a usage error (exit 2) naming the offending line,
matching `copy_path`, which already refused a pathless copy for the same reason. Pinned by
`diff_ingest_refusals::empty_new_side_target_is_refused_naming_the_line`, with
`dev_null_target_is_not_an_empty_target` as the positive control — `+++ /dev/null` is a *seen* target
meaning "deleted", and the obvious over-correction would turn every deletion section into a refusal.

**Critical — the Action advertised a stdin diff it cannot supply.** `action.yml` and both locale doc
pages documented `diff: "-"`, and the composite step forwarded `--diff -`. A `uses:` step has no
caller-controlled stdin, so the CLI read an empty patch — which it accepts as valid — and
`--only-changed` then passed any ceiling while omitting every cluster in the tree. The Action now
fails closed on `diff: "-"` before a CLI is downloaded, exiting `2` with the patch-file form spelled
out; the EN and ZH docs no longer advertise the form. Pinned by
`action-contract-shape-checks::the action rejects the stdin diff form before downloading a CLI`,
which also asserts the guard runs *before* the resolve step and that the docs stop advertising it.

**High — the three gate outputs were computed but never exported.** `action-read-outputs.mjs` wrote
`gate-scope`, `gate-percent` and `gate-threshold-percent`, the Action's own gate step consumed them
step-locally, and the public `outputs:` block declared only the older seven — so
`steps.<id>.outputs.gate-scope` read empty for every caller, and the hosted self-test's assertion on
it could only have failed *after* a release. The contract test missed it because its "every output"
list was hand-maintained beside the declaration and carried the same seven names. All three are now
declared, wired and documented in both locales, and **the contract check derives the list from the
helper** and fails in both directions — an output the helper emits that is undeclared, and an output
no check covers.

The audit's sixth-file 500-line finding is a pre-existing repository-standard gap, unchanged by this
branch and tracked separately; `overlap.rs` was split at 517 lines rather than added to it.

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

## Blocking the PR — none

- [x] **Duplication gate.** Measured 12.8257% against main's 14.6239% on one binary; ledger restored and
      ceiling ratcheted **down** 14.5 → 12.9. `make dup-gate` exits 0. Driving it lower is gh #397.
- [x] **The four red `type3_enclosing_method` cases.** Fixed in the engine, not tracked around:
      `structural` is now measured subtree overlap ([FUSION-SHARED-SUBTREE]) and all five languages
      pass. Every assertion intact, nothing ignored.

## Remaining — engine accuracy

- [x] **#408** — **fixed.** Shared-subtree overlap is measured at admission *and* at render,
      [CLONE-BUCKETS-ROUTING] row 4b routes "high structural overlap, moderate token overlap" on the
      same two floors that admit the pair, and cross-cluster subsumption nominates enclosure in both
      directions. All five languages green, nothing ignored. See § "#408 — fixed".
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
