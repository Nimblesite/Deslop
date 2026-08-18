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

## Regression audit — persisted processing (2026-08-17)

Audited and resolved the same day. Scope: the persisted per-fingerprint MinHash work of Phases 2–4 — blob integrity, warm/cold report equivalence, signature reuse, the workspace opt-out, and the resource cost of storing full signatures. Cross-run report diffing was out of scope.

**Verdict: merge-ready.** Every finding below is closed in code with matching tests, and the two accuracy defects the audit *uncovered* — one false negative and one false positive, both on the anchor-free routing row — are fixed at the root and pinned from both directions. The store binds every blob to the lookup address, refuses malformed, misplaced, or stale data before serving it, bounds every allocation the decode can drive, and deletes nothing it cannot prove is safe to delete. Nothing here is accepted as a known-bad. Where a bound is a deliberate ceiling rather than an eliminated risk, it is stated as such.

### Findings

| Finding | Status | Evidence |
|---|---|---|
| 1. Corrupted signature payloads served as hits | **Fixed** | Blob format v3 carries a BLAKE3 binding digest over the whole payload, verified *before* any payload byte is decoded. `fpcache::tests::a_flipped_signature_byte_fails_the_binding_digest`; E2E self-heal in `cache_blob_integrity.rs` 4/4. |
| 2. Blob not bound to its content address | **Fixed** | The digest is recomputed from the lookup's own address — language, **tool version**, `min_nodes`, source hash, magic, semantic epoch, signature width — before decode. `a_blob_is_never_served_under_a_different_address` covers all four axes including a cross-version relocation; `the_binding_digest_is_stable_per_address_and_distinct_across_addresses` proves the digest is a function of each field, not merely stable. |
| 3. Store-off accounting made `signature_reuse` red | **Fixed** | `ReuseCounters::assert_store_disabled` asserts `{hits: 0, misses: 0}`, every signature built, and skips store-on conservation. `signature_reuse.rs` **4/4**. |
| 4. Malformed lengths could panic | **Fixed, and hardened past the original ask** | Counts are proven against remaining bytes before any allocation; `MAX_AST_DEPTH` bounds one path and `MAX_DECODED_NODES` (4 M) bounds the whole tree, claimed per node *including the child slots that follow it* so an absurd child count is refused before its `Vec` is reserved; the read buffer is reserved fallibly. The file read is bounded on the read itself — one handle supplies both the length and the bytes, taken one byte past the 256 MiB ceiling so a file another binary grows mid-read is observable and refused, not silently truncated into a valid-looking prefix. |
| 5. RSS and disk regression | **Fixed** | The per-render flatten — an owned copy of every signature (~157 MiB on tokio), plus fingerprints, trees, and the source map, on *every* render — is deleted. The session owns one canonical flat store (`pipeline/session/store.rs`), spliced per change and borrowed by renders. Warm peak RSS 1,609 → **1,495 MB**. Disk is bounded by [PIPELINE-INCREMENTAL-RETENTION] (below). |
| 6. LSH-only equivalence coverage | **Fixed** | `lsh_only_nearmiss_recall.rs` carries the route-specific matrix: an embeddings-off Python pair with `structural = 0.00` that can only enter through LSH, asserted across cold, fully warm, a mixed pass, and a revert — exact bucket, signal triple, files, spread, ranking, and re-derived `duplication_percent` on every state, reports byte-equal modulo `cache_stats`, and the mixed pass pinned to an **exact** rebuild/reuse split derived from a one-file measurement rather than a hardcoded number. |
| 7. Semantic invalidation under `0.0.0-dev` | **Fixed** | `SEMANTIC_EPOCH`, `MAGIC`, and `SIGNATURE_LEN` are bound into the digest; superseded magics and trailing data are rejected. |
| 8. Architecture-dependent fallback signatures | **Fixed** | Byte offsets widened to `u64` before hashing; pinned against fixed slots. |
| Workspace opt-out | **Fixed** | `[analysis] incremental = false` gates the effective setting and store creation. `session_config().incremental` reports the *effective* mode via `PipelineSession::effective_incremental`, MCP forwards it unchanged, and `live_session_status.rs` 3/3 pins the toggle-under-opt-out transition and the store never being created. |

### Accuracy defects this audit uncovered

The LSH-only fixture finding 6 asked for did its job twice: it exposed a false negative, and fixing that exposed a false positive. Both are on [CLONE-BUCKETS-ROUTING] row 4, and both are now pinned from opposite sides so neither can be traded for the other.

**False negative — row 4 was implemented for one language (gh #390).** An authored Python pair (`structural = 0.00`, `token_jaccard = 0.9297`, endpoints past every LSH-only survival floor) rendered **zero** duplication: `classify_signals` had no row-4 arm, so the triple fell to `LooselySimilar`, which the renderer hides, while `report_render::is_csharp_lsh_type3_near_miss` patched the row for C# members only. The carve-out is dissolved into the router — row 4 now routes in every language, as the spec always said.

**False positive — row 4 reached an act-now bucket on no evidence.** Six distinct Flutter widgets measure `structural = 0.00, token_jaccard = 0.93` over whole-file spans whose `build` bodies share nothing — the framework-mandated declaration is most of each file — and once row 4 routed in every language they were reported at `fused = 0.93` as "nearly identical, review the locations" (#331). The same door admitted #108's JSON-schema pair at `token_jaccard = 0.96`.

The obvious fix — send row 4 through [FUSION-CONTENT-GATE] like every other route that rests on the normalised representation — is **wrong, and was measured to be wrong before being discarded.** Both of that gate's populations assume the members align position for position, and `structural ≤ 0.01` says the shapes differ. Against the `csharp-type3` fixture — a genuine Type-3 clone with *every* identifier renamed and one extra statement — agreement collapses to 0.19 (the literals) and rename consistency to 0.00, because the extra statement destroys the alignment the rename proof needs. Gating row 4 on content demoted that pair to `structural_only` at `fused = 0.17`: a false negative on the most valuable clone class there is, traded for #331's precision. `cli/detection.rs::detects_type3_clone_in_csharp_fixture` caught it.

`ContentEvidence::substance_varies` fails the same fixture for the same reason — it reads `true` for a consistent rename whose extra statement breaks the alignment — so it is not a narrower substitute either. Any future narrowing needs a discriminator that survives `csharp-type3`.

### Parser defect — the no-newline marker refused a legitimate `git diff` (found 2026-08-18)

`diff_scope/parser.rs` recognised `\ No newline at end of file` only in the open-hunk branch, but `feed_hunk_body` closes a hunk the moment its declared counts reach zero — and `git` emits the marker *after* that last body line. The marker therefore always arrived with no hunk open, fell through to the header parser, and was refused: **every diff touching a file without a trailing newline was rejected whole**, exit `2`, so diff-scoped reporting could not run at all on it. `no_newline_marker_does_not_count_as_a_body_line` had pinned this since the parser landed and was **red in the tree, unrun** — the workspace lint gate was failing to compile, which is what hid it.

The recognition is hoisted into `feed`, ahead of both branches, so there is one marker rule rather than one per state. Strictness is unchanged: a marker with no file section above it is still refused. Three tests pin the fix from the directions that matter — the marker between two file sections (neither section is truncated), the marker mid-hunk (it consumes no count, so no new-side line number shifts and no occurrence is mis-tagged), and the stray marker (still refused).

### Three further ingestion defects — all three fixed (found 2026-08-18, implementations landed 2026-08-19)

An adversarial review of the shipped diff-scope code found five candidate defects; two independent verification lenses per candidate confirmed three and **refuted** one, which matters as much as the confirmations. The two confirmed defects below were initially quarantined with panics; since the whole `diff_scope` module exists only on this unmerged branch — no shipped build ever carried the defects — the user directed the panics be replaced with real implementations, and both pinning tests are now green.

**Fixed — C-quoted paths are now unquoted ([PIPELINE-DIFF-INGEST], was a silent false negative).** `new_side_path` returned the target verbatim, so `git`'s quoted form — `"b/caf\303\251.rs"`, quotes and octal escapes included — became the path. It matched nothing in the corpus, and an unmatched path is *ignored* rather than refused, so every clone added in that file went untagged: dropped by `--only-changed`, missing from the `added_loc` denominator, exit 0. `git` quotes any path with non-ASCII bytes, a quote, or a backslash with stock config, so one accented or CJK filename triggers it on the documented `git diff | deslop --diff -` flow. The implementation decodes `git`'s full C-quoting table — octal byte escapes plus `\\ \" \a \b \f \n \r \t \v` — and *refuses* rather than guesses on anything else: a missing closing quote, an unknown escape, an octal value past one byte, or a decode to invalid UTF-8 is a `DiffParse` error naming the line. Green: `c_quoted_new_side_path_is_unquoted`, `c_quoted_simple_escapes_decode_to_their_bytes`, `malformed_c_quoted_paths_are_refused` (five refusal cases).

**Fixed — plain multi-file diffs keep one section per file ([PIPELINE-DIFF-INGEST], was a silent false negative).** Only a `diff ` line opened a section and `--- ` is swallowed as metadata, so a second `+++` overwrote the path while the first file's hunks stayed attached to it. Where the two files share the hunk's content — the copy-paste this tool exists to find — verification passes and the first file silently receives no added spans at all; where they differ it raises a `DiffStale` naming the wrong file. `git` and `hg` are unaffected; the triggers are `black --diff`, `ruff format --diff`, and `diff -u` loops. The fix makes the `+++` target a section delimiter: it opens a section when none is open and starts the next one when the current section already carries a path or hunks — the only line that can, in a prefix-less diff. Green: `plain_multi_file_diff_keeps_each_file_section_separate`.

**Repaired — a `+0,n` hunk header was absorbed instead of refused.** New-side lines are 1-indexed, and `verify_line` saturates `new_line - 1` at zero, so lines `0` and `1` both read the first line: the whole added span shifted one line up, the real trailing added line fell outside the scope, and its occurrences tagged `in_diff: false`. This one is *repaired* rather than quarantined because the spec-correct behaviour is precisely a refusal — there was no wrong code to replace, only a missing validation, and a panic would be strictly worse than the exit 2 the spec already mandates. `+0,0` stays legal, since that is how a deletion names an empty new side. Pinned by `zero_new_side_start_with_added_lines_is_refused`.

**Refuted — `duplicated_loc` counting clusters with one visible occurrence.** The claim was that a clone pair with one copy hidden inflates the headline percentage. It does not: [`exclusion.md`](../specs/exclusion.md) legislates this exact case — a cluster with at least one non-hidden occurrence is kept intact so the user sees "regular code duplicates generated code" — and `showstoppers.rs` deliberately builds a 1-visible/5-hidden cluster and asserts it survives. Acting on the claim would have **introduced** a false negative, reporting 0% for every hand-written-duplicates-generated case. Recorded because the near-miss is the point: the stricter reading is also incoherent, since in any ordinary cross-file pair each line is covered by exactly one occurrence.

**Resolved (2026-08-19) — the spec side won, per the user's no-false-negative directive.** `pipeline.md` said `clusters_total` "always equals `clusters.len()`", but `compute_repo_metrics` counted only clusters with at least two visible occurrences, so a mixed cluster sat in the body while the banner above it said one fewer. Excluding the cluster from the report was the false-negative direction; the accepted fix counts every cluster the body carries: `clusters_total` is now `inputs.clusters.len()` — the exact post-hide list the report renders — so the banner equals the body by construction and cannot diverge again. `duplicated_loc` is untouched (visible lines counted, hidden lines not, exactly as [METRICS-REPO] specifies). Pinned red-then-green by `showstoppers.rs::mixed_cluster_is_counted_in_clusters_total` on the 1-visible/5-hidden fixture; `metric_excludes_hidden_clusters.rs` now asserts the spec rule directly instead of re-deriving the old ≥2-visible gate, and the wire-model doc for `clusters_total` (typeDiagram source, regenerated) states the invariant.

Row 4 is routed on cluster **spread** instead, and only two shapes are demoted:

1. **A cross-file spread** (3+ members over 3+ files) — the #134 scaffolding pattern arriving through the token door instead of the structural one. This is the widget family. **This is a trade, not a free win:** a genuine clone family that wide is demoted to a hint too, exactly as `is_cross_file_scaffolding` already does for shape-identical spreads, for the same stated reason. It is recorded in the function doc, the taxonomy row, and the PR body rather than left implicit.
2. **An unmeasured cluster**, where the content pass could not compare two members at all. The anchored routes may take one on trust because their Merkle equality is itself proof; row 4 has no such signal, so unmeasured there means *nothing is known*. This is #108. `ContentEvidence` gained a `measured` flag to express it, because "measured full agreement" and "nothing was measured" were previously the same value.

Both demote to `LooselySimilar`, which the renderer hides — never `StructuralOnly`, which would claim a shape match `structural = 0.00` says does not exist. A measured *pair* is left alone even at low agreement: that is the renamed Type-3 clone.

Four suites pin the four corners, and no two can be satisfied by loosening a threshold: `lsh_only_nearmiss_recall.rs` (genuine LSH-only pair keeps `fused ≥ 0.85`), `cli/detection.rs` (the renamed C# pair still surfaces at `structural = 0.00`), `issue_331_336_shape_only_saturation.rs` (the scaffold family stays below the act-now line), and `issue_98_99_108_120_122_thresholds.rs` (unmeasured noise stays out of the ranked report entirely).

### Retention, and what a bound means here

[PIPELINE-INCREMENTAL-RETENTION] runs after every full store-on pass, the one moment the addressable blob set is exactly known. Under budget it deletes **nothing**: an orphan is the content-addressed set a revert or branch switch full-hits (the equivalence suite asserts exactly that), and a blob under another tool version may belong to a second binary sharing the workspace — an installed VSIX's LSP beside a freshly-built CLI — which two mutually-sweeping binaries would deadlock into permanent rebuild churn. Over the 2 GiB budget, eviction is by class (other-version, then orphan, then live), then oldest-first, path as tie-break. Class outranks age in both directions.

Evicting any blob is correctness-free: the next pass that addresses it misses, rebuilds from source, and self-heals. That is why the budget is a hard bound rather than an accuracy surface.

Two ceilings are deliberate ceilings, not eliminated risks, and both degrade to a plain miss:

- **256 MiB per blob** and **4 M decoded nodes** are far past anything a real source file produces. A file that somehow exceeded either would never cache — it would re-parse every pass. That is a cost, never a wrong answer.
- Both bounds sit *behind* the digest, so ordinary corruption never reaches them: it fails verification first. They exist for a payload whose digest checks out — an encoder bug, or a store an attacker can already write to, in which case they can equally edit the source the tool reads.

### Verification

- `make ci` — full workspace gate (fmt, clippy `-D warnings`, tests with coverage thresholds, build).
- Accuracy suites re-run after every fix in this round: `lsh_only_nearmiss_recall` 2/2 · `issue_331_336_shape_only_saturation` 3/3 · `issue_98_99_108_120_122_thresholds` 1/1 · `cache_retention` 3/3 · `fpcache` unit group 21/21 (14 in `fpcache::tests`, 7 in `fpcache::retention::tests`).
- Incremental suites: `signature_reuse` 4/4 · `cache_blob_integrity` 4/4 · `incremental_equivalence` 6/6 · `incremental_multilang_golden` 3/3 · `incremental_multilang_matrix` 4/4 · `live_session_status` 3/3.
- Real-repo lifecycle proof on `deslop-core/src` (135 files, release): cold 1.03 s (0 hit / 135 miss, 29,120 signatures built) → warm 0.43 s (135 hit / 0 miss, **0 built / 29,120 reused**) → edit one file 0.43 s (134 hit / 1 miss, only 165 signatures rebuilt) → revert full-hits. Cold, warm, and revert are byte-equal to the `--no-incremental` report modulo `cache_stats`; the store survives process death (136 blobs / 34 MiB).
- Pinned tokio benchmark (release, default `min-nodes`, `--embeddings off`): `--no-incremental` 6.45 s / 1,532 MB · cold 6.48 s / 1,550 MB · **warm 3.31 s / 1,495 MB** · one-file edit 3.39 s · revert 3.42 s · store 185.8 MiB / 759 blobs.

Everything downstream of signatures — band enumeration, pairing, clustering, ranking, metrics, rendering — still recomputes corpus-wide on every pass. Making that cost track the size of the change is the remaining work of [PIPELINE-INCREMENTAL-ANALYSIS] (gh #383); the re-measured attribution in Phase 3 names banding (~44%) as the next target.

## Spec IDs

| ID | Section | Status |
|---|---|---|
| [PIPELINE-INCREMENTAL] | The persisted parse store and its content addressing | ✅ implemented |
| [PIPELINE-INCREMENTAL-INTEGRITY] | Blob binding digest, bounded decode, size-bounded reads | ✅ implemented, pinned by `cache_blob_integrity.rs` + `fpcache/tests.rs` |
| [PIPELINE-INCREMENTAL-RETENTION] | Store pruning: stale-version partitions, orphan policy, 2 GiB budget | ✅ implemented, pinned by `cache_retention.rs` + `fpcache/retention/tests.rs` |
| [PIPELINE-INCREMENTAL-ANALYSIS] | What an incremental pass may reuse, and the equivalence it owes | ⏳ signature reuse implemented and pinned; downstream stages open |
| [CONFIG-INCREMENTAL-OPTOUT] | `[analysis] incremental = false` escape hatch | ✅ implemented, pinned by `signature_reuse.rs` |
| [PIPELINE-DETERMINISM] | The property every reuse rests on | ✅ implemented |
| [ACTION-CACHE] | The store restored and saved around the action's run step | ✅ implemented (`action.yml` restore/save around the run step), pinned by two contract checks + the two-runner `cache-seed`/`cache-warm` self-test |
| [CLI-ARG-DIFF] + [CLI-ARG-ONLY-CHANGED] | The two flags and their conflicts | ⚠️ implemented, but the branch-only #364 review below found open fail-open and output-contract blockers |
| [PIPELINE-DIFF-INGEST] | Strict unified-diff parser and tree verification | 🚫 not merge-ready: malformed/pathless sections, missing in-root targets, and metadata-only copies can scope to zero instead of refusing or tagging |
| [OUTPUT-SCHEMA-DIFF-TAGS] | The five `Option` wire fields | ⚠️ emitted, but hidden existing occurrences currently cannot veto `is_newly_introduced` |
| [METRICS-DIFF-SCOPE] | `metrics.diff` and the `--only-changed` gate | ⚠️ core gate rerouting works; filtered counts and the HTML gate verdict still contradict the report/exit contract |

## Checklist

The live TODO for this plan. Every work session updates this list in the same change as the work it records.

**Phase 0 — attribution and baseline**
- [x] Attribute the LSH block: signature construction ~69%, band enumeration ~30%, pair scoring ~1% (release, `crates/`, 92,973 fingerprints)
- [x] Record the attribution and the selected Phase 2 design in this plan
- [x] Commit a benchmark corpus that later phases measure against — the pinned tokio manifest (`corpus/tokio.json`, sha-verified clone)
- [x] Record the cold and warm baseline for that corpus in this plan — 5.97 s / 5.96 s / 5.58 s, reports identical modulo `cache_stats`
- [x] Commit cold golden reports that every later phase must reproduce byte-identically — `report_golden.rs` + `tests/fixtures/report-golden/` (byte-equality half plus an independent contract half derived from the authored sources)
- [x] Extend the golden to a mixed-language corpus — `incremental_multilang_golden.rs` + `tests/fixtures/incremental-multilang/` (Rust, Python, TypeScript, Dart, C#, Go; one authored Type-1 pair each, twelve byte-distinct files sharing one store). `expected-report.json` blessed and reviewed: exactly six `identical` clusters, one per language, weights ranked 52→35. Scanned at `--min-nodes 20` — below 14 the C# pair renders a second signature-line cluster that straddles [PIPELINE-CLUSTER-SUBSUME] containment by 7 bytes (gh #389, filed as its own edge)

**Phase 1 — equivalence contract ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE])**
- [x] E2E test: cold run vs warm run — reports identical field for field, `cache_stats` the sole difference (`incremental_equivalence.rs::cold_and_warm_cached_runs_match_the_uncached_cold_report`)
- [x] E2E test: one-file edit — warm report equals a cold run of the edited tree (`editing_one_file_matches_the_cold_report_of_the_post_edit_tree`)
- [x] E2E tests: file add, file delete, rename, revert-to-previous-content (four scenarios in `incremental_equivalence.rs`, each with exact `cache_stats` and cluster-shape assertions)
- [x] Per-language invalidation matrix — `incremental_multilang_matrix.rs`: touch one language (exactly 1 miss / 11 hits, all six clusters unmoved), delete one language (that cluster gone, other five field-for-field identical), revert (content-addressed full-hit restore), a six-step cumulative edit chain, and byte-identical `.ts`/`.js` twins proving the store key's language component
- [x] All equivalence tests green against today's behaviour before any reuse lands — verified 6/6 green; the reuse pin `signature_reuse.rs` is in the tree born-red against the missing `signatures_built`/`signatures_reused` event fields

**Phase 2 — signature persistence**
- [x] Decide the persistence format against #379 — full signatures in the parse blob; band hashes rejected because `estimate_jaccard` consumes full signatures for scoring and cluster means (rationale recorded above)
- [x] Blob format bump: signatures persisted beside fingerprints, positionally 1:1; decode rejects a count mismatch; pre-signature magic decodes as a plain miss (unit-pinned in `fpcache.rs`)
- [x] LSH consumes persisted signatures for unchanged files and constructs only for changed files — hit path validates re-derived fingerprints against stored records before attaching (`corpus/tests.rs` pins the reject-and-self-heal path); `signature_reuse.rs` green (4/4, including the store-disabled accounting contract and the `[analysis] incremental = false` config escape hatch)
- [x] Blob trust hardened ([PIPELINE-INCREMENTAL-INTEGRITY]): binding digest over the full address verified before decode, size-bounded reads, global decoded-node budget — findings 1, 2 and 4 of the [regression audit](#regression-audit--persisted-processing-2026-08-17), pinned by `cache_blob_integrity.rs` + `fpcache/tests.rs`
- [x] One-file change on the benchmark corpus measurably cheaper; Phase 1 equivalence tests green. Release, pinned tokio, `--embeddings off`, binding-digest format: `--no-incremental` 5.88 s / 1,649 MB; cold store-on 6.22 s / 1,665 MB; fully warm 2.91 s / 1,609 MB; **one-file edit (757 hit / 1 miss) 2.92 s** — 2.0× cheaper than the store-off pass; revert restores a full-hit 2.94 s pass. All six states render byte-equal reports modulo `cache_stats`. The edit pass is *not* cheaper than fully-warm because everything downstream of signatures still runs corpus-wide — exactly the remaining phases' target
- [x] Follow-up recorded: `band_key` identity concatenation instead of blake3 (Phase 0 attribution section)

**Phase 3 — re-measure**
- [x] Re-run attribution after Phase 2 (release, warm tokio pass, debug spans): discovery 7 ms (~0.2%) · parse-store load (decode + digest verify + fingerprint re-derivation, `signatures_built=0`) ~663 ms (~23%) · **LSH band enumeration ~1,276 ms (~44%) — now the dominant stage** · candidate scoring ~54 ms (~2%) · closure + rank + content ~82 ms (~3%) · buckets + metrics + JSON write ~0.7 s (~25%). Decision with numbers: signature construction is eliminated from the warm path, so the next targets in order are **banding (~44%)** — the already-recorded `band_key` follow-up and/or a persisted band index — then **buckets+metrics (~25%)**, then store-load decode (~23%). Buckets+metrics is now worth touching, but only after banding

**Phase 4 — parse-store economics**
- [x] Re-run the #379 disk numbers under the new economics. Store: **185.8 MiB / 759 blobs** for a 7.3 MiB source tree (~25×; signatures are ~85% of blob bytes; +32 bytes/blob for the binding digest is noise). Verdict: **keep** — the disk buys a halved warm wall and the store is the substrate the remaining phases build on. **Shrink path recorded**: if the banding phase persists the band index, the per-fingerprint signatures (~85% of the store) stop earning their bytes and the blob drops back to roughly the pre-signature 29 MB shape
- [x] Retention landed ([PIPELINE-INCREMENTAL-RETENTION]): nothing deleted under the 2 GiB budget, class-before-age eviction over it (other-version → orphan → live) — policy and rationale in the regression audit's retention section above, pinned by `cache_retention.rs` + `fpcache/retention/tests.rs` 7/7
- [x] Warm-RSS regression removed (audit finding 5): the per-render flatten replaced by one session-owned flat store (`session/store.rs`). Re-measured (release, pinned tokio, `--embeddings off`): `--no-incremental` 6.45 s / 1,532 MB · **fully warm 3.31 s / 1,495 MB** · one-file edit 3.39 s · revert full-hits; all states byte-equal modulo `cache_stats`

**Follow-ups**
- [x] Persisted-signature recall pinned on the LSH-only route (audit finding 6) — `lsh_only_nearmiss_recall.rs`: cold, warm, mixed, and revert each assert the exact signal triple, bucket, files, metrics, and an exact mixed-pass rebuild/reuse split
- [x] `tool_version` bound into the binding digest (audit finding 2); cross-partition relocation refused on both the decode path and the digest-distinctness axis test
- [x] Hostile-size gaps closed (audit finding 4): one-handle bounded read taken past the ceiling, fallible reserves, 4 M decoded-node budget claimed per node including its child slots
- [x] Retention safe across concurrently running tool versions: `OtherVersion` partitions never deleted under budget, evicted first under pressure — pinned at unit and E2E level
- [x] Stale `RED PIN` prose replaced with the row's recall + precision contract, and `LSH_ONLY_NEARMISS_MIN_JACCARD` **is** `pair::LSH_ONLY_MIN_JACCARD` rather than a copy of its value — one number, two named uses
- [x] Full `make ci` gate run on the final snapshot; the two accuracy defects it caught (#331 row-4 false positive, #108 unproven anchor-free promotion) are fixed at the root, not suppressed — see the audit's "Accuracy defects this audit uncovered"
- [x] `band_key` identity concatenation landed (the Phase 2 follow-up): band enumeration ~1,276 ms (~44%) → ~359 ms (~12.5–14.2% across three warm tokio passes). Red→green identity test on the old hash; `incremental_equivalence` 6/6, `signature_reuse` 4/4, `lsh_only_nearmiss_recall` 2/2, `report_golden` 2/2 byte-for-byte

**Phases 5–10 — CI cache and diff scoping**
- [x] Decisions recorded; specs updated in the same change ([ACTION-CACHE], [CLI-ARG-DIFF], [CLI-ARG-ONLY-CHANGED], [PIPELINE-DIFF-INGEST], [OUTPUT-SCHEMA-DIFF-TAGS], [METRICS-DIFF-SCOPE])
- [x] Phase 5 — action cache restore/save + two-pass self-test, closes gh #381 (`action.yml`: restore before / save after the run step under the per-run key with version+OS prefix fallback, `cache: "false"` opt-out, storeless-run save skip; contract checks pin ordering, key sharing, scan-root path, and opt-out; `action-selftest.yml` `cache-seed`→`cache-warm` runs on two runners and asserts `hits > 0`, `misses == 0`, and seed/warm reports `deepStrictEqual` modulo `cache_stats`)
- [ ] Phase 6 — parser + refusal: close the three P0 ingestion false-pass paths in the branch-only #364 review below; keep the existing stale/malformed/stdin coverage green
- [ ] Phase 7 — wire fields + tagging: resolve whether hidden existing occurrences veto `is_newly_introduced`, then pin the public definition from both directions
- [ ] Phase 8 — `--only-changed`, `diff_metrics`, gate: restore the declared `clusters_total == clusters.len()` invariant (or introduce a separately named repo-wide count) without changing the repo-wide line metrics
- [ ] Phase 9 — text summary, badges, HTML toggle: make HTML reflect the governing diff threshold, stop filtered-empty runs claiming the codebase is clean, and report the promised cross-file count
- [ ] Phase 10 — `diff:` / `only-changed:` action inputs: the 39 static contract checks are green, but run the black-box diff-gate job before calling the action path proven (the current self-test is skipped until release `>= 0.33.0`)
- [x] Post the worked `git diff | deslop` example on #364 — [comment](https://github.com/Nimblesite/Deslop/issues/364#issuecomment-5327619707): the fixture patch piped on stdin under `--only-changed --fail-over 5` (exit 3, `diff: 97.4% of added lines duplicated (37 / 38 added LOC)`, one newly introduced group, one untouched group omitted), plus the flip side — a clean diff passes at `--fail-over 0` where the repo-wide gate exits 3. **The issue stays open**; closing it is the user's call, not the agent's
- [x] Workspace lint gate back to green (`make lint`, `--all-targets --workspace`) and the defects it was hiding fixed: the no-newline marker refusal recorded above, plus the new wire fields that never reached the LSP, markdown, live and report-API test builders. Every clippy deny — `expect_used`, `expect_err`, `panic`, `indexing_slicing`, `unreadable_literal` — was cleared by making the tests return `Result` and propagate with `?`, never by an `#[allow]` and never by dropping an assertion
- [x] `diff_render_tags.rs` rebuilt (2026-08-19): its only test was an assertion-free `dump_renderings` that printed the renders and asserted nothing — the file's own header promised two pinned properties it never pinned. Replaced with three byte-exact tests over the real report shapes: the untagged render's exact pre-diff bytes (text golden + exact banner element, plain card classes, zero badge elements), the `--diff` and `--only-changed` text goldens (gate lines, badged occurrence rows, the delta line's honest counts — the delta renders only under `--only-changed`, where `apply_only_changed` has made "clusters intersecting" equal the body by construction), and the tagged HTML markers (`in-diff` card class ×2 vs plain ×1, `[in diff]`/`[existing]` badge spans ×3 each, both diff facet inputs and chip labels, banner with and without the delta segment)
- [x] Final branch verification (2026-08-19): `make test` exit 0 — 174 suites green with coverage enforced — and `make lint` exit 0 on the snapshot carrying the parser implementations, the `clusters_total` fix, and the renderer pins
- [x] On-main defects surfaced by this branch's audit filed as issues (never closed, per standing instruction): gh #412 (`make test` skips 30+ tests via `--skip ollama_ --skip corpus_` name-substring filters), #413 (`clusters_total` ≥2-visible banner/body divergence — fixed on this branch, still broken on main), #414 (`corpus_flutter_dart` breaches its own [CORPUS-CEILINGS], hidden by #412), #415 (`fused_score_bounds.rs` fail-open helper), #416 (`dependency_reactivity.rs` fail-open `unwrap_or_default()` assertion)

## #364 branch review — open merge blockers (2026-08-19)

**Classification against main.** None of the findings in this section exists on `main` (`e5b5ea0df72de020b03ef19bd2d90134a84d2ce0`): `diff_scope`, `--diff`, `--only-changed`, the diff tags/metrics, and their renderer/action paths are all introduced by this branch (`cffb79cec252a205a8360943a4d99b6a07363781`). They are therefore recorded in this plan rather than filed as main bugs. The independent on-main findings remain gh #412–#416 above.

**What is proven.** A real Deslop-core run over 140 production files / 33,399 LOC retained 2 diff-intersecting clusters and omitted 109, preserved the repo metrics exactly, kept untouched cross-file siblings as `[existing]`, measured the one added line as duplicated (`1 / 1 = 100%`), and exited `3` against a 6% diff ceiling while the repo-wide 5.44% verdict stayed clean. That validates the normal, line-bearing hunk path; it does not clear the blockers below. Focused suites are green (28 diff-core tests, 7 CLI E2Es, 39 action-contract checks), and the full local `make test` / `make lint` gates are green, but none asserts these failure modes.

| Priority | Open finding and reproduced evidence | Acceptance required to close |
|---|---|---|
| **P0** | **A malformed non-empty section without `+++` fails open.** `printf 'diff nonsense\n@@ -0,0 +1 @@\n+x\n' | deslop repo --diff - --only-changed --fail-over 0` exits `0`, reports `0` added LOC / `0` clusters, and omits all 3 legacy clusters although the repo verdict is breached. `parser.rs` accepts any `diff ` and permits a hunk on a pathless section; `verify.rs` ignores it. | A CLI E2E must exit `2` and name the offending diff line. Require a recognised target before hunks while preserving valid binary/deletion entries. |
| **P0** | **A supported target missing inside the scan root fails open.** A valid new-file hunk for `b/repo/src/missing.rs` produces the same exit-`0`, zero-scope result because `verify.rs` treats every corpus miss as ignorable. | An in-root supported missing target must be a stale-diff usage error naming path/line. Out-of-root and intentionally excluded/unsupported files remain ignorable. |
| **P0** | **A metadata-only 100%-similarity Git copy is invisible.** `similarity index 100%` plus `copy from legacy_a.rs` / `copy to legacy_b.rs` is accepted as `0 / 0` diff LOC and `0` clusters although the full scan detects the identical pair. | A real Git copy patch must resolve/verify source and target, mark every target line added, keep source `[existing]`, report 100%, and breach `--fail-over 0`; malformed/missing copy targets must exit `2`. |
| **P1** | **HTML can contradict the governing gate.** On the real run above, JSON/text and exit `3` say the 100% diff breached 6%; HTML renders green `metrics-banner--ok`, says the 5.44% repo threshold is OK, and omits the diff breach. `html.rs` derives class/text only from `metrics.threshold`, not `metrics.diff.threshold`. | Under `--only-changed`, the HTML status class and named gate verdict must derive from `metrics.diff.threshold`; E2E must assert the HTML breach agrees with exit `3`. |
| **P1** | **`is_newly_introduced` does not implement #364's “all occurrences” definition.** `tag.rs` filters hidden occurrences before `all()`, and its test explicitly lets a hidden existing copy fail to veto “newly introduced.” | Either make every out-of-diff occurrence — hidden included — veto the flag, or explicitly change the public issue/schema contract and stop every surface saying “all occurrences.” Pin the chosen rule. |
| **P1** | **Filtering breaks the declared cluster-count invariant.** The fixture renders `clusters.len() == 2`, `metrics.clusters_total == 3`, `clusters_outside_diff == 1`; text consequently says both “2 cluster(s)” and “3 clusters.” `apply_only_changed` filters after metrics and the new golden pins the contradiction, while [METRICS-REPO] says `clusters_total` always equals `clusters.len()`. | Choose one meaning and make names/specs/renderers agree. Preferred: keep repo-wide line metrics unchanged, set `clusters_total` to the rendered body count, and add a separately named field if consumers need the unfiltered repo count. |
| **P2** | **Scoped summaries mislead and omit a requested classification.** A filtered-empty run says “your codebase is clean” immediately after saying 3 untouched groups were omitted; the change fixture contains one newly introduced and one cross-file clone, but stderr/text never report the cross-file count requested by #364. | Say “no diff-affected duplication” when legacy clusters were omitted, preserve the omitted count, and render/assert `cross-file = intersecting - newly introduced` in stderr and text. |
| **P2** | **Action evidence is static only.** The 39 contract checks pass, but the composite action's diff-gate job is version-gated off until `>= 0.33.0`; there is no branch-executed black-box proof of the pre-release action path. | Execute a branch-built black-box action run covering clean legacy debt and a breaching changed-code diff before marking Phase 10 complete. |

- [ ] Close every P0/P1 row above with red-then-green tests before #364 is described as shipped or merge-ready.
- [ ] Re-run the real 140-file evidence case and the exact `make test` / `make lint` gates on the repaired snapshot.
