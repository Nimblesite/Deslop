# PR validation — local gates

Generated at `2026-09-10T11:55:23+00:00` by `target/deslop-astra1-merge/generate-review-report.py` from captured command logs and CLI JSON. HEAD: `bf338e96c7bce1f7d803f1f8c7286d33acceeb5a`; source hashes below identify the measured review implementation.

Latest full `make ci`: exit `0`, source unchanged `True`, completed `2026-09-10T11:46:51+00:00`. Final coverage output is recorded below.

The approved accessor-contract correction is applied. Both tests run without skips and reject the pair held CLEARLY OUT in the accuracy register. They retain the measured pair evidence and exact positive-control assertions. The skip-policy registry and plan now agree with the active tests. This report records local validation before PR submission.

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
| Accessor correction before application: isolated harness | test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.12s | [core-proposal/proposal-test-final.log](../target/deslop-astra1-merge/core-proposal/proposal-test-final.log) |
| Re-enabled accessor contracts in the live tree | test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 493 filtered out; finished in 0.72s | [approved-accessor-tests.log](../target/deslop-astra1-merge/approved-accessor-tests.log) |
| Wrapper-callee pin before repair: saved owner tool output | test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 226 filtered out; finished in 0.00s | [wrapper-owner-test-0.log](../target/deslop-astra1-merge/wrapper-owner-test-0.log) |
| Wrapper-callee filter suite after repair: saved owner tool output | test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 214 filtered out; finished in 0.01s | [wrapper-owner-test-1.log](../target/deslop-astra1-merge/wrapper-owner-test-1.log) |

## Completed CI: `make-ci-final`

`make ci` exited `0` at `2026-09-10T11:01:23+00:00`. Source unchanged during execution: `False`. A passing command with source changes does not certify the current tree.

Changed during the run:

- `crates/deslop-core/src/cluster_filters/calls.rs`
- `crates/deslop-core/src/cluster_filters/calls/args.rs`
- `crates/deslop-core/src/cluster_filters/calls/callee.rs`
- `crates/deslop-core/src/cluster_filters/calls/dataflow.rs`
- `crates/deslop-core/src/cluster_filters/calls/tests.rs`

Completed test results:

```text
  497 passing (33s)
  1 passing (73ms)
  9 passed (2.5s)
  3 passed (1.8s)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 12 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 491 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 20.11s
test result: ok. 239 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.37s
test result: ok. 193 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.48s
test result: ok. 62 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 21.60s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 126 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.78s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 136 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```


## Completed CI: `make-ci-handoff`

`make ci` exited `0` at `2026-09-10T11:18:19+00:00`. Source unchanged during execution: `False`. A passing command with source changes does not certify the current tree.

Changed during the run:

- `crates/deslop-core/src/cluster_filters/calls.rs`
- `crates/deslop-core/src/cluster_filters/calls/args.rs`
- `crates/deslop-core/src/cluster_filters/calls/callee.rs`

Completed test results:

```text
  497 passing (33s)
  1 passing (67ms)
  9 passed (2.6s)
  3 passed (2.0s)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 12 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 491 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 18.36s
test result: ok. 240 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.26s
test result: ok. 193 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.76s
test result: ok. 62 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 21.44s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 126 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 136 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```


## Completed CI: `make-ci-frozen`

`make ci` exited `2` at `2026-09-10T11:18:45+00:00`. Source unchanged during execution: `True`. A passing command with source changes does not certify the current tree.

Changed during the run:


Completed test results:

```text
```


## Completed CI: `make-ci-verified`

`make ci` exited `0` at `2026-09-10T11:33:31+00:00`. Source unchanged during execution: `True`. A passing command with source changes does not certify the current tree.

Changed during the run:


Completed test results:

```text
  497 passing (32s)
  1 passing (57ms)
  9 passed (2.5s)
  3 passed (4.4s)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 12 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 491 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 19.55s
test result: ok. 240 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.70s
test result: ok. 193 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.06s
test result: ok. 62 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 21.61s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 126 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.35s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 136 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```


## Completed CI: `make-ci-approved`

`make ci` exited `0` at `2026-09-10T11:46:51+00:00`. Source unchanged during execution: `True`. A passing command with source changes does not certify the current tree.

Changed during the run:


Completed test results:

```text
  497 passing (35s)
  1 passing (57ms)
  9 passed (2.2s)
  3 passed (1.6s)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 12 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 493 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 19.61s
test result: ok. 240 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.17s
test result: ok. 193 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.25s
test result: ok. 62 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 21.22s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 126 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.95s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 136 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Coverage enforcement

Both coverage logs captured after the final unchanged-source CI run: `True`.

Rust coverage enforcement output:

```text
==> Coverage calculation and threshold enforcement...

    Finished report saved to lcov.info
  deslop-core    95.2% (threshold 95% + 1% slack) OK
  deslop         95.9% (threshold 94% + 1% slack) OK
  deslop-lsp     95.9% (threshold 95% + 1% slack) OK
  deslop-mcp     97.8% (threshold 93% + 1% slack) OK
Workspace total: 93.6% (26122/27902 lines)
```

Extension and webview coverage enforcement output:

```text
cd clients/vscode && npm run coverage:extension:check

> deslop-live@0.0.0-dev coverage:extension:check
> node ./scripts/check-coverage.mjs --extension

Extension-host line coverage: 88.7% (threshold: 88% + 1% slack)
OK: 88.7% + 1% slack >= 88%
cd clients/vscode && npm run coverage:webview:check

> deslop-live@0.0.0-dev coverage:webview:check
> node ./scripts/check-coverage.mjs --webview

Webview line coverage: 96.5% (threshold: 95% + 1% slack)
OK: 96.5% + 1% slack >= 95%
```


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

## Existing baseline defects

AST inspection found 4 ignored contracts in the selected baseline files. Their bytes were checked against `f92300e5e1004ef6c53a94174a0d7e842232ec80`. Their ignore reasons already describe the embedding defects; these remain outside the requested post-baseline repair scope.

| Baseline test | Source SHA-256 |
|---|---|
| `mid_band_cluster_confidence_never_exceeds_its_strongest_axis` | `ca448fedd3ad76d722482afd6da86928e96e23c080f60fc45f1a7cf8e40d61c4` |
| `lsp_embedding_refresh_is_bounded_and_reproducible` | `3c9dce62b946d63bdcd199416a4c99652020e1fed80c1d1987940a9081b8b45f` |
| `embeddings_on_reports_every_file_set_embeddings_off_reported` | `9d001efe1b264220e9cada0f7f0ef1e95a5d481d0b86250d723ad99d22ba4774` |
| `embeddings_on_never_moves_a_reported_bucket` | `9d001efe1b264220e9cada0f7f0ef1e95a5d481d0b86250d723ad99d22ba4774` |

Exact paths and original ignore reasons: `target/deslop-astra1-merge/baseline-skipped-contracts.json`.

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

## Assertion similarity attribution

Both binaries scanned the same current source with embeddings off, min-nodes 30, and incremental caching disabled. The baseline already groups the inner tuples of these two assertions. The current match spans the surrounding assertion syntax. The assertions check disjoint facts and already share the canonical `field` accessor; no additional extraction was made.

| Build | Cluster | Classification | Source spans |
|---|---|---|---|
| f92300e5 | `131a3472c873d8c7` | structural_only | `crates/deslop/tests/common/seeded.rs:8036..8443`; `crates/deslop/tests/content_gate_signal_honesty.rs:11104..11490` |
| current | `c98c950cc4a62edc` | loosely_similar | `crates/deslop/tests/common/seeded.rs:8012..8666`; `crates/deslop/tests/content_gate_signal_honesty.rs:11078..11585` |

Raw measurements: `target/deslop-astra1-merge/baseline-self-scan.json`, `final-self-scan.json`, and `approved-similarity.json`.

## Source snapshot

| Source | SHA-256 |
|---|---|
| `crates/deslop-core/src/fingerprint.rs` | `ad65f0909c6cb9527d609a11e4e14d22c6e76b81cc186b539d7fa01938f23796` |
| `crates/deslop-core/src/overlap/core.rs` | `c0545ff387011d2a23e454cd4a834c14c0fd73e16053f6605ddb39a7199736a0` |
| `crates/deslop-core/src/overlap/core/tests/wrapped_spans.rs` | `b51c3d7afb771b04c5b2ac144e27cf13924640e78737879f335dd4ed012a0229` |
| `crates/deslop-core/src/overlap/core/tests/caller_parity.rs` | `24b7db35af634474e83169cb9da4e7e63f2ede8ad47a942e8e02314c31fdccf3` |
| `crates/deslop-core/src/cluster_filters/calls.rs` | `1601ca3a04c81ecf83d8f705e8c79eb86ca7836c8194383b21bda2c785122955` |
| `crates/deslop-core/src/cluster_filters/calls/args.rs` | `9e4732805a3e505f0ec451cd5b001714262576d09a353d1b3e64e68e2f2e8d4f` |
| `crates/deslop-core/src/cluster_filters/calls/callee.rs` | `f879e36b980bb742e7aab69762b8507c8a6ee4fe5c93cfd5dd018f8e3350d8d5` |
| `crates/deslop-core/src/cluster_filters/calls/dataflow.rs` | `0a17f17b35ea2f7fbc82b529b5310dfa6dfef24f6afc5e1b6c72c9ad10f1d61f` |
| `crates/deslop-core/src/cluster_filters/calls/tests.rs` | `124d7aa43dc66386074ec2f6b2b2c442cf5a1cc9bd70c041bfb67a4f9ae02b07` |
| `clients/vscode/src/clusterSelection.ts` | `80a83277be2f456cbcade5bc62098b636aa9f3685c91f615086bb5815c3bcea0` |
| `crates/deslop/tests/content_gate_signal_honesty.rs` | `52ad62980a500d804c248d6f83a6f9adda4c970b6f8c00017414bade526c0ff0` |
| `crates/deslop/tests/skip_policy_contract.rs` | `5492911b4573e414d50838bda70c6cb70d3aec9bfc1f589c19a6883e487175eb` |
