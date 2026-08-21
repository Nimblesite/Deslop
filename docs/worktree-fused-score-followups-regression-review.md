# Regression and test-strength review: `worktree-fused-score-followups`

## Verdict

**Do not merge the reviewed branch head (`36ee14bcfbaf7fd894594a865ded1187844c4ce5`) yet.**

The branch is 89 commits / 191 files ahead of its `main` merge base
(`42b2c928ca15e7e11b0e6439d647100dd4e1a5e7`). The review found one
branch-integrity failure, three high-risk false-positive/false-negative paths,
and multiple tests that exist in the tree but do not run in either ordinary CI
or the corpus target.

Per request, this review did **not** run full CI or the full corpus suite. The
only executed checks were cheap, focused unit/integration tests. Local files
also contain concurrent follow-up edits that are not present at the reviewed
remote head; those edits must be committed and re-reviewed before they can be
treated as branch fixes.

## Required actions before merge

1. Repair and compile-check the committed corpus target.
2. Add a cheap corpus preflight before fetch/build/scan, and refuse vacuous
   manifests.
3. Wire the new corpus precision tests into an executed target.
4. Add adversarial false-negative controls for the three widened suppression
   filters.
5. Commit CI coverage for the issue-report generator and site Playwright suite.

## Findings

### P0 — The committed corpus integration target is internally inconsistent

The branch comparison for `crates/deslop/tests/corpus_repos.rs` removes the
local `array` and `field_u64` helpers, but the committed file does not import
their replacements from `deslop_test_support::corpus`. The branch adds
`corpus_precision` to `deslop-test-support`, but the committed corpus runner
also does not import or call `check_boilerplate_not_ranked_first`.

This means the committed branch and the locally observed test behavior are not
the same artifact. A local overlay currently supplies the missing imports and
call, masking the branch-head failure.

Action:

- Import `array` and `field_u64` from `deslop_test_support::corpus`.
- Import and invoke `corpus_precision::check_boilerplate_not_ranked_first`.
- Include `boilerplate_rank` in the evaluated baseline check identifiers.
- Prove a clean branch-head checkout compiles with the cheap command:
  `cargo test -p deslop --test corpus_repos --no-run`.

### P0 — Expensive scans can run while asserting almost nothing

`Makefile` runs corpus fetch, release build, and then the corpus integration
test. It does not run a cheap manifest contract first. Structurally inspecting
the nine repository manifests shows:

- all nine omit a minimum `files_analysed` expectation;
- all nine omit a plausible cluster-count band; and
- Django, F#, Hugo, Jellyfin, Laravel, and React have no curated recall pair or
  ranking assertion at all.

Those six repositories can consume minutes and gigabytes, return an empty or
badly collapsed report, and still provide no evidence about accuracy. A warning
printed after the scan is not an assertion.

Action:

- Add a cheap `test-corpus-preflight` target and make both `test-corpus` and
  `test-corpus-ci` run it before `fetch-corpus.mjs` or a release build.
- In preflight, require every selected manifest to have a positive
  `expect_files_min`, a valid inclusive `expect_clusters.min..=max` band, and at
  least one curated recall or precision assertion.
- Refuse uncurated selected repositories instead of spending resources on a
  run whose result cannot establish accuracy.
- After a scan, assert `files_analysed` and cluster count against those curated
  bounds and include their check IDs in baseline classification.
- Validate ordering without a scan using `make -n test-corpus` and
  `make -n test-corpus-ci`.

### P1 — The global `corpus_` name filter skips at least 42 cheap tests

`crates/deslop-test-support/src/corpus_precision.rs` contains nine useful unit
tests. Their fully qualified names begin with `corpus_precision::`, while
`make test` passes `--skip corpus_`. The corpus target runs only
`-p deslop --test corpus_repos`, so it does not execute the support-crate unit
tests either. The same filter skips the confidence and scope helper units plus
ordinary non-network regression tests whose names happen to contain
`corpus_`—including diff-scope, token-Jaccard, synthetic-scale, live-config, and
LSP-refresh coverage. At least 42 cheap tests are excluded.

The tests pass when invoked manually, but neither normal CI nor the corpus
command invokes them. They therefore cannot prevent a regression.

Action:

- Make the expensive `corpus_repos` integration target explicitly opt-in by
  target/feature selection, then remove the global substring skip.
- Run the support-crate corpus contracts explicitly from
  `test-corpus-preflight` as a second line of defense.
- Add a cheap command-level contract that lists executed tests and fails if the
  precision/scope contract set is absent.
- Keep the expensive repository tests skipped from ordinary CI; do not skip the
  cheap logic that decides whether their results are meaningful.

### P1 — Body-shape comparison aliases different operators

`crates/deslop-core/src/cluster_filters/body_shape.rs` builds its comparison
stream from `named_children` only. Tree-sitter represents many operators and
punctuation tokens as anonymous children, so bodies such as `return a + b` and
`return a - b` can produce the same stream. Both the signature-only and
polymorphic-signature filters use that stream to decide whether implementations
differ.

This can preserve a false positive for same-signature methods whose behavior
differs only by an operator, or make the two suppression policies disagree with
the actual syntax.

Action:

- Include behavior-bearing anonymous token kinds in the shape stream while
  continuing to normalize identifier and literal text.
- Add a direct comparator unit test for `+` versus `-`, `==` versus `!=`, and
  `&&` versus `||`.
- Add an end-to-end same-signature fixture proving operator-distinct
  implementations are suppressed, while a consistently renamed copy remains
  visible.

### P1 — Literal-variation call suppression ignores non-call logic in the range

`crates/deslop-core/src/cluster_filters/calls.rs` reduces a reported range to
its contained call sequence. The widened sequence rule checks matching
callee/arity/keyword headers and requires each call position to vary by literal,
but it never proves the rest of the range is inert scaffolding.

A pair can therefore contain duplicated arithmetic, mutation, or branching and
one otherwise matching call with different string payloads; the call-only view
can classify the entire pair as disposable test/data scaffolding and hide real
Type-2/Type-3 logic.

Action:

- Require the member range, after removing the accepted call expressions, to
  contain only an explicit AST whitelist of inert scaffolding.
- Add a two-sided negative fixture with identical non-call logic plus a
  literal-varying call; assert the real clone remains visible.
- Retain a positive fixture containing call scaffolding only, so tightening the
  rule does not reintroduce the original false positive.

### P1 — Python collection-cell suppression uses a one-node blacklist

`crates/deslop-core/src/cluster_filters/python_collection_cells.rs` suppresses
differing snippets in one collection literal unless a snippet contains a
`lambda`. Calls, comprehensions, conditional expressions, assignments via
expressions, and other behavior-bearing subtrees are not protected.

Consequently, repeated extractable logic in sibling dictionary/list entries can
be hidden simply because it is inside the same collection and is not spelled as
a lambda.

Action:

- Replace the `lambda` blacklist with a positive AST definition of inert record
  cells (literal and simple identifier/member payloads only).
- Add dedicated fixtures for call, comprehension, and conditional-expression
  cells and assert each real clone stays visible.
- Move the GH #421 assertion out of
  `python_issue_69_abstract_method.rs`; its current all-empty expectation has no
  same-run positive detector control.

### P1 — The shared negative pin can pass without proving the target filter fired

`crates/deslop/tests/common/negative_pin.rs` checks that no visible cluster spans
the family and that the report-wide `clusters_hidden` count is at least one.
That counter is global. An unrelated hidden cluster can satisfy it while the
target family was never discovered or never reached the intended filter.

The visible control proves the detector is alive in general, but does not prove
the target candidate was generated and suppressed for the expected reason.

Action:

- Expose test-only suppression diagnostics keyed by reason and occurrence
  paths/ranges.
- Assert that the target family was generated and hidden by the intended filter
  reason, not merely that some cluster was hidden.
- Keep the unrelated visible clone control as the false-negative guard.

### P1 — The determinism gate compares only cluster IDs

`corpus_repos.rs` documents a full report-agreement guarantee, but
`determinism_gate` fails only when the ordered cluster-ID vector changes.
`duplication_percent` is printed, not asserted. Stable IDs with changed
occurrences, ranges, buckets, scores, ordering details, or repository metrics
therefore pass. Missing IDs are silently discarded by `filter_map`, weakening
the comparison further.

Action:

- Compare canonicalized full JSON reports, excluding only fields proven to be
  intentionally nondeterministic.
- Fail on a missing cluster ID rather than dropping it.
- Add synthetic controls with equal IDs but changed duplication percentage and
  equal IDs but changed occurrence ranges.

### P1 — Type-1 recall accepts hidden or demoted matches

The Type-1 corpus recall check delegates to `reports_clone_spanning`, which
searches occurrences without requiring that the cluster is shown or remains in
an actionable/identical bucket. A hand-curated byte-identical pair can be hidden
or demoted and still satisfy recall.

Action:

- Require each `must_find` pair to appear in a shown `identical` cluster with
  the expected distinct files and occurrence count.
- Add negative controls where the pair exists only in a hidden cluster and only
  in a structural/demoted cluster; both must fail recall.

### P1 — Mutable JavaScript declarations are treated as constant tables

The ECMAScript constant-table predicate accepts `const`, `let`, and `var`
declarations identically when their initializers are literal-shaped. Mutable
state is not a constant registry: copied runs can be behaviorally significant,
especially when the values are updated later. Existing coverage exercises
`export const`, not the mutable forms.

Action:

- Require the declaration keyword to be `const`; classify `let` and `var` as
  behavior-bearing/non-table declarations.
- Add two-file `let` and `var` fixtures with later mutation and assert exact
  visible paths, occurrence count, rank/bucket, and duplicated LOC.

### P1 — Branch-head CI does not execute the new site/generator tests

The branch adds `scripts/issues/test_generate_issue_report.py` and a large
Playwright suite in `site/tests/issues.spec.js`. At the reviewed branch head,
the site job builds the site but runs neither Python unit tests nor
`npm test`. `site/package.json` also makes every build perform a live issue
refresh through `gh`, so a local/offline build now depends on network and
authentication.

A concurrent local overlay adds Python validation, classifies
`scripts/issues/**` as site input, and runs Playwright, but those changes are not
in the reviewed remote branch.

Action:

- Commit the CI wiring: classify `scripts/issues/**` as site input; set up
  Python; run the unit/type checks; install Chromium; and run Playwright.
- Grant the least GitHub token permission required by the live refresh.
- Make ordinary site builds deterministic from a checked-in/input snapshot;
  reserve the live GitHub refresh for an explicit refresh/deploy target.

### P2 — Issue relationships accept unrelated `#123` text as local edges

`extract_references` in `scripts/issues/generate_issue_report.py` treats every
`#` followed by digits as a local issue reference when the number happens to be
open. It does not exclude fenced/inline code or foreign `owner/repo#123`
references. This can add false graph edges and distort inbound-link priority.

Action:

- Parse GitHub reference context rather than scanning every hash-number token.
- Accept local `#123` and this repository's qualified references; reject foreign
  repository references and code spans/blocks.
- Add table-driven tests for local, same-repository qualified, foreign,
  inline-code, fenced-code, duplicate, closed, and self references.

## Focused validation plan

Run these after the fixes; none is a full corpus or full CI run:

1. `cargo test -p deslop --test corpus_repos --no-run`
2. the new cheap corpus preflight target
3. `cargo test -p deslop-test-support --lib corpus_precision`
4. only the new adversarial filter fixtures
5. `python -m unittest scripts.issues.test_generate_issue_report`
6. the issue-page Playwright spec only

Run at most one selected corpus repository only when a focused fixture cannot
reproduce a suspected regression and the scan is being used to hunt that
specific defect. A full corpus or CI run is explicitly out of scope for this
review.
