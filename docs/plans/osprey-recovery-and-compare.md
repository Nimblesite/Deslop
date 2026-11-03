# Osprey recovery and comparison responsiveness

## [LANG-CAND-OSPREY-RECOVERY]

An unfinished declaration must not remove findings for intact neighboring functions. Reproduce through the CLI against the existing Osprey ML fixture, asserting the exact clone, both paths, source ranges, occurrence count, bucket and rank. Follow the accuracy quarantine in `AGENTS.md` before implementing recovery; accepting error-containing ASTs indiscriminately is not a grammar fix.

- [x] Locate the whole-file rejection in `crates/deslop-core/src/lang/osprey.rs`.
- [x] Confirm the new black-box test fails against the original implementation: expected one intact clone cluster, reported zero.
- [x] Quarantine the defective rejection and confirm the test fails at the quarantine; the clean baseline passes the same cluster, range, bucket and rank assertions first.
- [ ] Obtain acknowledgment required by the accuracy quarantine and fix-bug skill.
- [ ] Repair the upstream Osprey grammar with construct-specific fixtures and update the pin.
- [ ] Implement recovery without inventing matches inside erroneous syntax.
- [ ] Verify the real LSP edit/recovery cycle and run required gates.

## [VSIX-PAIR-COMPARE]

`clients/vscode/src/commands/compare.ts::openCompareDiff` awaits `measurePair` before invoking `vscode.diff`; the client request in `clients/vscode/src/compare/title.ts` has no bounded wait. Pair evidence therefore blocks the source comparison. The comparison should show both occurrence ranges while the engine measurement remains pending, then show the engine's exact pair verdict without losing the selected ranges.

Completed duplication analysis does not avoid this request: `PipelineSession::measure_pair` reparses the two files, regenerates candidate anchors across the workspace through `comparison_anchor_pairs`, constructs fresh measurement indices, and requests fresh embeddings when a provider is active. These are confirmed code paths; their individual contributions to the observed latency have not yet been measured.

- [x] Identify the engine request on the diff opening path.
- [ ] Reproduce comparison responsiveness with deterministic synchronization and the real engine.
- [ ] Confirm the failure with the user as required by the invoked skill.
- [ ] Implement and verify responsive comparisons and truthful verdicts.
