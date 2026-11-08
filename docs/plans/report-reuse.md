# [OUTPUT-REPORT-REUSE] Report generation without repeated analysis

- [x] Reproduce unchanged CLI exports restarting analysis with a populated duplicate fixture.
- [x] Obtain acknowledgment of the failing test before implementation.
- [x] Verify repeated real LSP HTML/JSON exports preserve the snapshot and generation.
- [x] Implement a bounded completed-report cache validated against source bytes and analysis inputs.
- [x] Preserve fingerprint-store assertions by explicitly evicting the upper cache in store-isolation helpers.
- [x] Document snapshot telemetry, invalidation, and cache opt-outs in [the CLI specification](../specs/cli.md).
- [x] Verify membership, same-mtime edits, policy changes, corrupted entries, and output-path reuse.
- [ ] Pass static analysis and the complete required test suite.
- [x] Record reproducible results, commit, and remove temporary build/review files.

Report-specific verification passes, including the real VS Code button. Workspace release Clippy passes. The full test and coverage gates remain red on pre-existing Osprey ML quarantine cases; the installed server also reproduces the separate pair-comparison rediscovery failure. See [baseline evidence](../../reports/tests/report-reuse-baseline-blockers.md); these assertions and the quarantine remain intact.
