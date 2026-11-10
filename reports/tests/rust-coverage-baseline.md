# Rust coverage collection baseline

Source baseline: `a66cbaa52916e9a0c01186e709efa358faa1d4ed` (before the CLI summary changes).

Command: `env -u RUST_LOG CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 timeout 1800 nice -n 15 make _coverage-run`

Observed process exit: 0.

Test binaries reporting success: 15. Passed: 1797; failed: 0; ignored: 17.

Collection log SHA-256: `23db3ce5b1820c3bf9ebe04c1f9bf6889719db320aa3077e97a367afa0aee851`.

This is collection evidence only. It does not establish coverage thresholds or validate subsequent CLI renderer changes. Recollect current source before the final coverage report.
