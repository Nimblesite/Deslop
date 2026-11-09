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

Report-specific verification includes the real VS Code button. The Osprey ML recovery and pair-comparison rediscovery failures recorded in [the historical baseline](../../reports/tests/report-reuse-baseline-blockers.md) were subsequently fixed and verified in [the recovery and comparison plan](osprey-recovery-and-compare.md). Static analysis and the unchanged Osprey recovery test pass on the upstream-main grammar pin; the full current-commit suite and coverage gates remain pending in [the CI checklist](ci-prep-2026-11-10.md). Do not interpret the historical failure output as the current result or mark the full-suite gate complete from focused tests.
