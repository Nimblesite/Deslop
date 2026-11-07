# Osprey recovery and comparison responsiveness

## [LANG-CAND-OSPREY-RECOVERY]

An unfinished declaration must not remove findings for intact neighboring functions. Reproduce through the CLI against the existing Osprey ML fixture, asserting the exact clone, both paths, source ranges, occurrence count, bucket and rank. Follow the accuracy quarantine in `AGENTS.md` before implementing recovery; accepting error-containing ASTs indiscriminately is not a grammar fix.

- [x] Locate the whole-file rejection in `crates/deslop-core/src/lang/osprey.rs`.
- [x] Confirm the new black-box test fails against the original implementation: expected one intact clone cluster, reported zero.
- [x] Quarantine the defective rejection and confirm the test fails at the quarantine; the clean baseline passes the same cluster, range, bucket and rank assertions first.
- [x] Obtain acknowledgment required by the accuracy quarantine and fix-bug skill (user goal, 2026-11-08).
- [ ] Repair the upstream Osprey grammar with construct-specific fixtures and update the pin. Blocker: the ML scanner (`tree-sitter-osprey-ml/src/scanner.c`, `skip_layout_space` + `emit`) places the zero-width `_dedent` after every following blank line, so `describe` spans `[0..85]` when a blank line and another declaration follow it, but `[0..84]` at end of file. `osprey_recovery` stays red on that byte until the scanner ends layout tokens at the first line break. The Osprey repo is owned by its own agents, so the grammar fix is theirs.
- [x] Implement recovery without inventing matches inside erroneous syntax: `osprey.rs::intact_root` keeps only error-free top-level declarations; a file with no intact declaration stays a visible skip.
- [ ] Verify the real LSP edit/recovery cycle and run required gates.

## [VSIX-PAIR-COMPARE]

`clients/vscode/src/commands/compare.ts::openCompareDiff` awaits `measurePair` before invoking `vscode.diff`; the client request in `clients/vscode/src/compare/title.ts` has no bounded wait. Pair evidence therefore blocks the source comparison. The comparison should show both occurrence ranges while the engine measurement remains pending, then show the engine's exact pair verdict without losing the selected ranges.

Completed duplication analysis does not avoid this request: `PipelineSession::measure_pair` reparses the two files, regenerates candidate anchors across the workspace through `comparison_anchor_pairs`, constructs fresh measurement indices, and requests fresh embeddings when a provider is active. These are confirmed code paths; their individual contributions to the observed latency have not yet been measured.

- [x] Identify the engine request on the diff opening path.
- [x] Reproduce repeated discovery with the real engine after report completion: `completed_discovery_is_reused_by_repeated_pair_comparisons` asserts the engine's endpoint echoes and admitted verdict for three requests, including reversal, then fails because discovery ran four times instead of once.
- [ ] Confirm the failure with the user as required by the invoked skill.
- [ ] Implement and verify responsive comparisons and truthful verdicts.
