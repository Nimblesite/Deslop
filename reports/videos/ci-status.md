# Full CI gate [DOC-FILMS-VALIDATION]

- Command: `make ci`
- Captured log modified: 2026-11-10T10:46:05.473Z
- Log SHA-256: 607730946ddacd450089dd1abfd2e11b8e54618496f6d5fa5e1d6f8cdb48346a
- Result: test result: FAILED. 626 passed; 4 failed; 4 ignored; 0 measured; 0 filtered out; finished in 96.65s
- Failing tests: 4

This is the captured full CI run, not a green gate for the film changes. Website build and browser validation are recorded separately. The PR submission skill requires the full gate to pass.

## Failure output

```text
failures:

---- cli::jobs::every_parallel_stage_stays_within_the_jobs_budget stdout ----

thread 'cli::jobs::every_parallel_stage_stays_within_the_jobs_budget' (10952542) panicked at crates/deslop/tests/cli/jobs.rs:109:9:
--jobs 1: the fingerprint stage must report its workers; stderr: deslop scanning /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpQMwWF3/workload for duplicated code...
  report → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmptvfmC4/report.{json,txt,html}
  log    → stderr (--log-to-console)

Found 1 groups of duplicated code across 48 file(s)
  diagnostic severity: 1 × warning
  Highest mass: 1932 from 24 occurrences × 84 canonical nodes in copy_00.py, copy_01.py, copy_02.py (+21 more)

Top 1 findings
  5885cb2 Identical code — rank #1, mass 1932, 24 occurrences in copy_00.py, copy_01.py, copy_02.py (+21 more)

Next: open the .html report to inspect occurrences or explicitly compare two endpoints.

✔ done
    report /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmptvfmC4/report.json
    report /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmptvfmC4/report.txt
    report /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmptvfmC4/report.html

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- rust_escaped_string_literals::a_copied_string_is_reported_without_the_unrelated_ones stdout ----

thread 'rust_escaped_string_literals::a_copied_string_is_reported_without_the_unrelated_ones' (11005732) panicked at crates/deslop/tests/rust_escaped_string_literals.rs:108:5:
assertion `left == right` failed: the copy group holds exactly the original and its copy: [["help.rs", "run_output.rs", "usage.rs"]]
  left: 3
 right: 2

---- rust_escaped_string_literals::unrelated_escaped_strings_are_never_grouped stdout ----

thread 'rust_escaped_string_literals::unrelated_escaped_strings_are_never_grouped' (11005824) panicked at crates/deslop/tests/rust_escaped_string_literals.rs:56:9:
unrelated strings only: run_output.rs holds a string unrelated to every other one, yet it was grouped: [["run_output.rs", "usage.rs"]]

---- spec_id_traceability::every_cited_rule_identifier_resolves_to_a_written_rule stdout ----

thread 'spec_id_traceability::every_cited_rule_identifier_resolves_to_a_written_rule' (11013694) panicked at crates/deslop/tests/spec_id_traceability.rs:73:5:
1 rule identifier(s) are cited in code or tests and defined in no document under docs/. Give each one a section carrying its identifier, rename the citation to the identifier it meant, or delete the citation with the code it describes:
  [TESTING-CLI-RUN-SCRATCH]
      /Users/christianfindlay/Documents/Code/Deslop/crates/deslop/tests/fixtures/rust-escaped-string-literals/run_output.rs:1


failures:
    cli::jobs::every_parallel_stage_stays_within_the_jobs_budget
    rust_escaped_string_literals::a_copied_string_is_reported_without_the_unrelated_ones
    rust_escaped_string_literals::unrelated_escaped_strings_are_never_grouped
    spec_id_traceability::every_cited_rule_identifier_resolves_to_a_written_rule

test result: FAILED. 626 passed; 4 failed; 4 ignored; 0 measured; 0 filtered out; finished in 96.65s

error: test failed, to rerun pass `-p deslop --test suite`
error: process didn't exit successfully: `/Users/christianfindlay/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo test --release --manifest-path /Users/christianfindlay/Documents/Code/Deslop/Cargo.toml --target-dir /Users/christianfindlay/Documents/Code/Deslop/target/llvm-cov-target --workspace --all-targets --features deslop-core/live,deslop-lsp/profiling` (exit status: 101)
make[3]: *** [_coverage-run] Error 101
make[2]: *** [_ci-test-rust] Error 2
make[1]: *** [_ci-test] Error 2
make: *** [ci] Error 2
```
