# Registry scaffolding false positive — baseline attribution

Generated mechanically by `target/deslop-astra1-merge/baseline-registry-attribution/measure_registry_baseline.py` (sha256 `b8cc2d190ccba2807643567b213ba64384359b046ce8fe721a2da43096969400`) at 2026-09-10T09:49:09+00:00.
Raw evidence: `target/deslop-astra1-merge/baseline-registry-attribution/evidence.json`.

## Question

`crates/deslop/tests/rust_test_boilerplate_false_positive.rs::registry_call_payload_variation_keeps_only_authored_control` fails because the registry scaffolding files close into a reported cluster. Did the baseline binary, built from `f92300e5`, already report that cluster for the same fixture?

## Verdict: PRE-EXISTING AT BASELINE, SUPPRESSED IN THE CURRENT BUILD

The baseline binary reports the registry scaffolding as a duplicate cluster; the current build does not. The false positive predates the baseline commit, so it is not a regression since the baseline.

| Build | Registry scaffolding reported | Authored control reported |
| --- | --- | --- |
| baseline f92300e5 | yes | yes |
| current worktree (isolated build) | no | yes |
| shared target/release binary | no | yes |

## Provenance

| Item | Value |
| --- | --- |
| Baseline commit | `f92300e5e1004ef6c53a94174a0d7e842232ec80` |
| Baseline commit date | Sat Aug 15 11:15:30 2026 +1000 |
| Baseline commit subject | Fused confidence: honest cluster signals, content gate, subsumption, correct __file__ ranges, and contract-checked AST goldens (#368) |
| Current HEAD | `e88e56a70babb623563632d7acf00c0860be2255` |
| Current branch | fix/regression-rollbacks |
| Baseline archive | `f923-source.tar` sha256 `26bd017d4159a50e3ad3ecfbb1032acf4d40b0f919362230617763be48457c2f` |
| Unpacked baseline tree | 1284 blobs compared with `git ls-tree -r f92300e5`: 7 missing, 3 mismatched, 0 of them Rust sources or manifests (analysis inputs verified: yes) |
| Fixture analysed | `crates/deslop/tests/fixtures/rust-call-scaffolding` from the current worktree |
| Fixture exists at the baseline commit | no — it was added after the baseline, so the baseline binary is run against the current fixture files. |

### Fixture file hashes

| File | sha256 |
| --- | --- |
| `control_first.rs` | `8b2db97f9aad921c4e783d643ee9e5a1fc0db8c832cbece99ba167180dd54996` |
| `control_second.rs` | `8b2db97f9aad921c4e783d643ee9e5a1fc0db8c832cbece99ba167180dd54996` |
| `registry_first.rs` | `9dd8242fe3ab2a60f74963973cae4b5255115d65688387b1e00cd2502d090283` |
| `registry_second.rs` | `30cb893ec44920d6fa4b9d00ef882eecfe0b49ba79ee707ef03565e4ca4cbf24` |

### Literal-variation call filter sources

| Side | File | sha256 |
| --- | --- | --- |
| baseline f92300e5 | `crates/deslop-core/src/cluster_filters/calls.rs` | `0c99f07bc226ad95d91c0f5fcbe0e1e5b00bd67f47c8283d7daf750e6e733348` |
| current worktree (isolated build) | `crates/deslop-core/src/cluster_filters/calls/args.rs` | `901a847663ae5f71ba5629a4cc048012709ba4c5c1b0c722c4c15b09fb2dfd9c` |
| current worktree (isolated build) | `crates/deslop-core/src/cluster_filters/calls/asserts.rs` | `8f91f588be117ecdc357823fc44f5b686bf0e1ef93102bb1ed2643224ad961b6` |
| current worktree (isolated build) | `crates/deslop-core/src/cluster_filters/calls/callee.rs` | `23d577f78c50916418bf33f4e7c1984af8fcd83d6546824c9bed6396c2305bd8` |
| current worktree (isolated build) | `crates/deslop-core/src/cluster_filters/calls/dataflow.rs` | `b9bc05aa99b28176dc283a7783f66a2814e36f0834bae1c260335d4b365f2cf5` |
| current worktree (isolated build) | `crates/deslop-core/src/cluster_filters/calls/sequence.rs` | `e6eef7a041aece6b81534178a0d6bab3db211cdc885d70ba25ff8a8209f2491e` |
| current worktree (isolated build) | `crates/deslop-core/src/cluster_filters/calls/statements.rs` | `47f77b75160acbb245cf2ceb3fd37262f59b54dc57b0115e66a82748252da6e3` |
| current worktree (isolated build) | `crates/deslop-core/src/cluster_filters/calls/tests.rs` | `b7f93802c3c75e56343c2207aab7c305c037180d79dd4fc5069aa8c6cb7cfe71` |

## Builds

| Build | Command | Working directory | Exit code | Log |
| --- | --- | --- | --- | --- |
| baseline f92300e5 | `CARGO_TARGET_DIR=/Users/christianfindlay/Documents/Code/DeslopWorktree/target/deslop-astra1-merge/f923-build cargo build --release --locked -p deslop --bin deslop` | `/Users/christianfindlay/Documents/Code/DeslopWorktree/target/deslop-astra1-merge/f923-source` | 0 | `target/deslop-astra1-merge/baseline-build.log` |
| current worktree (isolated build) | `CARGO_TARGET_DIR=/Users/christianfindlay/Documents/Code/DeslopWorktree/target/deslop-astra1-merge/baseline-current-build cargo build --release --locked -p deslop --bin deslop` | `/Users/christianfindlay/Documents/Code/DeslopWorktree` | 0 | `target/deslop-astra1-merge/baseline-current-build.log` |

## Binaries

| Binary | Path | sha256 | Size | Modified (UTC) | `--version` |
| --- | --- | --- | --- | --- | --- |
| baseline f92300e5 | `/Users/christianfindlay/Documents/Code/DeslopWorktree/target/deslop-astra1-merge/f923-build/release/deslop` | `95816eeada3642fd4981669c9a25915f0a72cdd33f69335e9f36b7d6e2c77ddc` | 28985584 | 2026-09-10T09:43:28+00:00 | `deslop 0.0.0-dev` |
| current worktree (isolated build) | `/Users/christianfindlay/Documents/Code/DeslopWorktree/target/deslop-astra1-merge/baseline-current-build/release/deslop` | `d99c78c3be4d3cf2be342625e740f26bf9e8c2205cef1d2278119e728a503406` | 30020832 | 2026-09-10T09:47:47+00:00 | `deslop 0.0.0-dev` |
| shared target/release binary | `/Users/christianfindlay/Documents/Code/DeslopWorktree/target/release/deslop` | `cb0614eb294406182135b3f41d4e5e23b097dfa8c36e2b4563794d2e6bc6f969` | 30020832 | 2026-09-10T09:48:25+00:00 | `deslop 0.0.0-dev` |

## Commands executed

| Run | Command | Exit code | Report sha256 |
| --- | --- | --- | --- |
| baseline f92300e5 | `/Users/christianfindlay/Documents/Code/DeslopWorktree/target/deslop-astra1-merge/f923-build/release/deslop /Users/christianfindlay/Documents/Code/DeslopWorktree/crates/deslop/tests/fixtures/rust-call-scaffolding --output /Users/christianfindlay/Documents/Code/DeslopWorktree/target/deslop-astra1-merge/baseline-registry-attribution/baseline-f923/report --min-nodes 30 --embeddings off --no-incremental` | 0 | `0cc8ca1d950f707fc0006caba4edd91f300bdf9294bd24acdd2b7abb9075544d` |
| current worktree (isolated build) | `/Users/christianfindlay/Documents/Code/DeslopWorktree/target/deslop-astra1-merge/baseline-current-build/release/deslop /Users/christianfindlay/Documents/Code/DeslopWorktree/crates/deslop/tests/fixtures/rust-call-scaffolding --output /Users/christianfindlay/Documents/Code/DeslopWorktree/target/deslop-astra1-merge/baseline-registry-attribution/current-worktree/report --min-nodes 30 --embeddings off --no-incremental` | 0 | `508f426973ed87c7306cff3553bcd40e8f148d749245d1aed3e50e48f4cd932f` |
| shared target/release binary | `/Users/christianfindlay/Documents/Code/DeslopWorktree/target/release/deslop /Users/christianfindlay/Documents/Code/DeslopWorktree/crates/deslop/tests/fixtures/rust-call-scaffolding --output /Users/christianfindlay/Documents/Code/DeslopWorktree/target/deslop-astra1-merge/baseline-registry-attribution/shared-release/report --min-nodes 30 --embeddings off --no-incremental` | 0 | `508f426973ed87c7306cff3553bcd40e8f148d749245d1aed3e50e48f4cd932f` |

## Result — baseline f92300e5

`baseline f92300e5` — exit code 0, 4 files analysed at `--min-nodes 30`, 2 cluster(s) reported.

| Cluster id | Rank | Kind/bucket | Occurrences | Canonical nodes | Mass | Weight | Files |
| --- | --- | --- | --- | --- | --- | --- | --- |
| a5a7f6fd0423c5d6 | n/a | identical | 2 | 71 | n/a | 71.0 | `control_first.rs`, `control_second.rs` |
| 0cf8050ef19e5deb | n/a | nearly_identical | 2 | 51 | n/a | 41.19230769230769 | `registry_first.rs`, `registry_second.rs` |

Flattened occurrence paths — the value the pinned assertion compares: ["control_first.rs", "control_second.rs", "registry_first.rs", "registry_second.rs"]

Registry scaffolding reported: **yes**. Authored control reported: **yes**. Control matches every constant asserted by the pinned test: **no**.

Authored control against the pinned constants:

| Field | Expected | Observed |
| --- | --- | --- |
| bucket (schema variant) | "<not asserted>" | "identical" |
| canonical_node_count | 71 | 71 |
| end_line | 13 | ["13"] |
| hidden | false | ["false"] |
| id (schema variant) | "<not asserted>" | "a5a7f6fd0423c5d6" |
| kind | "identical" | "<field absent>" |
| mass | 71 | "<field absent>" |
| occurrence_count | 2 | "<field absent>" |
| occurrences_total (schema variant) | "<not asserted>" | 2 |
| rank | 1 | "<field absent>" |
| size (schema variant) | "<not asserted>" | 2 |
| start_line | 1 | ["1"] |
| weight (schema variant) | "<not asserted>" | 71.0 |

## Result — current worktree (isolated build)

`current worktree (isolated build)` — exit code 0, 4 files analysed at `--min-nodes 30`, 1 cluster(s) reported.

| Cluster id | Rank | Kind/bucket | Occurrences | Canonical nodes | Mass | Weight | Files |
| --- | --- | --- | --- | --- | --- | --- | --- |
| d7d759160a366e13 | 1 | identical | 2 | 71 | 71 | n/a | `control_first.rs`, `control_second.rs` |

Flattened occurrence paths — the value the pinned assertion compares: ["control_first.rs", "control_second.rs"]

Registry scaffolding reported: **no**. Authored control reported: **yes**. Control matches every constant asserted by the pinned test: **yes**.

Authored control against the pinned constants:

| Field | Expected | Observed |
| --- | --- | --- |
| canonical_node_count | 71 | 71 |
| end_line | 13 | ["13"] |
| hidden | false | ["false"] |
| id (schema variant) | "<not asserted>" | "d7d759160a366e13" |
| kind | "identical" | "identical" |
| mass | 71 | 71 |
| occurrence_count | 2 | 2 |
| occurrences_total (schema variant) | "<not asserted>" | 2 |
| rank | 1 | 1 |
| start_line | 1 | ["1"] |

## Result — shared target/release binary

`shared target/release binary` — exit code 0, 4 files analysed at `--min-nodes 30`, 1 cluster(s) reported.

| Cluster id | Rank | Kind/bucket | Occurrences | Canonical nodes | Mass | Weight | Files |
| --- | --- | --- | --- | --- | --- | --- | --- |
| d7d759160a366e13 | 1 | identical | 2 | 71 | 71 | n/a | `control_first.rs`, `control_second.rs` |

Flattened occurrence paths — the value the pinned assertion compares: ["control_first.rs", "control_second.rs"]

Registry scaffolding reported: **no**. Authored control reported: **yes**. Control matches every constant asserted by the pinned test: **yes**.

Authored control against the pinned constants:

| Field | Expected | Observed |
| --- | --- | --- |
| canonical_node_count | 71 | 71 |
| end_line | 13 | ["13"] |
| hidden | false | ["false"] |
| id (schema variant) | "<not asserted>" | "d7d759160a366e13" |
| kind | "identical" | "identical" |
| mass | 71 | 71 |
| occurrence_count | 2 | 2 |
| occurrences_total (schema variant) | "<not asserted>" | 2 |
| rank | 1 | 1 |
| start_line | 1 | ["1"] |

## Corroborating test failure

`target/deslop-astra1-submit-pr/calls-quarantine-test.log` (sha256 `015331177a883d743b76fdebeab3612e70d488e014533ec7f8286ce954ba95d2`, modified 2026-09-10T09:36:11+00:00) records:

```
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/DeslopWorktree/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/DeslopWorktree/crates/deslop-test-support)
   Compiling deslop v0.0.0-dev (/Users/christianfindlay/Documents/Code/DeslopWorktree/crates/deslop)
    Finished `release` profile [optimized] target(s) in 53.34s
     Running tests/suite.rs (target/release/deps/suite-bd68161f6f825ff1)

running 1 test

thread 'rust_test_boilerplate_false_positive::registry_call_payload_variation_keeps_only_authored_control' (9612082) panicked at crates/deslop/tests/rust_test_boilerplate_false_positive.rs:161:5:
assertion `left == right` failed: registry setup only varies path payloads; it is call scaffolding, not a clone
  left: ["control_first.rs", "control_second.rs", "registry_first.rs", "registry_second.rs"]
 right: ["control_first.rs", "control_second.rs"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test rust_test_boilerplate_false_positive::registry_call_payload_variation_keeps_only_authored_control ... FAILED

failures:

failures:
    rust_test_boilerplate_false_positive::registry_call_payload_variation_keeps_only_authored_control

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 496 filtered out; finished in 0.73s

error: test failed, to rerun pass `-p deslop --test suite`
```

The log was written at 2026-09-10T09:36:11+00:00; the current worktree binary measured above was built at 2026-09-10T09:47:47+00:00. That binary does not reproduce the flattened path list the log records, so the analysed sources changed between the recorded failure and this measurement.

## Reproduce

```sh
python3 target/deslop-astra1-merge/baseline-registry-attribution/measure_registry_baseline.py --emit-report
```
