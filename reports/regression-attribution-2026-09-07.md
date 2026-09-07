# Regression attribution — gh #520 to #526

Measured against `main` at `77b4681b` by `scripts/repository/regression-attribution.py`. Every row is a `git log -S` result re-checked by counting the anchor string at the commit and at its parent.

## Attributed transitions

| gh | transition | PR | commit | check |
| --- | --- | --- | --- | --- |
| #520 | literal-variation filter bails out on any body-carrying call | #518 | `8324e479` | verified: absent at parent, 3 occurrence(s) after |
| #520 | structural family split stage added to the cluster pipeline | #518 | `8324e479` | verified: absent at parent, 1 occurrence(s) after |
| #521 | cluster colour table rekeyed from clone kind to rank band | #485 | `1ecfc997` | verified: 1 occurrence(s) at parent, 0 after |
| #521 | spec table mapping NearlyIdentical to a colour deleted | #485 | `1ecfc997` | verified: 2 occurrence(s) at parent, 0 after |
| #521 | assertion that a demoted family is not painted act-now deleted | #485 | `1ecfc997` | verified: 1 occurrence(s) at parent, 0 after |
| #521 | band-keyed colour table added under the SEVERITY-COLOR id | #485 | `1ecfc997` | verified: absent at parent, 2 occurrence(s) after |
| #521 | bucket-keyed colour table removed | #485 | `1ecfc997` | verified: 2 occurrence(s) at parent, 0 after |
| #522 | percentile ramp table added | #36 | `dcc12296` | verified: absent at parent, 1 occurrence(s) after |
| #522 | chart-palette table added to the tree surface | #63 | `345c7d35` | verified: absent at parent, 1 occurrence(s) after |
| #522 | evidence-keyed paint table added | #392 | `2dae2a5b` | verified: absent at parent, 1 occurrence(s) after |
| #522 | comment claiming the ramp is not the paint added | #392 | `2dae2a5b` | verified: absent at parent, 1 occurrence(s) after |
| #522 | paint table fed from the rank band | #485 | `1ecfc997` | verified: absent at parent, 2 occurrence(s) after |
| #523 | IPC report deserialised with no version negotiation | #144 | `9cec1e82` | verified: absent at parent, 1 occurrence(s) after |
| #523 | wire field renamed, firing the unversioned parse | #485 | `1ecfc997` | verified: absent at parent, 1 occurrence(s) after |
| #524 | compare-against-canonical removed | #485 | `1ecfc997` | verified: 40 occurrence(s) at parent, 4 after |
| #524 | per-row two-step selection added | #485 | `1ecfc997` | verified: absent at parent, 2 occurrence(s) after |
| #524 | test pinning the removed commands added | #485 | `1ecfc997` | verified: absent at parent, 3 occurrence(s) after |
| #525 | two-decimal helper added | #420 | `42b2c928` | verified: absent at parent, 2 occurrence(s) after |
| #525 | integer mass routed through the two-decimal helper | #485 | `1ecfc997` | verified: absent at parent, 6 occurrence(s) after |
| #525 | wire field became an integer | #485 | `1ecfc997` | verified: absent at parent, 1 occurrence(s) after |

## gh #520 — measured detector behaviour

One frozen copy of `site/tests` scanned by a release build of each commit; `mixed` counts published clusters whose occurrences do not all span the same number of lines.

```
fc779f7b OK mixed=2/24 clusters=24 top_occ=2 top_spans=[7] pct=14.31159420289855 :: Add a pinned real-repository corpus gate and reorien
a3fe320b OK mixed=2/24 clusters=24 top_occ=2 top_spans=[7] pct=14.31159420289855 :: Content-gate fused confidence so shape-only families
170d3606 OK mixed=2/24 clusters=24 top_occ=2 top_spans=[7] pct=14.31159420289855 :: Content-gate fused confidence on rename evidence and
f92300e5 OK mixed=2/27 clusters=27 top_occ=2 top_spans=[7] pct=15.217391304347828 :: Fused confidence: honest cluster signals, content ga
7e61e379 OK mixed=2/27 clusters=27 top_occ=2 top_spans=[7] pct=15.217391304347828 :: Persist parse and signature work across runs, and fi
2dae2a5b OK mixed=3/32 clusters=32 top_occ=2 top_spans=[7] pct=16.213768115942027 :: Fused-score follow-ups: offset-invariant sibling-win
8fb1b15c OK mixed=3/32 clusters=32 top_occ=2 top_spans=[7] pct=16.213768115942027 :: Diff-aware duplication scanning: --diff / --only-cha
42b2c928 OK mixed=13/39 clusters=39 top_occ=3 top_spans=[5] pct=20.742753623188406 :: Measure structural as subtree overlap: close the fiv
b235c1a5 OK mixed=4/11 clusters=11 top_occ=2 top_spans=[5, 7] pct=6.61231884057971 :: Fused-score accuracy follow-ups: verbatim subgroups,
c3ce7882 OK mixed=4/11 clusters=11 top_occ=2 top_spans=[5, 7] pct=6.61231884057971 :: Corpus-scale pipeline performance: streamed banding,
1dac359b OK mixed=4/11 clusters=11 top_occ=2 top_spans=[5, 7] pct=6.61231884057971 :: Stop the content gate publishing an unmeasured token
45858fa8 OK mixed=4/11 clusters=11 top_occ=2 top_spans=[5, 7] pct=6.61231884057971 :: Container election, LSP lifecycle exit codes, and co
7d6d6996 OK mixed=4/11 clusters=11 top_occ=2 top_spans=[5, 7] pct=6.61231884057971 :: Fix the content-gate verdict lie, the sibling-cell f
d9f06d0c OK mixed=2/11 clusters=11 top_occ=2 top_spans=[2] pct=5.344202898550725 :: Replace cluster fused confidence with elected-pair e
1ecfc997 OK mixed=2/5 clusters=5 top_occ=2 top_spans=[11, 12] pct=4.438405797101449 :: Render no pair-only evidence on cluster surfaces; ad
bb17d9cb OK mixed=2/5 clusters=5 top_occ=2 top_spans=[11, 12] pct=4.438405797101449 :: Refuse one-sided echo rescues, collapse straddling w
b5273c16 OK mixed=2/5 clusters=5 top_occ=2 top_spans=[11, 12] pct=4.438405797101449 :: Resolve subsumption per file set as a published-set 
eedfe5dd OK mixed=2/7 clusters=7 top_occ=2 top_spans=[11, 12] pct=5.88768115942029 :: Same-file pairs pay the same content floor, and four
1c2b9110 OK mixed=2/7 clusters=7 top_occ=2 top_spans=[11, 12] pct=5.88768115942029 :: Bump fast-uri from 3.1.2 to 3.1.7 in /clients/vscode
d5509f64 OK mixed=2/7 clusters=7 top_occ=2 top_spans=[11, 12] pct=5.88768115942029 :: Spell every published path with a forward slash, and
54d06fb2 OK mixed=2/7 clusters=7 top_occ=2 top_spans=[11, 12] pct=5.88768115942029 :: Read the archives we ship without an external tool, 
8324e479 OK mixed=9/30 clusters=30 top_occ=18 top_spans=[2, 3, 5] pct=21.920289855072465 :: Independent clone registers, a corpus accuracy gate,
77b4681b OK mixed=9/31 clusters=31 top_occ=18 top_spans=[2, 3, 5] pct=22.01086956521739 :: Find duplicated code inside a single file (#508)
```

## gh #526 — identifiers cited in code but defined in no document

Orphaned identifiers: **36**.

| identifier | production files citing it | first cited by | document dropped by |
| --- | --- | --- | --- |
| `[AUTOFIX-CONSOLIDATE-CODE-ACTION]` | 0 | #485 | never documented |
| `[BRACKETED-ID]` | 0 | #424 | never documented |
| `[CLI-TEXT]` | 0 | #503 | never documented |
| `[CLONE-NOISE-DART-WIDGET-SCAFFOLD]` | 1 | #341 | never documented |
| `[CLONE-NOISE-EMBEDDING-ROLE]` | 1 | #485 | never documented |
| `[CLONE-NOISE]` | 1 | #424 | never documented |
| `[CORPUS-SCORE-GATE]` | 0 | #518 | never documented |
| `[DESLOP-LIVE]` | 0 | #400 | never documented |
| `[FACET-GROUP-BY-SEVERITY]` | 0 | #485 | never documented |
| `[FUSED-RANK-MASS]` | 0 | #485 | never documented |
| `[LANG-CAND-DART]` | 0 | #161 | #368 |
| `[LANG-CAND-GO]` | 1 | #311 | #368 |
| `[LANG-CAND-JAVASCRIPT]` | 3 | `862ea369` | #368 |
| `[LANG-CAND-KOTLIN]` | 0 | #311 | #368 |
| `[LANG-CAND-TYPESCRIPT]` | 2 | `862ea369` | #368 |
| `[LOCATION-LINE-COLUMN]` | 1 | #485 | never documented |
| `[LSP-CLI-HELP]` | 2 | #479 | never documented |
| `[LSP-SEVERITY-BAND]` | 1 | #485 | never documented |
| `[MCP-ROOT-CANONICAL]` | 0 | #485 | never documented |
| `[MCP-TOOLS-FIND-SIMILAR]` | 0 | #485 | never documented |
| `[PARSE-FSHARP-NORMALIZE]` | 1 | #272 | #368 |
| `[PARSE-PHP-NORMALIZE]` | 1 | #265 | never documented |
| `[PERF-SAMPLE]` | 0 | #484 | never documented |
| `[PIPELINE-SIGNATURE-FALLBACK]` | 0 | `c3ce7882` | never documented |
| `[PRINCIPLES-LOGGING]` | 4 | #420 | never documented |
| `[RANK-SCORE]` | 0 | #311 | #420 |
| `[REPAIR-COSINE-MERGE]` | 2 | #368 | #484 |
| `[REPAIR-RENAME-ANCHOR-MASS]` | 1 | #420 | #485 |
| `[REPORTING-CONTEXT]` | 2 | #392 | #485 |
| `[SKIP-BREAKING-CI]` | 0 | #424 | never documented |
| `[SPEC-ID]` | 0 | #424 | never documented |
| `[TEST-ONE-BINARY]` | 0 | #424 | never documented |
| `[TESTS-NO-INDEXING]` | 0 | #36 | never documented |
| `[VSIX-PAIR-COMPARE]` | 4 | #485 | never documented |
| `[VSIX-REACTIVITY-DIRTY]` | 1 | #420 | never documented |
| `[VSIX-SETTINGS-RANKING]` | 4 | #202 | #485 |
