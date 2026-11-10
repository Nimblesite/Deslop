# Rust workspace tests

Command: `env -u RUST_LOG CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 timeout 1200 nice -n 15 make test`

Exit status: **0**.

Source: `ba05c404c861ff7a32d887a3262291ac3365ba3f`. Log SHA-256: `53f132f8e35696e68f31914f4903736bd8ecffeeab61ded97aa983b3d520ecf9`.

1797 passed; 0 failed; 17 ignored across 15 test binaries. Ignored tests remain incomplete work; this is the standard workspace gate.

The earlier run inherited `RUST_LOG=warn`, which suppressed the debug worker-count evidence required by the jobs test. The unchanged suite passes with that shell override removed.

```text
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 12 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 624 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 127.50s
test result: ok. 367 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.25s
test result: ok. 228 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.11s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.03s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 98 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 125.94s
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 149 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 23.15s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 219 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.97s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
```
