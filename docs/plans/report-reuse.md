# [OUTPUT-REPORT-REUSE] Report generation without repeated analysis

- [x] Reproduce unchanged CLI exports restarting analysis with a populated duplicate fixture.
- [x] Obtain acknowledgment of the failing test before implementation.
- [x] Verify repeated real LSP HTML/JSON exports preserve the snapshot and generation.
- [x] Implement a bounded completed-report cache validated against source bytes and analysis inputs.
- [x] Preserve fingerprint-store assertions by explicitly evicting the upper cache in store-isolation helpers.
- [x] Document snapshot telemetry, invalidation, and cache opt-outs in [the CLI specification](../specs/cli.md).
- [ ] Verify membership, same-mtime edits, policy changes, corrupted entries, and output-path reuse.
- [ ] Pass static analysis and the complete required test suite.
- [ ] Record reproducible results, commit, and remove temporary build/review files.
