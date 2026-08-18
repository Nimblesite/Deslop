# Incremental analysis and diff-scoped reporting

Turns `deslop` from "re-analyse everything, but skip re-parsing" into an analyser whose cost tracks the size of the change, then puts that cost profile to work where it matters: pre-merge CI that flags only the duplication a change introduces. Incremental analysis is [gh #383](https://github.com/Nimblesite/Deslop/issues/383) and the disk economics that motivate it are [gh #379](https://github.com/Nimblesite/Deslop/issues/379); the GH-action cache that carries the store between CI runs is [gh #381](https://github.com/Nimblesite/Deslop/issues/381); diff-scoped reporting is [gh #364](https://github.com/Nimblesite/Deslop/issues/364). Delivery is ordered: the store and its reuse (Phases 0–4, landed), the CI cache (Phase 5), the diff flags on top (Phases 6–10). Specified by [`pipeline.md §PIPELINE-INCREMENTAL-ANALYSIS`](../specs/pipeline.md), [`release.md §ACTION-CACHE`](../specs/release.md), [`cli.md §CLI-ARG-DIFF`](../specs/cli.md), [`pipeline.md §PIPELINE-DIFF-INGEST`](../specs/pipeline.md), [`pipeline.md §OUTPUT-SCHEMA-DIFF-TAGS`](../specs/pipeline.md), and [`pipeline.md §METRICS-DIFF-SCOPE`](../specs/pipeline.md).

**The whole migration is behaviour-preserving.** An incremental pass must produce the report a cold pass produces, field for field. No threshold moves, no bucket moves, no ranking change. Any report difference is a defect in the incremental path, never an accepted cost of it.

## Where the time actually goes

400-file / 2.9 MB Rust corpus, warm cache (400 hits / 0 misses), three consecutive runs:

| stage | run 1 | run 2 | run 3 | share |
|---|---|---|---|---|
| discovery | 16 ms | 18 ms | 18 ms | 0.5% |
| parse + normalise + fingerprint — **the only persisted stage** | 302 ms | 326 ms | 304 ms | ~9% |
| LSH + candidate pair evaluation | 2486 ms | 2618 ms | 2598 ms | **~78%** |
| cluster + rank | 23 ms | 13 ms | 13 ms | 0.5% |
| render | 7 ms | 12 ms | 7 ms | 0.3% |
| buckets + metrics | 317 ms | 314 ms | 338 ms | ~10% |
| write JSON | 69 ms | 88 ms | 84 ms | 2.5% |

Wall clock corroborates: `--no-incremental` 7.10 s, fully warm 6.39 s. The store saves 10% because it is attached to a stage worth 9%.

**Two targets, in order: the 78% and the 10%.** Everything else in the table is noise, and optimising it would be theatre.

## What licenses reuse

[PIPELINE-DETERMINISM] already guarantees that identical corpus state produces identical MinHash signatures, identical candidate sets, identical fused scores, and identical cluster ids — and explicitly that this holds over corpus *state*, not edit history. That is the whole basis for reusing downstream work: a value computed from content that has not changed is still correct.

It also bounds what may **not** be reused. The embedding/ANN layer is the one approximate stage ([FUSION-EMBED-PROVIDER]); reuse there trades recall in a way the rest of this plan does not, and is out of scope below.

## Phase 0 — break down the 2.5 seconds

**Attributed.** The existing `debug` spans around signature construction, band collision enumeration, and candidate scoring already carried the split — no new instrumentation was needed. Release build over `crates/` (92,973 fingerprints), stable across three consecutive runs:

| sub-stage | time | share of the LSH block |
|---|---|---|
| MinHash signature construction (one blake3 XOF fill per k-gram) | ~1,656–1,708 ms | **~69%** |
| band collision enumeration | ~710–773 ms | ~30% |
| candidate pair scoring (`estimate_jaccard`) | ~30 ms | ~1% |
| closure + rank | ~30 ms | — |

The "pair scoring is almost certainly not the cost" hunch is confirmed and quantified. **Signature construction dominates**, which selects the first Phase 2 design below. Banding is the secondary target: `band_key` hashes the 4 row values through blake3, and identity concatenation of the rows has identical collision semantics — recorded as a follow-up optimisation, separate from the reuse work.

**Benchmark corpus and baseline — recorded.** The benchmark corpus is the pinned tokio clone ([`corpus/tokio.json`](../../corpus/tokio.json), tag `tokio-1.49.0`, sha-verified by `scripts/fetch-corpus.mjs`) — committed as a manifest rather than vendored source, deterministic by pin, and the corpus the Makefile already names fastest and most stable. Release binary, `--embeddings off`, 758 files / 1,779 clusters:

| run | wall | peak RSS | `cache_stats` |
|---|---|---|---|
| `--no-incremental` | 5.97 s | 1,412 MB | 0 / 0 |
| cold, store on | 5.96 s | 1,411 MB | 0 hit / 758 miss |
| fully warm | 5.58 s | 1,368 MB | 758 hit / 0 miss |

The parse store costs 29 MB for this corpus. All three reports are field-for-field identical with `cache_stats` removed — verified by direct JSON comparison, corroborating [PIPELINE-DETERMINISM] and the warm/cold agreement invariant on a real corpus. The cold golden report is committed at `crates/deslop/tests/fixtures/report-golden/` and enforced byte-for-byte by `report_golden.rs`, whose second half independently re-derives the golden's occurrence slices, ranking, and metrics arithmetic from the authored fixture sources so a wrongly-blessed golden cannot self-certify.

## Phase 1 — write the contract before the code

[PIPELINE-INCREMENTAL-ANALYSIS] states what an incremental pass may reuse and what equivalence it owes. It exists now as a specification of intent; Phase 1 is where it stops being aspirational:

- An equivalence test that runs a corpus cold, then runs it again after touching one file, and asserts the two reports are identical field for field — cluster ids, occurrence byte ranges, bucket, signals, ranking order, and `metrics`. This is the test the whole plan is judged by, and it must exist and pass **before** any reuse is implemented, so that it is proven to be a real assertion rather than one that happens to hold.
- The same equivalence asserted across a file add, a file delete, a rename, and a revert-to-previous-content — the last one specifically, because content-addressed reuse makes "back to a state we have seen" a distinct code path.

Exit: equivalence tests exist, pass against today's non-incremental behaviour, and would fail if a later phase served stale downstream state.

## Phase 2 — make the dominant sub-stage incremental

Phase 0 attributed ~69% of the block to signature construction, selecting the first design. The second stays recorded against the banding ~30% but is not this phase's deliverable.

**Selected and landed — full signatures in the parse blob.** A signature is a pure function of one subtree's normalised token k-grams, so it is content-addressed by the existing blob key exactly as the tree is — and inherits [PIPELINE-INCREMENTAL-INVALIDATION] unchanged: a stale signature is unaddressable, not merely unused. Each file's signatures are built once at parse/load time (`signatures_for_file`), persisted beside the fingerprints (blob magic bumped; decode enforces signature count == fingerprint count), and attached on a validated hit — the hit path re-derives fingerprints from the cached tree and any disagreement with the stored records voids the blob and takes the miss path, self-healing on the store that follows. Reuse is observable as `signatures_built` / `signatures_reused` on the `fingerprint corpus built` event, pinned by `signature_reuse.rs`.

**Why not the cheaper formats.** Persisting only the 32 band hashes per fingerprint (256 B vs 1 KB) fails on consumption: `estimate_jaccard` consumes *full* signatures for every candidate pair that reaches scoring and for every cluster's signal means, so band hashes alone would force full-signature reconstruction for exactly the fingerprints that matter — the cost the persistence exists to elide. The in-memory band index avoids disk but dies with the process, which is the wrong shape for CLI runs and CI (#381). Full signatures grow the store (measured below) and that lands squarely in the #379 economics question, which Phase 4 answers with numbers — including whether the normalised tree still earns its share of the blob now that signatures ride along.

**Recorded alternative — if banding or collision enumeration had dominated.** Persist the band index (band → bucket → fingerprint hash) rather than the signatures. An incremental pass evicts the changed files' fingerprints from their buckets, inserts the new ones, and reads off only the collisions involving them — which is the O(k·N) rather than O(N²) win, and the one that makes cost track change size.

Whichever lands, the surviving pair set is small enough to persist outright: 5,960 pairs at ~80 B is under 500 KB, three orders of magnitude below the parse store.

Exit: a one-file change on the benchmark corpus is measurably cheaper, with the Phase 1 equivalence tests green.

## Phase 3 — buckets and metrics

~10%, and second-largest once Phase 2 lands. Not worth touching before then, and worth re-measuring after — a stage that is 10% of 3.3 s is a very different proposition at 10% of 800 ms.

## Phase 4 — decide what the parse store is for

Once cost tracks change size, revisit the parse store on its own merits rather than as the only persistence there is. It costs ~40× the source it describes on this repository ([gh #379](https://github.com/Nimblesite/Deslop/issues/379)) and buys 9%. The honest options are that it earns its disk under the new economics, that it shrinks (store fingerprints, drop the normalised tree, re-derive), or that it goes. This plan does not pre-judge which; it does insist the question is asked with numbers rather than left to inertia.

## Diff-scoped reporting — `--diff` / `--only-changed`

Implements [gh #364](https://github.com/Nimblesite/Deslop/issues/364) end to end: scope a report to the code a change actually touches, so pre-merge CI flags new duplication without tripping on legacy debt.

### Shape

```bash
git diff main...HEAD | deslop src/ --diff - --only-changed
deslop src/ --diff change.patch --only-changed
```

The scan is always the **whole tree** — cross-file clones between changed code and untouched helpers are the second half of the ask, and the warm parse store ([PIPELINE-INCREMENTAL]) makes the full scan cheap — in CI only once [ACTION-CACHE] carries it between runs (Phase 5, which lands before the diff flags). The diff scopes the *report*, never the *analysis*.

### Decisions — settled here, not during coding

**A over B.** We scope by diff line ranges (what the issue asks for), not by diffing against a persisted prior report. Baseline diffing answers "what changed since CI last ran", fails open on a cold cache or rebased base branch, and inherits an id-stability defect: the cluster id is the minimum member hash (`cluster.rs::cluster_id_source`), so editing that one member re-ids the whole cluster and a legacy cluster reports as newly introduced. Diff scoping is stateless and deterministic. `ReportDelta` stays what it is — the live-session generation delta.

**The diff is parsed, not pattern-matched.** A hand-written line-oriented parser in `deslop-core` (module `diff_scope`) consumes the unified-diff grammar: `diff --git` / `---` / `+++` file headers, rename and `Binary files` lines, `@@ -l[,n] +l[,n] @@` hunk headers, and ` `/`+`/`-`/`\` body lines. No regex anywhere — every token is recognised by exact structural prefix and integer parsing, the same class of code as the TOML config loader. `tree-sitter-diff` 0.1.0 exists but is experimental, and tree-sitter's error-recovery would turn a malformed diff into silently wrong spans; a strict parser that **rejects** anything it does not recognise is the accuracy-correct tool. Output: `path → merged, sorted new-side added-line spans`.

**Stale diffs are refused, not tolerated.** The hunk body carries the new-side content. For every hunk, every context and added line must byte-match the scanned file at the line number the hunk claims (content compared exactly as carried, `\n` terminator excluded). First mismatch → exit `2` naming the file and line. A diff that disagrees with the tree would tag the wrong occurrences, and under `--only-changed` a mis-tag is a silent false negative in a merge gate — the one outcome the accuracy rule exists to prevent.

**Tags are `Option`, never defaulted-false.** `in_diff`, `intersects_diff`, `is_newly_introduced`, `clusters_outside_diff`, and `metrics.diff` are all `Option<...>`, absent unless `--diff` was given. A run without a diff must not assert `is_newly_introduced: false` about anything — that is a claim it has no evidence for.

**`duplication_percent` never changes meaning.** `metrics` stays repo-wide and byte-identical with and without `--diff` (test invariant, same as the [METRICS-REPO-WEIGHTED] no-knob rule). The diff-scoped figure is a separate `metrics.diff` block with its own denominator: duplicated added lines over added lines in analysed files. Under `--only-changed`, `--fail-over` gates on the diff-scoped percent and the report header names which number gated.

**Out of scope.** Persisting baselines across runs (rejected above); tagging in live/LSP/MCP sessions (fields stay `None`; a later issue can thread a diff through the session config); `--from-report` + `--diff` (conflict, exit `2` — re-rendering has no tree to verify the diff against).

### Semantics

- Diff paths resolve against the invocation working directory after stripping the `a/`/`b/` prefixes, then re-relativise to the scan root — the form `ReportOccurrence.path` carries. Diff files outside the scan root or absent from the corpus are ignored for tagging and counted on the `diff ingested` tracing event; a repo-root diff legitimately touches files the scan never sees.
- Only **new-side added lines** scope the report (`+` lines; context and deletions do not). A pure rename with no content change adds no lines and tags nothing. Binary hunks tag nothing.
- Intersection is closed-interval on 1-indexed lines — occurrences already carry `start_line`/`end_line` in exactly that form (`report_metrics.rs::byte_range_to_line_range`). One added line inside a 40-line occurrence tags it: touching a clone counts as touching the clone.
- Cluster rollups ignore `hidden` occurrences, matching [METRICS-REPO]'s projection: `intersects_diff` = any non-hidden occurrence in diff; `is_newly_introduced` = all non-hidden occurrences in diff.
- `--only-changed` drops clusters where `intersects_diff != true` from `clusters` before ranking output, counts them in `clusters_outside_diff`, and leaves `metrics` untouched.

Every phase below is test-first: the E2E tests are written against fixture repos with committed `.patch` files, watched red, then the code lands. Fixtures live beside the existing incremental fixtures; each scenario asserts exact cluster ids, occurrence paths and line ranges, tag values, counts, and exit codes.

## Phase 5 — the store survives CI runs ([ACTION-CACHE], gh #381)
The action restores `<scan-root>/.deslop/cache` before the run step and saves it after, keyed and bounded per [`release.md §ACTION-CACHE`](../specs/release.md); a `cache: "false"` input opts out. Both of #381's former blockers are landed: retention bounds every save at 2 GiB ([PIPELINE-INCREMENTAL-RETENTION]), and every restored blob is digest-verified against its full address or refused into a plain miss ([PIPELINE-INCREMENTAL-INTEGRITY]) — a stale or poisoned restore degrades to re-parsing, never to a wrong report.
Exit: `action-selftest.yml` gains a two-pass job — the second pass restores the first's store, logs non-zero cache hits, and renders a report byte-identical to the first modulo `cache_stats`.

## Phase 6 — diff ingest ([PIPELINE-DIFF-INGEST])
`diff_scope` module: parser, path resolution, span merge, tree-verification refusal. `--diff <path|->` accepted and validated; tags not yet emitted.
Exit: unit suite over the grammar (renames, quoted paths, CRLF content, `\ No newline`, binary, malformed input rejected); E2E: stale diff refused with exit `2`; matching diff accepted.

## Phase 7 — tagging ([OUTPUT-SCHEMA-DIFF-TAGS])
Wire fields added in `live-ipc.td` (regenerated, never hand-written); intersection pass stamps occurrences and clusters at render time.
Exit: E2E over the four populations — new duplicate wholly in diff (`is_newly_introduced: true`), changed code cloning an untouched helper (`intersects_diff: true`, `is_newly_introduced: false`, the untouched occurrence `in_diff: false`), legacy cluster (`intersects_diff: false`), and a no-`--diff` run whose JSON carries none of the fields.

## Phase 8 — filtering and the gate ([METRICS-DIFF-SCOPE])
`--only-changed` (usage error without `--diff`), `clusters_outside_diff`, `metrics.diff`, gate rerouting under `--only-changed`.
Exit: E2E: legacy-heavy fixture passes the gate under `--only-changed` with an empty diff and fails it when the diff introduces a clone; `metrics` byte-identical across `--diff` on/off; threshold summary names the diff scope.

## Phase 9 — renderers
Text delta summary (newly-introduced count, cross-file count), occurrence badges (`[in diff]` / `[existing]`) through the one shared occurrence renderer, HTML CSS-only "only diff-affected" toggle. JSON stays canonical; both views derived.
Exit: rendered `.txt`/`.html` assertions in the same E2E fixtures.

## Phase 10 — action surface
`diff:` and `only-changed:` inputs on `action.yml`, forwarded as `--diff` / `--only-changed`; when the diff-scoped percentage gated, the [ACTION-GATE] message names it ([METRICS-DIFF-SCOPE]).
Exit: shape assertions in `test-action-contract.mjs`; a self-test leg where a legacy-heavy fixture passes the gate under `only-changed`.

## Non-goals

- **Embedding/ANN reuse.** Approximate stage, different risk profile, bounded separately by [FUSION-EMBED-PROVIDER]. The embedding cache already handles the expensive part (inference).
- **Any accuracy change.** If a phase makes a report better, that is a bug in the phase — the change belongs in its own test-first work stream with its own corpus measurement.

## Spec IDs

| ID | Section | Status |
|---|---|---|
| [PIPELINE-INCREMENTAL] | The persisted parse store and its content addressing | ✅ implemented |
| [PIPELINE-INCREMENTAL-INTEGRITY] | Blob binding digest, bounded decode, size-bounded reads | ✅ implemented, pinned by `cache_blob_integrity.rs` + `fpcache/tests.rs` |
| [PIPELINE-INCREMENTAL-RETENTION] | Store pruning: stale-version partitions, orphan policy, 2 GiB budget | ✅ implemented, pinned by `cache_retention.rs` + `fpcache/retention/tests.rs` |
| [PIPELINE-INCREMENTAL-ANALYSIS] | What an incremental pass may reuse, and the equivalence it owes | ⏳ signature reuse implemented and pinned; downstream stages open |
| [CONFIG-INCREMENTAL-OPTOUT] | `[analysis] incremental = false` escape hatch | ✅ implemented, pinned by `signature_reuse.rs` |
| [PIPELINE-DETERMINISM] | The property every reuse rests on | ✅ implemented |
| [ACTION-CACHE] | The store restored and saved around the action's run step | ⏳ specified ([release.md](../specs/release.md)), not shipped |
| [CLI-ARG-DIFF] + [CLI-ARG-ONLY-CHANGED] | The two flags and their conflicts | ⏳ specified ([cli.md](../specs/cli.md)), not shipped |
| [PIPELINE-DIFF-INGEST] | Strict unified-diff parser and tree verification | ⏳ specified, not shipped |
| [OUTPUT-SCHEMA-DIFF-TAGS] | The five `Option` wire fields | ⏳ specified, not shipped |
| [METRICS-DIFF-SCOPE] | `metrics.diff` and the `--only-changed` gate | ⏳ specified, not shipped |

## Checklist

The live TODO for this plan. Every work session updates this list in the same change as the work it records.

### Phase 0 — attribution and baseline
- [x] Attribute the LSH block: signature construction ~69%, band enumeration ~30%, pair scoring ~1% (release, `crates/`, 92,973 fingerprints)
- [x] Record the attribution and the selected Phase 2 design in this plan
- [x] Commit a benchmark corpus that later phases measure against — the pinned tokio manifest (`corpus/tokio.json`, sha-verified clone)
- [x] Record the cold and warm baseline for that corpus in this plan — 5.97 s / 5.96 s / 5.58 s, reports identical modulo `cache_stats`
- [x] Commit cold golden reports that every later phase must reproduce byte-identically — `report_golden.rs` + `tests/fixtures/report-golden/` (byte-equality half plus an independent contract half derived from the authored sources)
- [x] Extend the golden to a mixed-language corpus — `incremental_multilang_golden.rs` + `tests/fixtures/incremental-multilang/` (Rust, Python, TypeScript, Dart, C#, Go; one authored Type-1 pair each, twelve byte-distinct files sharing one store). `expected-report.json` blessed and reviewed: exactly six `identical` clusters, one per language, weights ranked 52→35. Scanned at `--min-nodes 20` — below 14 the C# pair renders a second signature-line cluster that straddles [PIPELINE-CLUSTER-SUBSUME] containment by 7 bytes (gh #389, filed as its own edge)

### Phase 1 — equivalence contract ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE])
- [x] E2E test: cold run vs warm run — reports identical field for field, `cache_stats` the sole difference (`incremental_equivalence.rs::cold_and_warm_cached_runs_match_the_uncached_cold_report`)
- [x] E2E test: one-file edit — warm report equals a cold run of the edited tree (`editing_one_file_matches_the_cold_report_of_the_post_edit_tree`)
- [x] E2E tests: file add, file delete, rename, revert-to-previous-content (four scenarios in `incremental_equivalence.rs`, each with exact `cache_stats` and cluster-shape assertions)
- [x] Per-language invalidation matrix — `incremental_multilang_matrix.rs`: touch one language (exactly 1 miss / 11 hits, all six clusters unmoved), delete one language (that cluster gone, other five field-for-field identical), revert (content-addressed full-hit restore), a six-step cumulative edit chain, and byte-identical `.ts`/`.js` twins proving the store key's language component
- [x] All equivalence tests green against today's behaviour before any reuse lands — verified 6/6 green; the reuse pin `signature_reuse.rs` is in the tree born-red against the missing `signatures_built`/`signatures_reused` event fields

### Phase 2 — signature persistence
- [x] Decide the persistence format against #379 — full signatures in the parse blob; band hashes rejected because `estimate_jaccard` consumes full signatures for scoring and cluster means (rationale recorded above)
- [x] Blob format bump: signatures persisted beside fingerprints, positionally 1:1; decode rejects a count mismatch; pre-signature magic decodes as a plain miss (unit-pinned in `fpcache.rs`)
- [x] LSH consumes persisted signatures for unchanged files and constructs only for changed files — hit path validates re-derived fingerprints against stored records before attaching (`corpus/tests.rs` pins the reject-and-self-heal path); `signature_reuse.rs` green (4/4, including the store-disabled accounting contract and the `[analysis] incremental = false` config escape hatch)
- [x] Blob trust hardened ([PIPELINE-INCREMENTAL-INTEGRITY]): binding digest over the full address verified before decode, size-bounded reads, global decoded-node budget — findings 1, 2 and 4 of the [regression audit](../incremental-persistence-regression-audit.md), pinned by `cache_blob_integrity.rs` + `fpcache/tests.rs`
- [x] One-file change on the benchmark corpus measurably cheaper; Phase 1 equivalence tests green. Release, pinned tokio, `--embeddings off`, binding-digest format: `--no-incremental` 5.88 s / 1,649 MB; cold store-on 6.22 s / 1,665 MB; fully warm 2.91 s / 1,609 MB; **one-file edit (757 hit / 1 miss) 2.92 s** — 2.0× cheaper than the store-off pass; revert restores a full-hit 2.94 s pass. All six states render byte-equal reports modulo `cache_stats`. The edit pass is *not* cheaper than fully-warm because everything downstream of signatures still runs corpus-wide — exactly the remaining phases' target
- [x] Follow-up recorded: `band_key` identity concatenation instead of blake3 (Phase 0 attribution section)

### Phase 3 — re-measure
- [x] Re-run attribution after Phase 2 (release, warm tokio pass, debug spans): discovery 7 ms (~0.2%) · parse-store load (decode + digest verify + fingerprint re-derivation, `signatures_built=0`) ~663 ms (~23%) · **LSH band enumeration ~1,276 ms (~44%) — now the dominant stage** · candidate scoring ~54 ms (~2%) · closure + rank + content ~82 ms (~3%) · buckets + metrics + JSON write ~0.7 s (~25%). Decision with numbers: signature construction is eliminated from the warm path, so the next targets in order are **banding (~44%)** — the already-recorded `band_key` follow-up and/or a persisted band index — then **buckets+metrics (~25%)**, then store-load decode (~23%). Buckets+metrics is now worth touching, but only after banding

### Phase 4 — parse-store economics
- [x] Re-run the #379 disk numbers under the new economics. Store: **185.8 MiB / 759 blobs** for a 7.3 MiB source tree (~25×; signatures are ~85% of blob bytes; +32 bytes/blob for the binding digest is noise). Verdict: **keep** — the disk buys a halved warm wall and the store is the substrate the remaining phases build on. **Shrink path recorded**: if the banding phase persists the band index, the per-fingerprint signatures (~85% of the store) stop earning their bytes and the blob drops back to roughly the pre-signature 29 MB shape
- [x] Retention landed ([PIPELINE-INCREMENTAL-RETENTION]): nothing deleted under the 2 GiB budget, class-before-age eviction over it (other-version → orphan → live) — policy and rationale in the audit's retention section, pinned by `cache_retention.rs` + `fpcache/retention/tests.rs` 7/7
- [x] Warm-RSS regression removed (audit finding 5): the per-render flatten replaced by one session-owned flat store (`session/store.rs`). Re-measured (release, pinned tokio, `--embeddings off`): `--no-incremental` 6.45 s / 1,532 MB · **fully warm 3.31 s / 1,495 MB** · one-file edit 3.39 s · revert full-hits; all states byte-equal modulo `cache_stats`

### Follow-ups
- [x] Persisted-signature recall pinned on the LSH-only route (audit finding 6) — `lsh_only_nearmiss_recall.rs`: cold, warm, mixed, and revert each assert the exact signal triple, bucket, files, metrics, and an exact mixed-pass rebuild/reuse split
- [x] `tool_version` bound into the binding digest (audit finding 2); cross-partition relocation refused on both the decode path and the digest-distinctness axis test
- [x] Hostile-size gaps closed (audit finding 4): one-handle bounded read taken past the ceiling, fallible reserves, 4 M decoded-node budget claimed per node including its child slots
- [x] Retention safe across concurrently running tool versions: `OtherVersion` partitions never deleted under budget, evicted first under pressure — pinned at unit and E2E level
- [x] Stale `RED PIN` prose replaced with the row's recall + precision contract, and `LSH_ONLY_NEARMISS_MIN_JACCARD` **is** `pair::LSH_ONLY_MIN_JACCARD` rather than a copy of its value — one number, two named uses
- [x] Full `make ci` gate run on the final snapshot; the two accuracy defects it caught (#331 row-4 false positive, #108 unproven anchor-free promotion) are fixed at the root, not suppressed — see the audit's "Accuracy defects this audit uncovered"

### Phases 5–10 — CI cache and diff scoping
- [x] Decisions recorded; specs updated in the same change ([ACTION-CACHE], [CLI-ARG-DIFF], [CLI-ARG-ONLY-CHANGED], [PIPELINE-DIFF-INGEST], [OUTPUT-SCHEMA-DIFF-TAGS], [METRICS-DIFF-SCOPE])
- [ ] Phase 5 — action cache restore/save + two-pass self-test, closes gh #381
- [ ] Phase 6 — parser + refusal, unit + E2E red→green
- [ ] Phase 7 — wire fields + tagging E2E
- [ ] Phase 8 — `--only-changed`, `diff_metrics`, gate
- [ ] Phase 9 — text summary, badges, HTML toggle
- [ ] Phase 10 — `diff:` / `only-changed:` action inputs
- [ ] Close #364 with a worked `git diff | deslop` example in the issue
