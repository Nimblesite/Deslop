# Live directory reconciliation validation

Generated from captured command output at 2026-10-03T22:54:07.955952+00:00.

## Initial directory reconciliation red

Artifact: [target/live-directory-reconcile-red.log](../target/live-directory-reconcile-red.log)

SHA-256: `677c130fda9a86daa887afe8b68dcd1481b7673fcc562dbad79c2f97d8e85d57`

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.30s
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 207 filtered out; finished in 0.06s
error: test failed, to rerun pass `-p deslop-core --test suite`
```

## Ignore/config policy red

Artifact: [target/live-directory-policy-red.log](../target/live-directory-policy-red.log)

SHA-256: `08dc1e668024e4167e855b2a3971b0d362ddad1689c8f9b6cad07bb201a4c355`

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.23s
test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 207 filtered out; finished in 0.12s
error: test failed, to rerun pass `-p deslop-core --test suite`
```

## Completeness and traversal red

Artifact: [target/live-directory-completeness-red.log](../target/live-directory-completeness-red.log)

SHA-256: `7168061f737f26482433edd3e87f01748c2f1eaef4911f4c2d3614ecda0bcad2`

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.99s
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 346 filtered out; finished in 0.00s
error: test failed, to rerun pass `-p deslop-core --lib`
```

## Deleted directory rule lifecycle red

Artifact: [target/live-directory-lifecycle-red.log](../target/live-directory-lifecycle-red.log)

SHA-256: `01f4b0ed808a304cfe66fab8d24d5ddfe2aa3fcb8f230f3512cceed601324b9b`

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.60s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 213 filtered out; finished in 0.02s
error: test failed, to rerun pass `-p deslop-core --test suite`
```

## Final discovery units

Artifact: [target/live-directory-completeness-green.log](../target/live-directory-completeness-green.log)

SHA-256: `b316c8c73871f01617064ecab9f6dd839875e3877a8aa5e035ea90569b9289e2`

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.24s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 346 filtered out; finished in 0.01s
```

## Final public live session tests

Artifact: [target/live-directory-live-green.log](../target/live-directory-live-green.log)

SHA-256: `12419fdcbc017c7f1a88c73d27fd8c5ca853ac5b4cfed8fee9aa18cbd72d9f3c`

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.07s
test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 185 filtered out; finished in 7.84s
```

## Strict core Clippy

Artifact: [target/live-directory-clippy.log](../target/live-directory-clippy.log)

SHA-256: `84ea618df068fcce177284ebb88fc2ca6a6015c61b525bf44913d847542a425b`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.86s
```

## Canonical reuse check

MCP was unavailable in this agent; the existing CLI analysed `crates/deslop-core/src` with embeddings off.

Canonical report: [target/live-discovery-reuse.json](../target/live-discovery-reuse.json)

Files analysed: 281. Total clusters: 233. Clusters touching the discovery/session mutation modules: 0.
