# Proposal: Recover the corpus accuracy lost since #518 (`ACCURACY-RECOVERY`)

```
Status:   Proposed — no engine code changed by this document
Compared: f92300e5e1004ef6c53a94174a0d7e842232ec80 (15 Aug) against acd5815ad080271d1f1e6fc244d6da229e0e9734 (21 Sep)
Scope:    all 13 judged registers, 280 judged pairs, 9 languages
```

The engine answers fewer judged pairs correctly today than it did in August: 274 of 280 against 276 of 280. It fixed three old defects and introduced five new false negatives, brought one false positive back, and hides a sixth false negative behind a hole in the scorer. **Every one of those defects entered after #518 (`8324e479`), which scores 100% on every repository probed.** Nothing here needs new detection machinery. It needs one rule made proportional, two regressions from a single pair of commits traced and undone, and the measuring equipment repaired so the next one cannot ship unseen.

## [ACCURACY-RECOVERY-EVIDENCE] Where every figure comes from

No figure below was worked out by hand. Each is lifted from a file the project's own tools wrote.

| What | File or command |
|---|---|
| The two-engine scorecard | `.corpus/version-compare/reports/SCORE.md` and `score.json`, written by `scripts/compare-versions.sh f92300e5e1004ef6c53a94174a0d7e842232ec80 acd5815ad080271d1f1e6fc244d6da229e0e9734` |
| What each engine reported | `.corpus/version-compare/reports/<repo>/<engine>/report.json` |
| The judged pairs | `corpus/register/polly.json`, `corpus/register/fsharp.data.json`, `corpus/register/axios.json`, `corpus/register/click.json` |
| Per-commit verdicts | each commit built from `git archive`, scanned with the comparison's flags (`--no-incremental --no-fail-over --no-color --embeddings off`) and scored by `corpus-score score`. Any row reproduces with `scripts/compare-versions.sh <commit-a> <commit-b> "<url>#<sha>#<language>"` |

`.corpus/` is git-ignored, so the scorecard is reproduced rather than read from the repository. The separate `corpus_repos` suite (`make test-corpus`) was not run for this analysis: at today's scan cost django alone takes fifteen minutes and the suite's larger repositories several times that, so everything here rests on the judged registers. The register pins are `https://github.com/App-vNext/Polly.git#9c133b6303ca70d2879fddf16fb201c209eb6fd2#csharp` and `https://github.com/fsprojects/FSharp.Data.git#3d432efaeffb0fa987f70f400521d4cb7cc6f89a#fsharp`.

### The scorecard

| measure | `f92300e5` | `acd5815a` |
|---|---|---|
| score | 98.6% | 97.9% |
| correct / judged | 276/280 | 274/280 |
| false negatives | 2 | 5 |
| false positives | 2 | 1 |

Eleven repositories are clean on both engines or better on the new one. All of the loss is in two: Polly (35/36 found, now 31/36) and FSharp.Data (the one CLEARLY OUT pair is reported by both).

### The history, commit by commit

Polly against `corpus/register/polly.json`, 38 judged pairs:

| commit | correct | scan wall | note |
|---|---|---|---|
| `f92300e5` | 37/38 | 6 s | misses the five-line `RetryAsync` callback |
| `8324e479` #518 | **38/38** | 11 s | the register and the gate arrive here |
| `77b4681b` #508 | **38/38** | 10 s | last good |
| `978002e2` #529 | 30/38 | 13 s | **first bad — eight sync/async twins lost in one commit** |
| `47947ddd` #541 | 30/38 | 13 s | |
| `79b5a128` | — | — | panics on Polly (exit 101), cannot be scored |
| `aac64bae` | 31/38 | 224 s | in this commit or `79b5a128`: the within-file pair of [ACCURACY-RECOVERY-MASKED-FN] is lost, the FSharp.Data false positive returns, and scan cost rises 17-fold |
| `acd5815a` | 33/38 | 353 s | three twins recovered as `loosely_similar`, five never recovered |

`8324e479` was also scanned over axios, click and FSharp.Data: 55/55 + 1/1, 18/18 and 11/11 + 1/1. Wall times were taken on a machine other builds were using, so read them as orders of magnitude.

## [ACCURACY-RECOVERY-TWIN-FN] Five false negatives: sync/async twins erased by the call-target rule

### The failing judged pairs

`corpus/register/polly.json`, `clearly_in` entries 6, 16, 19, 28 and 34 (counting from zero). The gate prints them as `Polly false negatives: allows at most 0, recorded 5`.

| entry | pair | judge's check |
|---|---|---|
| 6 | `Wrap/PolicyWrapSpecsAsync.cs:38-109` + `Wrap/PolicyWrapSpecs.cs:37-109` | 72 vs 73 lines, similarity 0.98 |
| 16 | `Wrap/PolicyWrapSpecsAsync.cs:18-104` + `Wrap/PolicyWrapSpecs.cs:17-104` | 87 vs 88 lines, similarity 0.98 |
| 19 | `Caching/CacheAsyncSpecs.cs:17-48` + `Caching/CacheSpecs.cs:16-47` | 32 vs 32 lines, similarity 0.99 |
| 28 | `Wrap/PolicyWrapSyntaxAsync.cs:8-42` + `Wrap/PolicyWrapSyntax.cs:8-42` | 35 vs 35 lines, similarity 0.96 |
| 34 | `Wrap/PolicyWrapSyntaxAsync.cs:44-77` + `Wrap/PolicyWrapSyntax.cs:44-77` | 34 vs 34 lines, 4 lines changed |

### A clear example

Entry 34 is two 34-line blocks. This is the whole difference between them:

```diff
-        public PolicyWrap<TResult> Wrap(ISyncPolicy innerPolicy)
+        public PolicyWrap<TResult> WrapAsync(IAsyncPolicy innerPolicy)
-                (func, context, cancellationtoken) => PolicyWrapEngine.Implementation<TResult>(func, context, cancellationtoken, this, innerPolicy),
+                (func, context, cancellationtoken, continueOnCapturedContext) => PolicyWrapEngine.ImplementationAsync<TResult>(func, context, cancellationtoken, continueOnCapturedContext, this, innerPolicy),
```

The same two edits repeat once more for the generic overload. Thirty of thirty-four lines are byte-identical. Nobody reading the two files would call this anything but a copy.

### What each engine reported

`f92300e5` published the pair twice over: cluster `8d7c18de33607631` at exactly `8-42` + `8-42`, and cluster `297511bba0a7f175`, six copies of the inner 51-node block across both files. For entry 6 it published twelve cross-file clusters, among them `51f8d27d30e9873b` at exactly `38-109` + `37-109` (461 nodes, `nearly_identical`, structural 1.00, token 1.00). For entry 19, cluster `9bc31c32ac2e4853` at exactly `17-48` + `16-47` (196 nodes).

`acd5815a` publishes **no cluster that contains both files** over any of the five ranges. The files are not hidden — they appear in the report with 5 to 108 occurrences each. What survives is one-sided: the six-copy block of entry 28 is now two separate three-copy clusters, one per file (`08858d6e4ec45094` and `79ccbfe2ae88e128`), and `CacheAsyncSpecs.cs:17-34` pairs with its generic sibling `CacheTResultAsyncSpecs.cs` but no longer with `CacheSpecs.cs`.

### Why it used to be answered correctly

Up to and including #508 the content gate read a twin as what it is: a consistent rename. `Cache`→`CacheAsync`, `ISyncCacheProvider`→`IAsyncCacheProvider`, `Wrap`→`WrapAsync` each map one way, every time, and nothing contradicts the mapping, so the pair was admitted as a Type-2 clone and routed `nearly_identical`.

#529 added `crates/deslop-core/src/content/call_targets.rs` and [FUSED-CONTENT-GATE-CALL-TARGET]. The rule: a changed method name on a receiver that was *not itself renamed* is a changed operation, not a rename, and it contradicts the copy. `client.startJob` against `client.cancelJob` is its model case. `contradicts()` is an `any()` — one such position rejects the whole pair.

A sync/async twin is that pattern by construction. `Policy.Cache(..)` against `Policy.CacheAsync(..)`, `PolicyWrapEngine.Implementation` against `PolicyWrapEngine.ImplementationAsync`: the receiver is a static class both sides share, so it can never be "a corroborated rename", and the selector change is read as a contradiction. The pair is not demoted. It is erased, along with every smaller window inside it that contains a call.

### Proof that this rule is the cause

Copy the two real files of entries 28 and 34 into an empty directory and scan it with the `acd5815a` binary: 5 clusters, **none cross-file**. The false negative reproduces with two files and nothing else. Now change one token in the async copy — `PolicyWrapEngine.ImplementationAsync` to `PolicyWrapEngine.Implementation` — and scan again: the engine reports `PolicyWrapSyntax.cs:7-138` + `PolicyWrapSyntaxAsync.cs:6-140` as **one 516-node `nearly_identical` cluster**. Everything else in the twin, `Wrap`→`WrapAsync` and `ISyncPolicy`→`IAsyncPolicy` included, was already acceptable to the engine.

The same experiment on the other two file pairs, removing the `Async` suffix from call selectors in the async copy: `CacheSpecs` goes from 18 small cross-file clusters to 34, and the largest becomes `CacheAsyncSpecs.cs:17-48` + `CacheSpecs.cs:16-47`, 196 nodes — the judged pair of entry 19, node for node the cluster `f92300e5` published. `PolicyWrapSpecs` goes from 4 cross-file clusters to 29.

### Why the rule exists, and why it overreaches

#529 answered a real defect: eighteen unrelated Playwright statements of two, three and five lines ranked first as one duplicate. `expect(row).toHaveCount(1)` against `expect(row).toContainText('a')` share a shape and nothing else. At that size the method name *is* the content, and refusing the pair is right.

The rule has no sense of proportion. It gives the same veto to a three-line statement, where the selector is most of what was written, and to an 88-line test class at 0.98 similarity, where it is two tokens in several hundred. "Not a rename" is a fair verdict on `Implementation`→`ImplementationAsync`. "Not a clone" does not follow from it: a copy with a few edited lines is the textbook Type-3 clone, and [CLONE-BUCKETS-ROUTING] already has a category for it.

### The repository's own test asserts the same mistake

`changed_operations_on_the_same_receiver_are_not_a_rename` in `crates/deslop/tests/type2_rename_call_targets.rs` scans `fixtures/type2-rename-method-calls/same-receiver/ledger.ts` and `reversal.ts` — 20 lines each, 15 byte-identical, 5 differing only in a method name on `ledger` — and asserts that **nothing is reported and no line is duplicated**. The independent register says the opposite about the same shape of pair, five times. Specification and test on one side, judged ground truth on the other: under this repository's rules that is a stop-and-decide, which is why this is a proposal and not a patch.

## [ACCURACY-RECOVERY-MASKED-FN] A sixth false negative the scorer cannot see

`corpus/register/polly.json` entry 30: `Caching/SerializingCacheProvider.cs:59-100` + `:10-51`, the generic and non-generic provider classes in one file. `f92300e5`, #518, #508, #529 and #541 all publish that pair (at `f92300e5`: 132 nodes, `nearly_identical`, exactly `10-51` + `59-100`). `acd5815a` publishes a single cluster touching the file: `SerializingCacheProvider.cs:4-101` + `SerializingCacheProviderAsync.cs:5-127`, `loosely_similar`. The two classes are never shown against each other.

The scorer marks the entry **correct**. `matching_cluster` in `crates/deslop-test-support/src/corpus_score.rs` asks whether every judged range is overlapped by *some* visible occurrence, and the one occurrence `4-101` overlaps both. A pair is two places; one occurrence cannot show a pair. Re-reading all 275 CLEARLY IN entries with the requirement that the ranges land on distinct occurrences changes exactly this one verdict, and only on the new engine. The honest count for `acd5815a` is six false negatives and a score of 273/280.

The pair was lost between #541 and `aac64bae` — the same window in which the Polly scan went from 13 s to 224 s.

A related softness, reported as description rather than as defects: the scorer accepts any one-line overlap. Of the 270 CLEARLY IN entries it marks correct for `acd5815a`, 41 are matched by a cluster covering under 60% of the judged range; for `f92300e5` it is 2 of 273. Entry 22 (`AdvancedCircuitBreakerSpecs.cs:44-138` against its async twin, 95 lines) counts as found on a cluster covering 39% of it. That is not 41 more false negatives — register extents were drawn from older engines' cluster boundaries, and the shifted-window pairs in axios `index.ts` are fairly reported today as a smaller repeating unit. It does mean the scorecard cannot tell a match from a touch, and a reader should be able to.

## [ACCURACY-RECOVERY-SHAPE-FP] One false positive, fixed once and now back

### The failing judged pair

`corpus/register/fsharp.data.json`, the single `clearly_out` entry: `src/FSharp.Data.Http/Http.fs:57-73` + `tests/FSharp.Data.Core.Tests/WorldBankRuntime.fs:23-173`. The gate prints `FSharp.Data false positives: allows at most 0, recorded 1`.

### A clear example

```fsharp
// Http.fs:57-73 — HTTP method names
let MkCol = "MKCOL"
/// Creates a duplicate of the source resource ...
let Copy = "COPY"
```

```fsharp
// WorldBankRuntime.fs:23-173 — mock JSON payloads
let mockIndicatorResponse = """
[
  { "page": 1, "pages": 1, "per_page": 1000, "total": 2 },
  ...
```

Seventeen lines against a hundred and fifty-one. Both are a run of `let name = "string"` bindings, so after normalisation they are the same 40-node tree; a triple-quoted string is one leaf however many lines it spans.

### What each engine reported, and when it was right

`f92300e5` published it as cluster `71f484c2fe6a6c20`, bucket `structural_only`, structural 1.00, token 0.00 — one of 366 shape-only clusters in that report. The axios false positive of the same run (`test/manual/promise.js:9` + `lib/axios.js:50-60`) was the same thing: `structural_only`, token 0.00, one of 55.

By `54d06fb2` and at #518 **the pair is not reported**: FSharp.Data scores 11/11 + 1/1. `acd5815a` reports it again, as cluster `a2daf2681f6f3338`, kind `structural_only`, severity `none`, mass 0, rank 0 — one of only 4 shape-only entries left in that report. The axios pair stayed fixed; of its 55 shape-only clusters one remains and it is not the judged one. So the scorecard's label "standing" is true of the two compared engines and misleading about the history: this defect was closed and has reopened. It is still absent at #541 and at `cde0afb8`, and present at `aac64bae`; `79b5a128` between them panics on FSharp.Data as it does on Polly. So it came back in the two `Fixes` commits of 12–17 September — the same window as [ACCURACY-RECOVERY-MASKED-FN] and the 17-fold rise in scan cost.

### Why it is reported

The new taxonomy is doing what it says. [CLONE-BUCKETS-ROUTING] sends "identical normalized structure with no or negligible content similarity" to StructuralOnly, and `pipeline.md` keeps such entries in `clusters`, "clearly distinguished and placed last". The engine is not claiming a clone: severity `none`, mass 0, no diagnostic, nothing added to `duplicated_loc`. The scorer does not know that. It counts any published cluster as a report, so an entry the engine labels "same shape, different content — not a clone" breaches a CLEARLY OUT exactly as a red `identical` cluster would.

Two things are true at once. A reader shown HTTP verbs beside World Bank fixtures, under any label, has been shown noise. And the project's own noise rules already say what this is: a run of literal bindings is a data table, and [CLONE-NOISE-LITERAL-TABLE] exists to keep tables out of the report. It does not reach F# module-level `let` bindings.

## [ACCURACY-RECOVERY-FIXED] What the 38 commits got right

| judged pair | `f92300e5` | `acd5815a` |
|---|---|---|
| axios CLEARLY OUT `test/manual/promise.js:9` + `lib/axios.js:50-60` | reported, `structural_only`, token 0.00 | not reported |
| click CLEARLY IN `decorators.py:258-290` + `:133-165` (`command`→`group`, `CmdType`→`GrpType`) | no cluster touches either range | cluster `987d1bd1f1614bcf`, `nearly_identical`, 152 nodes, exact extents |
| Polly CLEARLY IN `RetryAsyncSpecs.cs:378-382` + `RetryTResultSpecsAsync.cs:438-442` (byte-identical, five lines) | no cluster touches either range | cluster `92c3c6552329c44d`, `identical`, 30 nodes |

All three were already right at `54d06fb2`. The click pair is a same-file rename and the Polly pair sits exactly on the 30-node `min_nodes` floor; which commits surfaced them was not bisected. These are real gains and any fix below must keep them: they are in the registers, so the gate will hold them.

## [ACCURACY-RECOVERY-BLINDSPOT] Why this shipped unseen

`scripts/corpus/score-gate.sh` scores a default slice of click, cobra, axios and Deslop — the only register scoring CI performs. There is no C# and no F# in it. #529 took Polly from 38/38 to 30/38 and every gate stayed green, because no gate looks at Polly. The slice was chosen for speed, and the repositories that regressed are exactly the ones it leaves out. The full comparison (`make compare`) would have caught it and was not part of any merge.

The registers are also lopsided. They hold 275 CLEARLY IN pairs and **5 CLEARLY OUT pairs across all thirteen repositories**, against the README's target of roughly a hundred of each per repository. Recall is measured; precision barely is. That matters because the headline figure is moving with nothing to hold it: `.corpus/version-compare/reports/Polly/SUMMARY.md` has `duplication_percent` at 50.0% on `f92300e5` and 71.2% on `acd5815a`, and the per-commit scans put it at 81.8% at #518 and 65.2% at #541. bloc goes from 18.4% to 38.6%. One of those Polly numbers may be right. No test, register or gate can say which, and the percentage is half of what this project promises to get right.

The scan cost makes this worse each week. Scanning the thirteen registers took three and a half minutes on `f92300e5` and seventy-one on `acd5815a` (zod alone: 9 s to 1972 s). A check that takes seventy minutes does not get run.

## [ACCURACY-RECOVERY-PLAN] The proposal

Five steps, each test-first, in this order. Steps 1 and 2 need a decision from the project owner before any code moves, because each changes what an existing specification or test says.

### [ACCURACY-RECOVERY-PLAN-CALL-TARGET] 1. A changed call target costs the rename, not the clone

**Decision needed:** may [FUSED-CONTENT-GATE-CALL-TARGET] be amended, and the assertion in `changed_operations_on_the_same_receiver_are_not_a_rename` replaced by a stronger positive one?

Recommended change. When `call_targets::contradicts` fires, the pair loses what the rename proof would have given it: it cannot be `identical`, and it cannot reach `nearly_identical` through the Type-2 rename route. It stays a candidate. The contradicting positions count as edits, the pair's content is measured with them included, and [CLONE-BUCKETS-ROUTING] places it — or the ordinary admission floors refuse it. At two to five lines a changed selector is most of the content, so the #520 statements fail admission exactly as they do today. At 34 lines with 4 changed, or 88 lines at 0.98, the pair is published as the edited copy it is.

What this deliberately does not do: it does not re-admit a twin as a *rename*. If the owner wants `X`→`XAsync` recognised as a systematic rename and labelled `nearly_identical` — every changed selector in the pair differing by one consistent affix, on a receiver both sides share — that is a second, separable rule with its own fixtures. The register only requires the pair to be reported, so step 1 closes the five false negatives without it.

Rejected alternative: reverting the rule. It reopens #520.

Tests first, all black-box against the CLI, all failing today:

- A C# fixture cut from the shape of entry 34: two files, a sync and an async twin of 30+ lines differing only in `Foo`→`FooAsync` selectors on a shared static receiver plus the `Sync`→`Async` type rename. Assert one cluster, two occurrences, both paths, exact line extents, a clone kind, and both files' `duplicated_loc`.
- The existing `same-receiver` and `near-miss-same-receiver` corpora, re-asserted positively: the pair is reported, its kind is **not** one the rename proof grants, `duplicated_loc` covers the 15 shared lines. Before changing that assertion, hand `ledger.ts` and `reversal.ts` to the blinded `judge-clone-pairs` protocol, so the new expectation rests on the same kind of independent verdict as the register and not on this document's opinion.
- A guard that #520 stays fixed: the Playwright statement corpus, `js_literal_variation_calls::member_targets` and `cluster_extent_statement_runs` pass unchanged.

### [ACCURACY-RECOVERY-PLAN-LITERAL-TABLE] 2. Keep unrelated literal runs out of the report again

This pair was correctly silent from `54d06fb2` to `cde0afb8` and came back with the routing rework in `79b5a128`/`aac64bae`. Across that step FSharp.Data's published shape-only entries fall from 255 to 4 and this pair becomes one of the 4 — so the rework changed *which* pairs route to shape-only, and what had been keeping this one out was not identified here. The first task is to read those two commits for the filter or floor that stopped applying, and restore it. If nothing principled turns up, the fallback is to extend [CLONE-NOISE-LITERAL-TABLE]: a member made only of value bindings whose right-hand sides are literals is a data table, read from the parse tree, in every language and not only where a collection literal encloses it.

Test first either way: an F# fixture with two unrelated runs of string bindings of very different lengths, asserting an empty visible surface, plus a positive control — a genuinely copied F# function beside them that must still cluster.

**Decision needed, separately:** should the scorer treat a `structural_only` entry as a report at all? [CLONE-BUCKETS-STRUCTURAL-ONLY] says it is not a clone claim. If the answer is no, the rule must be symmetric — a CLEARLY IN matched *only* by a `structural_only` entry becomes a false negative — and it must be written into [CORPUS-SCORE] before the scorer changes. This proposal recommends fixing the engine first and deciding the scorer question on its own merits, so that a definition change is never what turns a red gate green.

### [ACCURACY-RECOVERY-PLAN-SCORER] 3. Make the scorer count pairs, and show how much it matched

- `matching_cluster` must place each judged range on a **distinct** visible occurrence. Test first in `crates/deslop-test-support/src/corpus_score/tests.rs`: a two-range same-file entry against a report whose single occurrence spans both ranges must score as a false negative. This turns Polly entry 30 red, which is correct.
- Each scored entry records the share of its judged lines the matching cluster covers, and `SCORE.md` prints it, computed in Rust with everything else. Description, not a gate, until the registers have been re-read with it in view.

### [ACCURACY-RECOVERY-PLAN-WITHIN-FILE] 4. Restore the within-file pair

Entry 30 disappears between #541 and `aac64bae`. In the new report the only cluster touching the file is the whole-file pair against `SerializingCacheProviderAsync.cs`, which suggests the enclosing pair now displaces the contained same-file one, but that is a reading of the output, not a traced cause. `79b5a128` cannot scan Polly, so the window cannot be narrowed by measurement; it needs reading. Test first: a fixture with two near-identical classes in one file and a loosely similar sibling file, asserting both the within-file cluster and the cross-file one, with extents.

### [ACCURACY-RECOVERY-PLAN-GATE] 5. Put the regressed languages where a merge can see them

Add a C# and an F# register to the score-gate slice. Polly scanned in 11 s at #518, so this is affordable once scan cost is back where it was and not before; the README already asks for "a small csharp repo to sit under Polly", and judging one is the cheaper route if cost recovery slips. Until then, run `scripts/compare-versions.sh` against the last release on every change under `crates/deslop-core/src/content/` or `cluster_filters/`.

Queue a judging pass aimed at CLEARLY OUT pairs, starting with the repositories whose `duplication_percent` moved most (Polly, bloc, tokio), so the next routing change has a precision figure to answer to.

The 17-fold cost rise between #541 and `aac64bae` deserves its own investigation. It is outside an accuracy proposal, but it is the reason step 5 cannot simply be switched on.

### Done means

`scripts/compare-versions.sh 8324e4795c7081a4ebc1f7876055ccac2b0acf50 <fix>` over every register shows: Polly 36/36 + 2/2 under the distinct-occurrence scorer; FSharp.Data 11/11 + 1/1; no repository below its #518 figure; the gate passes with `score-thresholds.json` still holding `"repos": {}`.

## For AI

- Spec sections touched: [FUSED-CONTENT-GATE-CALL-TARGET] and [FUSED-SHARED-SUBTREE-CORE] (`docs/specs/fused.md`), [CLONE-BUCKETS-ROUTING] and [CLONE-BUCKETS-STRUCTURAL-ONLY] (`docs/specs/taxonomy.md`), [CLONE-NOISE-LITERAL-TABLE] (`docs/specs/exclusion.md`, `docs/specs/noise.md`), [CORPUS-SCORE] and [CORPUS-SCORE-RENDER] (`docs/specs/corpus.md`).
- Code: `crates/deslop-core/src/content/call_targets.rs` (`contradicts`, `selector_renames`) and its caller in the content gate; `crates/deslop-test-support/src/corpus_score.rs` (`matching_cluster`, `overlaps`, `ScoredEntry`); `crates/deslop-test-support/src/corpus_score/render.rs`; `scripts/corpus/score-gate.sh` (`DEFAULT_SLICE`).
- Tests that pin today's behaviour: `crates/deslop/tests/type2_rename_call_targets.rs` with `fixtures/type2-rename-method-calls/{same-receiver,near-miss-same-receiver}`; `js_literal_variation_calls::member_targets`; `cluster_extent_statement_runs`; `crates/deslop-test-support/src/corpus_score/tests.rs::overlap_matches_a_clearly_in_that_exact_line_equality_would_miss`.
- Two-file reproduction of [ACCURACY-RECOVERY-TWIN-FN]: copy `src/Polly.Shared/Wrap/PolicyWrapSyntax.cs` and `PolicyWrapSyntaxAsync.cs` from the pinned Polly checkout into an empty directory; scan with `--no-incremental --no-fail-over --no-color --embeddings off`; expect zero clusters whose occurrences span both paths. Replace `PolicyWrapEngine.ImplementationAsync` with `PolicyWrapEngine.Implementation` in the async copy; expect one `nearly_identical` cluster spanning both files.
- First bad commit for [ACCURACY-RECOVERY-TWIN-FN]: `978002e2edc67132f74fffeef9084c901b6b9920`; last good `77b4681b0f45cc0d5e4b83ce1fb0924d0fa50d7f`. Window for [ACCURACY-RECOVERY-MASKED-FN], [ACCURACY-RECOVERY-SHAPE-FP] and the cost rise: last good `cde0afb8` (documents only, after `47947ddd`), first bad `aac64bae`, with `79b5a128` between them unscannable (exit 101 on Polly and on FSharp.Data).
- Polly entries lost at #529 and since recovered as `loosely_similar` (register indexes 5, 25, 26) must stay found under step 1.
