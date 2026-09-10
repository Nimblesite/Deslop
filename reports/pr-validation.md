# PR validation — in progress

Generated at `2026-09-10T10:50:02+00:00` by `target/deslop-astra1-merge/generate-review-report.py` from captured command logs and CLI JSON. HEAD: `e88e56a70babb623563632d7acf00c0860be2255`, with uncommitted review changes identified below.

Publication remains blocked by the two newly ignored accessor contracts, which require reporting the pair held CLEARLY OUT in the accuracy register. Their explicit failures are included below. The corrected expectation is pending approval under the instruction against removing assertions. The final full CI run must follow all source changes.

## Executed Rust checks

| Check | Tool result | Log |
|---|---|---|
| Shared-statement pin before repair | test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 224 filtered out; finished in 0.01s | [wrapped-span-before.log](../target/deslop-astra1-merge/wrapped-span-before.log) |
| Shared-statement and companion pins after repair | test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 222 filtered out; finished in 0.01s | [wrapped-span-final.log](../target/deslop-astra1-merge/wrapped-span-final.log) |
| Canonical fingerprint contracts | test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 222 filtered out; finished in 0.00s | [fingerprint-after.log](../target/deslop-astra1-merge/fingerprint-after.log) |
| Complete core library | test result: ok. 226 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 104.11s | [core-lib-after.log](../target/deslop-astra1-merge/core-lib-after.log) |
| Production scan/comparison parity | test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 224 filtered out; finished in 0.14s | [opus-core-proof/caller-parity-final.log](../target/deslop-astra1-merge/opus-core-proof/caller-parity-final.log) |
| Both original and stricter Python contract floors | test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 496 filtered out; finished in 0.86s | [inherited-contract-both-floors.log](../target/deslop-astra1-merge/inherited-contract-both-floors.log) |
| Newly ignored accessor contracts, explicitly executed | test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 495 filtered out; finished in 0.84s | [ignored-contract-tests.log](../target/deslop-astra1-merge/ignored-contract-tests.log) |

## Shared-core reproduction

The same Python fixtures were scanned at `--min-nodes 30 --embeddings off --no-incremental`. The baseline binary is the verified `f92300e5` build documented in [registry baseline attribution](registry-baseline-attribution.md). The pre-repair and repaired JSON are under `target/deslop-astra1-merge/wrapped-span-measurement/`.

| Fixture | Engine | Exit | Clusters |
|---|---|---:|---:|
| one_statement | baseline | 0 | 0 |
| one_statement | current | 0 | 0 |
| two_statements | baseline | 0 | 0 |
| two_statements | current | 0 | 1 |
| one_statement | repaired | 0 | 1 |
| two_statements | repaired | 0 | 1 |

This fixture does not prove a loss of reported clones relative to f92300e5: the baseline also misses the single-statement case. The unit failure pins the byte-range collision in the newly introduced aligned-core index. The repair keeps the canonical hash walk and uses exact node identity. The separate caller-parity suspicion was not reproduced through production scan/comparison paths; no production repair was made there.

## Additional checks

All-target, all-feature core Clippy output:

```text
Checking notify-types v2.1.0
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/DeslopWorktree/crates/deslop-core)
   Compiling clap_derive v4.6.4
    Checking notify v8.2.0
    Checking clap v4.6.6
    Checking deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/DeslopWorktree/crates/deslop-test-support)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.84s
```

**PASS** — every scored repository is inside its gate.

Corpus measurements: [generated scorecard](../.corpus/score-gate/SCORE.md).

Deslop similarity queries completed: 7. Matches and generations are recorded in `target/deslop-astra1-merge/identity-similarity.json`. The resolver match is its existing thin prefix calling the canonical range resolver; the changed indexing implementation is shared through the fingerprint visitor.

## Source snapshot

| Source | SHA-256 |
|---|---|
| `crates/deslop-core/src/fingerprint.rs` | `ad65f0909c6cb9527d609a11e4e14d22c6e76b81cc186b539d7fa01938f23796` |
| `crates/deslop-core/src/overlap/core.rs` | `c0545ff387011d2a23e454cd4a834c14c0fd73e16053f6605ddb39a7199736a0` |
| `crates/deslop-core/src/overlap/core/tests/wrapped_spans.rs` | `b51c3d7afb771b04c5b2ac144e27cf13924640e78737879f335dd4ed012a0229` |
| `crates/deslop-core/src/overlap/core/tests/caller_parity.rs` | `24b7db35af634474e83169cb9da4e7e63f2ede8ad47a942e8e02314c31fdccf3` |
