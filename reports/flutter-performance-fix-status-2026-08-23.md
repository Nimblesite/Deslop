# Flutter analyzer performance fix status

Date: 2026-08-23 (Australia/Sydney)

## Conclusion

The Flutter performance problems are **not fixed**. The branch contains partial mitigations, but the latest clean release-binary run still reached the 20-minute cutoff without producing a report, cluster count, or completion status.

The available evidence is:

- the forensic baseline in [the Flutter performance plan](../docs/plans/flutter-analyzer-performance-report.md);
- the latest clean-run result in [flutter-corpus-2026-08-23.md](flutter-corpus-2026-08-23.md);
- current source and the outstanding checklist in [BRANCH_REVIEW.md](../docs/BRANCH_REVIEW.md).

No raw analyzer log from the latest run is present in the workspace. Therefore, this report does not claim a current-stage timing, current peak RSS, or current pair count that the run did not emit.

## Status by performance issue

| Issue from the plan | Current status | Evidence | Assessment |
|---|---|---|---|
| 927.9s serial corpus/signature build | Not fixed / not re-measured | Current run timed out; corpus construction remains an ordinary serial path. | The code adds counters and a bounded signature memo, but there is no completed current measurement proving the stage is under budget. |
| Language-aware token/range work for fingerprints | Not proven fixed | `signature_for_fingerprint` still routes known-language fingerprints through `token_stream_for_fingerprint_with_language`; see `pipeline/signatures.rs`. | The memo can reduce repeated MinHash construction, but it does not remove repeated range resolution/token traversal or establish that the regression is gone. |
| 55.3m LSH pair fan-out | Partially fixed in code | `lsh.rs` now uses a star topology per bucket and sorts/deduplicates the resulting pairs. | This directly addresses the old quadratic bucket expansion, but the complete pair vector is still returned and retained. The latest run emitted no replacement cardinality. |
| Shared-subtree rescue taking 13+ minutes | Not fixed | `apply_shared_subtree_rescue` still iterates over every candidate pair; no hard candidate/work budget is present. | The bound shortcut and memoized overlap paths may reduce individual alignments, but the rescue population is still data-dependent and unbounded. |
| One-core/serial execution | Not fixed | The rescue loop is a normal sequential `for pair in pairs.iter_mut()` loop; corpus construction is also serial. | No current completed run demonstrates acceptable wall time. Parallelism was correctly not treated as a sufficient fix, but no bounded replacement exists. |
| Per-measurement debug logging | Fixed as an observability issue | `RescueTally` reports every 50,000 measured pairs and emits one aggregate completion record. | This removes the 793,076-record hot-loop log flood, but it does not reduce the underlying rescue work. |
| Stage-level corpus/rescue counters | Partially fixed | Corpus stats record read/parse/fingerprint/signature timings; rescue tally records scanned/eligible/measured/rescued counts. | LSH/candidate and peak-retained-population reporting are still incomplete, and the latest run did not preserve logs containing these counters. |
| Memory peak over 7,168 MiB | Not fixed / current value unknown | The plan's baseline reached 14,624.9 MB. Current run report has no peak-memory measurement. `BRANCH_REVIEW.md` still says full pair/candidate populations are retained. | No evidence shows the 7,168 MiB ceiling is now met. The bounded signature memo is only one bounded allocation; it does not bound pairs, candidates, trees, or rescue state. |
| Full pipeline completion and report rendering | Not fixed | Latest clean run was killed at 20 minutes before final output. | Clustering, ranking, rendering, cluster count, and duplication metrics remain unverified. |

## What has actually improved

1. LSH bucket expansion was changed from all pair combinations to a canonical-member star topology, followed by sorting and deduplication. This is a credible fix for the specific old 55-million-pair fan-out mechanism, subject to accuracy validation.
2. Rescue logging was changed from one debug record per overlap to fixed-interval aggregate progress records. This fixes diagnostic-log amplification, not rescue computation.
3. Corpus-stage counters now separate read, parse/normalise, fingerprint, and signature time and distinguish exact-node from sibling-window fingerprints.
4. Signature construction has a per-pass memo with a 262,144-entry cap. This bounds that memo, but not the rest of the retained analysis state.

## What remains open

The current branch review explicitly leaves the central Flutter work incomplete: it still calls out serial corpus construction, full pair/candidate retention, and shared-subtree rescue over a data-dependent population. Its acceptance criteria still require a controlled cold run under 10 minutes and 7,168 MiB.

The latest run fails the most basic acceptance condition: it did not complete within the 20-minute operational cutoff. Because it emitted no final metrics, it cannot demonstrate accuracy preservation, determinism, scope, cluster counts, or report identity either.

## Bottom line

The old logging problem and the worst LSH bucket-expansion shape appear to have been addressed in source. The analyzer's end-to-end performance problem has not been resolved: the current implementation still lacks bounded candidate/rescue work and a demonstrated memory budget, and the clean Flutter corpus still does not finish in time.

The next valid performance claim requires a controlled run that records exact binary/corpus provenance, stage timings, LSH and candidate cardinalities, rescue counters, peak RSS, completion status, and the resulting report artifact.
