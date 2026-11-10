# Current CI preparation evidence

Source commit: `c79016b4fe1fdd0c2245d9a2a81294f6467e5cca`.

## Static analysis

Command: `CARGO_BUILD_JOBS=1 nice -n 15 make _ci-analyze`. Exit status: 0.

```text
==> _typediagram-gen: regenerating IPC models from docs/models/live-ipc.td
node scripts/typediagram/generate.mjs
typediagram-gen: unchanged /Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core/src/wire_generated.rs
typediagram-gen: unchanged /Users/christianfindlay/Documents/Code/Deslop/clients/vscode/src/types/wire-generated.ts
==> Linting...
cargo clippy --release --all-targets --workspace --features deslop-core/live,deslop-lsp/profiling -- -D warnings
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
    Checking tree-sitter-osprey-ml v0.1.0 (https://github.com/Nimblesite/osprey?rev=093f04242c7d8b2f56d8d5051dd813b9e9d280f3#093f0424)
    Checking tree-sitter-osprey v0.0.0-dev (https://github.com/Nimblesite/osprey?rev=093f04242c7d8b2f56d8d5051dd813b9e9d280f3#093f0424)
    Checking deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Checking deslop-mcp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-mcp)
    Checking deslop-lsp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-lsp)
    Checking deslop v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop)
    Finished `release` profile [optimized] target(s) in 28.13s
Taxonomy gate OK — every product-facing Type-N reference has a bucket label.
env/PATH write lint: 8 files clean
```

Remaining gates are tracked in [the CI plan](../../docs/plans/ci-prep-2026-11-10.md).

## Release workspace build

Command: `CARGO_BUILD_JOBS=1 nice -n 15 make build`. Exit status: 0.

```text
==> Building...
cargo build --release --workspace
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
   Compiling deslop-lsp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-lsp)
   Compiling deslop-mcp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-mcp)
   Compiling deslop v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop)
    Finished `release` profile [optimized] target(s) in 4m 59s
```

## Repository contract tests

Command: `NODE_V8_COVERAGE=coverage/repository/ci-prep-2026-11-10 nice -n 15 make _ci-contract-tests`. Exit status: 0.

Passed tests across TAP groups: 215. Fresh raw coverage: `coverage/repository/ci-prep-2026-11-10`; thresholds are not yet calculated.

```text
# tests 52
# pass 52
# fail 0
# skipped 0
# tests 12
# pass 12
# fail 0
# skipped 0
# tests 15
# pass 15
# fail 0
# skipped 0
# tests 3
# pass 3
# fail 0
# skipped 0
# tests 4
# pass 4
# fail 0
# skipped 0
# tests 5
# pass 5
# fail 0
# skipped 0
# tests 11
# pass 11
# fail 0
# skipped 0
# tests 8
# pass 8
# fail 0
# skipped 0
# tests 8
# pass 8
# fail 0
# skipped 0
# tests 5
# pass 5
# fail 0
# skipped 0
# tests 9
# pass 9
# fail 0
# skipped 0
# tests 5
# pass 5
# fail 0
# skipped 0
# tests 4
# pass 4
# fail 0
# skipped 0
# tests 15
# pass 15
# fail 0
# skipped 0
# tests 4
# pass 4
# fail 0
# skipped 0
# tests 3
# pass 3
# fail 0
# skipped 0
# tests 11
# pass 11
# fail 0
# skipped 0
# tests 20
# pass 20
# fail 0
# skipped 0
# tests 5
# pass 5
# fail 0
# skipped 0
# tests 9
# pass 9
# fail 0
# skipped 0
# tests 3
# pass 3
# fail 0
# skipped 0
# tests 4
# pass 4
# fail 0
# skipped 0
```

## Coverage-phase correction verification

Source commit: `79102ef0`. Command: `NODE_V8_COVERAGE=coverage/repository/ci-phases nice -n 15 make _ci-contract-tests`. Exit status: 0.

The full contract suite passed 216 tests across 23 TAP groups, including the real-Make coverage-phase success/failure regression. Failed tests: 0.

## Upstream grammar pin: remote CI

Commit: `5f8a0e6100f9e2e92af751a0b8282310746c9a21`. [Workflow run](https://github.com/Nimblesite/Deslop/actions/runs/42720123053). Conclusion: `success`. This run predates the local coverage-phase and workflow-concurrency changes.

| Job | Result |
| --- | --- |
| Detect changes | success |
| security | success |
| Format + analyzers | success |
| Rust tests + coverage collection | success |
| Build release | success |
| Windows tests + coverage collection | success |
| Repository tests + coverage collection | success |
| Website build | success |
| VSIX tests + coverage | success |
| JetBrains package gate | success |
| Corpus targets | success |
| Corpus accuracy score (fastapi) | success |
| Corpus accuracy score (serde) | success |
| Corpus accuracy score (http) | success |
| Corpus accuracy score (vite) | success |
| Corpus accuracy score (viper) | success |
| Corpus accuracy score (FSharpPlus) | success |
| Corpus accuracy score (console) | success |
| Corpus accuracy score (gin) | success |
| Corpus accuracy score (date-fns) | success |
| Corpus accuracy score (express) | success |
| Corpus accuracy score (zlib) | success |
| Corpus accuracy score (monolog) | success |
| Corpus accuracy score (rayon) | success |
| Corpus accuracy score (axios) | success |
| Corpus accuracy score (BenchmarkDotNet) | success |
| Calculate coverage + thresholds | success |
| CI | success |

## Website evidence reuse

The source commit in [the browser receipt](../videos/site-validation.md) is `1c6bf836fc0a0315b43ba5fec0f65f6cf1bf066d`. Git tree identities below prove the current website source, assets, tests and issue generator are unchanged.

| Path | Git tree | Comparison |
| --- | --- | --- |
| `site` | `f5f7894af17190c485f95dd6839cca3093510eff` | identical |
| `scripts/issues` | `ee1233d831604d037bc9c48954944badc77103e3` | identical |

## Deployment gate retry

Captured 2026-11-09T23:50:19.555469+00:00 at `26e9c4e5f05f30bcfb8b0a40f83b6e54f6baf8e1`.

Command: `timeout 240 nice -n 15 make -o build _deployment-verify`

Exit status: **0**. The already verified release build was reused; every deployment verification recipe ran unchanged.

```text
node scripts/deployment/verify-deployment-manifest.mjs shipwright.json
/Users/christianfindlay/Documents/Code/Deslop/shipwright.json: valid deployment manifest
node scripts/deployment/verify-deployment-binaries.mjs shipwright.json target/release
Verified deployment binaries in /Users/christianfindlay/Documents/Code/Deslop/target/release for darwin-arm64
node scripts/release/verify-release-workflow-gates.mjs .github/workflows/release.yml
/Users/christianfindlay/Documents/Code/Deslop/.github/workflows/release.yml: release workflow gates wired (manifest + binaries + VSIX)
node scripts/release/test-release-workflow-contract.mjs
ok releaseBuildsTaggedSourceWithoutPostTagVersionCommit
ok releaseArchivesContainPackageManagerDeclaredBinaries
ok scoopManifestExtractDirMatchesWindowsArchiveRoot
ok scoopAutoupdateTemplatesEveryVersionedSegment
ok releaseBuildsPlatformSpecificVsixArtifacts
ok pagesDeployCleansRerunArtifactsAndRetries
ok dependabotSweepLeavesNoDeadCheckOnHumanPullRequests
ok contractFilesCheckOutLfOnEveryPlatform

8 release workflow contract tests passed
node scripts/release/test-release-publish-contract.mjs
ok marketplaceAttemptsEveryPlatformExactlyOnceWhenOneFails
ok openvsxAttemptsEveryPlatformExactlyOnceWhenOneFails
ok marketplacePublishesEveryPlatformAgainstAHealthyRegistry
ok openvsxPublishesEveryPlatformAgainstAHealthyRegistry
ok anIncompleteArtifactSetPublishesNothing
ok aHyphenatedTagPublishesAsAPrerelease
ok declaredPlatformsMatchTheBuildMatrix
ok thisSuiteRunsInTheDeploymentVerifyTarget

8 release publish contract tests passed
node scripts/deployment/test-deployment-docs-contract.mjs
ok releaseWorkflowPublishesPlatformSpecificVsixArtifacts
ok documentedExtensionDirectoryCarriesItsPlatformTarget
ok installerSnippetContractRunsWhereverTheSnippetCanChange
ok sharedConfigsAreNeverToldToRelyOnPath

4 deployment docs contract tests passed
node scripts/release/test-release-version-stamping.mjs
ok sourceProjectsUseVersionPlaceholder
ok stamperSetsEveryProjectVersion
ok stamperStampsGeneratedVsixManifest
ok stamperStampsEveryWorkspaceCrateInLock
ok stamperLeavesDocumentedPinsUntouched
ok stamperRejectsInvalidVersion

6 release version stamping tests passed
node scripts/deployment/test-verifiers.mjs
ok manifestRejectsMissingProductId
ok manifestRejectsLooseSemver
ok manifestRejectsHostVerifyingUnknownComponent
ok manifestRejectsExpectedVersionDrift
ok manifestAcceptsRepoManifest
ok manifestRejectsCargoBinSource
ok manifestRejectsBundledSourceWithoutBundle
ok manifestRejectsBundleWithoutBundledSource
ok manifestRejectsPathAheadOfBundle
ok manifestRejectsIdeHostComponentWithoutBundle
ok manifestShipsEveryBundledBinaryAsBundled
ok binariesRejectWrongVersion
ok binariesRejectWrongComponentName
ok binariesRejectMissingBinary
ok binariesNameTheFileEachTargetPlatformSpells
ok binariesRejectStaleJsonManifestVersion
ok binariesAcceptValidContract
ok jetbrainsRejectsMissingManifest
ok jetbrainsRejectsMissingBundledBinary
ok jetbrainsRejectsWrongVersionBundle
ok jetbrainsRejectsWrongComponentNameBundle
ok jetbrainsRejectsUndeclaredBundle
ok jetbrainsRejectsContentModuleJar
ok jetbrainsRejectsMissingSharedJar
ok jetbrainsAcceptsValidPackage
ok vsixRejectsMissingManifest
ok vsixRejectsMissingBundledLsp
ok vsixRejectsMissingBundledMcp
ok vsixRejectsWrongVersionBundle
ok vsixRejectsWrongComponentNameBundle
ok vsixRejectsUndeclaredBundle
ok vsixRejectsForeignPlatformBundle
ok vsixRejectsCompiledOutDir
ok vsixAcceptsValidPackage
ok releaseWorkflowRejectsMissingManifestGate
ok releaseWorkflowRejectsMissingBinaryGate
ok releaseWorkflowRejectsMissingVsixGate
ok releaseWorkflowRejectsMissingVersionStamper
ok releaseWorkflowRejectsBareVsce
ok releaseWorkflowAcceptsRepoWorkflow

40 verifier proof tests passed
node scripts/actions/test-action-contract.mjs
  ok  every published runner maps to its release asset
  ok  an unpublished runner names the offending pair
  ok  the release workflow still publishes every asset this action requests
  ok  a tag-pinned action installs the matching CLI version
  ok  an explicit version input overrides the pinned ref
  ok  a SHA-pinned action demands an explicit version instead of guessing latest
  ok  a branch-pinned action demands an explicit version
  ok  unix coordinates match the packaged tarball
  ok  windows coordinates match the packaged zip
  ok  a matching checksum is accepted
  ok  a tampered archive is rejected before extraction
  ok  the sidecar's `<hash>  <file>` form is parsed to the hash alone
  ok  an empty sidecar is rejected
  ok  outputs carry the measured percentage, cluster count and ceiling
  ok  a breached run still publishes its measurements
  ok  a usage error reports only its exit code
  ok  a diff-gated run names the added-lines scope with the diff figures
  ok  the report alone reroutes the scope when its diff threshold is live
  ok  a tagging-only --diff run keeps the repository gate scope
  ok  an only-changed run whose report lacks diff metrics is a hard error
  ok  a clean run that rendered no report is a hard error, not an empty output
  ok  action.yml declares the Marketplace-required metadata
  ok  the Marketplace name is not the org-colliding bare product name
  ok  every documented input is declared
  ok  the diff inputs are forwarded to the CLI only when set
  ok  only-changed without a diff fails fast with the CLI's usage status
  ok  a diff-gated breach names the added-lines population
  ok  the self-test carries the diff-gate leg, version-gated on the release
  ok  the branch-built action diff-gate proof runs in the deployment gate
  ok  every output the helper emits is declared and wired
  ok  the action rejects the stdin diff form before downloading a CLI
  ok  no run body interpolates a GitHub expression
  ok  every nested action is pinned to a full-length commit SHA
  ok  the parse store is cached around the run, keyed per version and run
  ok  the cache is opt-out and a storeless run skips the save instead of failing it
  ok  the pinned ref reaches the resolver
  ok  no documented pin commits a version
  ok  the data module backing the rendered pin exports only a default
  ok  each surface pins the form it can actually resolve
  ok  the gate re-raises the CLI status rather than swallowing it
  ok  a breach cannot abort the run step before the status is captured
  ok  extraction survives Git Bash tar on Windows

action contract: 42 checks passed
node scripts/actions/test-action-diff-gate.mjs
  ok  legacy debt passes a zero ceiling when the diff adds nothing
  ok  a changed-code diff that adds duplication breaches the same ceiling
  ok  the stdin-diff guard prints a runnable command for the event's base ref
  ok  a push event with no base ref still gets a runnable command
  ok  shell-significant text in the base ref is printed as data, never executed
action diff-gate black-box: 5 checks passed

```

## Repository coverage calculation

Command: `npx --yes c8@12.0.0 report --temp-directory coverage/repository/ci-phases --reporter=json-summary --reports-dir coverage/repository/current`

Exit status: **0**. Inputs are the current 216-test repository contract run and the successful deployment run with V8 coverage enabled. This reports executed repository scripts; it does not prove Rust, extension, or webview coverage.

Source commit: `79eb2821d2c0e59069ae9e1ddad3d449d2c7b8b4`. Summary SHA-256: `f9c5c6bc96e39201d9b83740fdefa2daa5fd137322758185f10de80c69d0aa93`.

```json
{
  "files": 54,
  "total": {
    "lines": {
      "total": 8682,
      "covered": 8340,
      "skipped": 0,
      "pct": 96.06
    },
    "statements": {
      "total": 8682,
      "covered": 8340,
      "skipped": 0,
      "pct": 96.06
    },
    "functions": {
      "total": 451,
      "covered": 437,
      "skipped": 0,
      "pct": 96.89
    },
    "branches": {
      "total": 1612,
      "covered": 1406,
      "skipped": 0,
      "pct": 87.22
    },
    "branchesTrue": {
      "total": 0,
      "covered": 0,
      "skipped": 0,
      "pct": 100
    }
  }
}
```

## Coverage-orchestration CI result

[CI run](https://github.com/Nimblesite/Deslop/actions/runs/42722383742) at `16aaf22b8333404bcd1cce065c70f74759e76e80`: **success**.

| Job | Result |
| --- | --- |
| security | success |
| Detect changes | success |
| Format + analyzers | success |
| Windows tests + coverage collection | success |
| Rust tests + coverage collection | success |
| Build release | success |
| Repository tests + coverage collection | success |
| Website build | success |
| JetBrains package gate | success |
| VSIX tests + coverage | success |
| Corpus targets | success |
| Corpus accuracy score (date-fns) | success |
| Corpus accuracy score (console) | success |
| Corpus accuracy score (axios) | success |
| Corpus accuracy score (BenchmarkDotNet) | success |
| Corpus accuracy score (FSharpPlus) | success |
| Corpus accuracy score (gin) | success |
| Corpus accuracy score (express) | success |
| Corpus accuracy score (rayon) | success |
| Corpus accuracy score (monolog) | success |
| Corpus accuracy score (vite) | success |
| Corpus accuracy score (viper) | success |
| Corpus accuracy score (zlib) | success |
| Corpus accuracy score (fastapi) | success |
| Corpus accuracy score (http) | success |
| Corpus accuracy score (serde) | success |
| Calculate coverage + thresholds | success |
| CI | success |

## Webview coverage restoration fix

Source: `8f644e8314c7f30b8c061148afd9df008e6e6113`.

### VSIX script contracts

```text
TAP version 13
# Subtest: every surface that must be gated still has a gate
ok 1 - every surface that must be gated still has a gate
  ---
  duration_ms: 0.51175
  type: 'test'
  ...
# Subtest: every gate names a floor that exists and is a real percentage
ok 2 - every gate names a floor that exists and is a real percentage
  ---
  duration_ms: 1.22275
  type: 'test'
  ...
# Subtest: no declared floor goes unenforced
ok 3 - no declared floor goes unenforced
  ---
  duration_ms: 0.365416
  type: 'test'
  ...
# Subtest: each gate reads its own coverage directory
ok 4 - each gate reads its own coverage directory
  ---
  duration_ms: 0.1485
  type: 'test'
  ...
# Subtest: diagnostics default off with open-file scope
ok 5 - diagnostics default off with open-file scope
  ---
  duration_ms: 0.61225
  type: 'test'
  ...
# Subtest: each category accepts only diagnostic levels, with omitted entries resolved by Rust
ok 6 - each category accepts only diagnostic levels, with omitted entries resolved by Rust
  ---
  duration_ms: 0.128125
  type: 'test'
  ...
# Subtest: the suite is handed staged fixtures, after instrumentation
ok 7 - the suite is handed staged fixtures, after instrumentation
  ---
  duration_ms: 579.127208
  type: 'test'
  ...
# Subtest: a failed clean recompile fails the command and names the instrumented tree
ok 8 - a failed clean recompile fails the command and names the instrumented tree
  ---
  duration_ms: 475.106916
  type: 'test'
  ...
# Subtest: a clean run with a clean restore passes
ok 9 - a clean run with a clean restore passes
  ---
  duration_ms: 0.318208
  type: 'test'
  ...
# Subtest: a failed restore alone still fails the run
ok 10 - a failed restore alone still fails the run
  ---
  duration_ms: 1.709958
  type: 'test'
  ...
# Subtest: a failed collection fails the run and is reported on its own
ok 11 - a failed collection fails the run and is reported on its own
  ---
  duration_ms: 0.06375
  type: 'test'
  ...
# Subtest: a failed collection keeps a failed restore as context
ok 12 - a failed collection keeps a failed restore as context
  ---
  duration_ms: 0.049125
  type: 'test'
  ...
# Subtest: an empty failure string is not a failure
ok 13 - an empty failure string is not a failure
  ---
  duration_ms: 0.045666
  type: 'test'
  ...
# Subtest: [\#472] the gate rejects the VSIX that actually shipped, naming the leaked entry
ok 14 - [\#472] the gate rejects the VSIX that actually shipped, naming the leaked entry
  ---
  duration_ms: 2.396791
  type: 'test'
  ...
# Subtest: [\#472] the same VSIX passes once the leaked directory is gone
ok 15 - [\#472] the same VSIX passes once the leaked directory is gone
  ---
  duration_ms: 0.298083
  type: 'test'
  ...
# Subtest: [\#472] a failing Playwright run's trace is rejected too, not just the run marker
ok 16 - [\#472] a failing Playwright run's trace is rejected too, not just the run marker
  ---
  duration_ms: 0.206375
  type: 'test'
  ...
# Subtest: [\#472] an allow-list keeps rejecting everything the old deny-list named
ok 17 - [\#472] an allow-list keeps rejecting everything the old deny-list named
  ---
  duration_ms: 0.190334
  type: 'test'
  ...
# Subtest: [\#472] a brand-new sibling directory fails closed without anyone updating a list
ok 18 - [\#472] a brand-new sibling directory fails closed without anyone updating a list
  ---
  duration_ms: 0.230167
  type: 'test'
  ...
# Subtest: the shipping surface accepts every metadata entry, named file, and directory it declares
ok 19 - the shipping surface accepts every metadata entry, named file, and directory it declares
  ---
  duration_ms: 0.151709
  type: 'test'
  ...
# Subtest: a file at the archive root outside vsce's metadata is rejected
ok 20 - a file at the archive root outside vsce's metadata is rejected
  ---
  duration_ms: 0.153917
  type: 'test'
  ...
# Subtest: declaredEntries resolves the manifest's own asset paths into archive entries
ok 21 - declaredEntries resolves the manifest's own asset paths into archive entries
  ---
  duration_ms: 0.133041
  type: 'test'
  ...
# Subtest: assertDeclaredEntriesPresent passes the real listing and catches an over-eager ignore rule
ok 22 - assertDeclaredEntriesPresent passes the real listing and catches an over-eager ignore rule
  ---
  duration_ms: 0.614583
  type: 'test'
  ...
# Subtest: [\#472] .vscodeignore keeps Playwright's output and config out of the package
ok 23 - [\#472] .vscodeignore keeps Playwright's output and config out of the package
  ---
  duration_ms: 2.148
  type: 'test'
  ...
# Subtest: [\#472] Playwright writes its output under target/, never beside the extension
ok 24 - [\#472] Playwright writes its output under target/, never beside the extension
  ---
  duration_ms: 202.27525
  type: 'test'
  ...
# Subtest: findStubToken ignores a clean Ollama-only bundle
ok 25 - findStubToken ignores a clean Ollama-only bundle
  ---
  duration_ms: 0.913833
  type: 'test'
  ...
# Subtest: findStubToken catches the blake3-stub model id
ok 26 - findStubToken catches the blake3-stub model id
  ---
  duration_ms: 0.168625
  type: 'test'
  ...
# Subtest: findStubToken catches the StubProvider type
ok 27 - findStubToken catches the StubProvider type
  ---
  duration_ms: 0.052959
  type: 'test'
  ...
# Subtest: findStubToken catches the quoted provider id "stub"
ok 28 - findStubToken catches the quoted provider id "stub"
  ---
  duration_ms: 0.079834
  type: 'test'
  ...
# Subtest: findStubToken catches the quoted provider id 'stub'
ok 29 - findStubToken catches the quoted provider id 'stub'
  ---
  duration_ms: 0.042625
  type: 'test'
  ...
# Subtest: findStubToken does not fire on the word stub inside other identifiers
ok 30 - findStubToken does not fire on the word stub inside other identifiers
  ---
  duration_ms: 0.041875
  type: 'test'
  ...
# Subtest: findStubSettingKey passes the Ollama-only enum
ok 31 - findStubSettingKey passes the Ollama-only enum
  ---
  duration_ms: 0.081958
  type: 'test'
  ...
# Subtest: findStubSettingKey catches stub in the enum
ok 32 - findStubSettingKey catches stub in the enum
  ---
  duration_ms: 0.047
  type: 'test'
  ...
# Subtest: findStubSettingKey catches stub as the default
ok 33 - findStubSettingKey catches stub as the default
  ---
  duration_ms: 0.224583
  type: 'test'
  ...
# Subtest: findStubSettingKey handles the array form of contributes.configuration
ok 34 - findStubSettingKey handles the array form of contributes.configuration
  ---
  duration_ms: 0.26225
  type: 'test'
  ...
# Subtest: isStubScanEntry selects the manifest and shipped text assets only
ok 35 - isStubScanEntry selects the manifest and shipped text assets only
  ---
  duration_ms: 0.104
  type: 'test'
  ...
# Subtest: assertNoStubProvider passes a clean VSIX and reports the scanned assets
ok 36 - assertNoStubProvider passes a clean VSIX and reports the scanned assets
  ---
  duration_ms: 0.348375
  type: 'test'
  ...
# Subtest: assertNoStubProvider rejects a stub model id leaked into the bundle
ok 37 - assertNoStubProvider rejects a stub model id leaked into the bundle
  ---
  duration_ms: 0.207667
  type: 'test'
  ...
# Subtest: assertNoStubProvider rejects StubProvider leaked into the schema doc
ok 38 - assertNoStubProvider rejects StubProvider leaked into the schema doc
  ---
  duration_ms: 0.064209
  type: 'test'
  ...
# Subtest: assertNoStubProvider rejects stub re-added to the settings enum
ok 39 - assertNoStubProvider rejects stub re-added to the settings enum
  ---
  duration_ms: 0.050709
  type: 'test'
  ...
# Subtest: assertNoStubProvider does not read binary entries
ok 40 - assertNoStubProvider does not read binary entries
  ---
  duration_ms: 0.04275
  type: 'test'
  ...
# Subtest: the suite-launching scripts are exactly the ones this contract knows about
ok 41 - the suite-launching scripts are exactly the ones this contract knows about
  ---
  duration_ms: 2.327666
  type: 'test'
  ...
# Subtest: `npm run test` compiles the suite before launching it
ok 42 - `npm run test` compiles the suite before launching it
  ---
  duration_ms: 0.409958
  type: 'test'
  ...
# Subtest: `npm run test:ollama` compiles the suite before launching it
ok 43 - `npm run test:ollama` compiles the suite before launching it
  ---
  duration_ms: 0.1265
  type: 'test'
  ...
# Subtest: `npm run coverage:collect` compiles the suite before launching it
ok 44 - `npm run coverage:collect` compiles the suite before launching it
  ---
  duration_ms: 0.111292
  type: 'test'
  ...
# Subtest: the compile script is the one that emits out/, not a type-check
ok 45 - the compile script is the one that emits out/, not a type-check
  ---
  duration_ms: 0.16575
  type: 'test'
  ...
# Subtest: a deep checkout still yields a socket path the kernel will accept on darwin
ok 46 - a deep checkout still yields a socket path the kernel will accept on darwin
  ---
  duration_ms: 0.521875
  type: 'test'
  ...
# Subtest: a deep checkout still yields a socket path the kernel will accept on linux
ok 47 - a deep checkout still yields a socket path the kernel will accept on linux
  ---
  duration_ms: 0.091625
  type: 'test'
  ...
# Subtest: the default profile location is what actually overflows
ok 48 - the default profile location is what actually overflows
  ---
  duration_ms: 0.063
  type: 'test'
  ...
# Subtest: Windows is not held to the Unix socket cap
ok 49 - Windows is not held to the Unix socket cap
  ---
  duration_ms: 0.155875
  type: 'test'
  ...
# Subtest: POSIX anchors at /tmp rather than the platform temp dir
ok 50 - POSIX anchors at /tmp rather than the platform temp dir
  ---
  duration_ms: 0.048959
  type: 'test'
  ...
# Subtest: two checkouts never share one profile
ok 51 - two checkouts never share one profile
  ---
  duration_ms: 0.111
  type: 'test'
  ...
# Subtest: the profile is anchored outside the checkout
ok 52 - the profile is anchored outside the checkout
  ---
  duration_ms: 0.168166
  type: 'test'
  ...
# Subtest: webview collection failures always restore the production bundle and retain restore failures
ok 53 - webview collection failures always restore the production bundle and retain restore failures
  ---
  duration_ms: 349.180959
  type: 'test'
  ...
1..53
# tests 53
# suites 0
# pass 53
# fail 0
# cancelled 0
# skipped 0
# todo 0
# duration_ms 1101.062709

```
### Real-browser webview coverage

```text

> deslop-live@0.0.0-dev coverage:webview
> node ./scripts/webview-coverage.mjs


> deslop-webview-ui@0.0.0-dev build
> node build.mjs --coverage


Running 33 tests using 2 workers

(node:19922) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
(node:19923) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓   1 scripts/playwright-report-parity.spec.ts:27:7 › HTML and VSIX agree on included groups and verdicts (430ms)
  ✓   2 scripts/playwright-webview-smoke.spec.ts:180:5 › cluster comparison controls distinguish canonical and selected endpoints (453ms)
  ✓   3 scripts/playwright-report-parity.spec.ts:27:7 › HTML and VSIX agree on empty-section groups and verdicts (203ms)
  ✓   4 scripts/playwright-webview-smoke.spec.ts:194:5 › live updates release a comparison selection whose range disappeared (255ms)
  ✓   6 scripts/playwright-webview-smoke.spec.ts:213:7 › VSIX webview bundles › excluded report rows are marked and muted until their kind is enabled (189ms)
  ✓   5 scripts/playwright-report-parity.spec.ts:27:7 › HTML and VSIX agree on excluded groups and verdicts (313ms)
  ✓   7 scripts/playwright-webview-smoke.spec.ts:237:7 › VSIX webview bundles › the report's path filter keeps exactly the clusters with a matching location, in any case (109ms)
  ✓   8 scripts/playwright-webview-smoke.spec.ts:251:7 › VSIX webview bundles › shape-only findings show information without duplication claims (218ms)
  ✓   9 scripts/playwright-webview-smoke.spec.ts:270:9 › VSIX webview bundles › cluster help is formatted, immediate and keyboard accessible on desktop (283ms)
  ✓  10 scripts/playwright-webview-smoke.spec.ts:288:9 › VSIX webview bundles › report view renders and posts commands on desktop (179ms)
  ✓  11 scripts/playwright-webview-smoke.spec.ts:320:9 › VSIX webview bundles › cluster view renders, navigates, and posts commands on desktop (497ms)
  ✓  12 scripts/playwright-webview-smoke.spec.ts:393:9 › VSIX webview bundles › duplication view renders file rollup on desktop (132ms)
  ✓  13 scripts/playwright-webview-smoke.spec.ts:270:9 › VSIX webview bundles › cluster help is formatted, immediate and keyboard accessible on narrow (262ms)
  ✓  14 scripts/playwright-webview-smoke.spec.ts:288:9 › VSIX webview bundles › report view renders and posts commands on narrow (189ms)
  ✓  15 scripts/playwright-webview-smoke.spec.ts:320:9 › VSIX webview bundles › cluster view renders, navigates, and posts commands on narrow (493ms)
  ✓  16 scripts/playwright-webview-smoke.spec.ts:393:9 › VSIX webview bundles › duplication view renders file rollup on narrow (120ms)
  ✓  17 scripts/playwright-webview-smoke.spec.ts:417:11 › VSIX webview bundles › desktop: a cluster panel loading via select then snapshot shows a spinner, never the empty state (#606) (225ms)
  ✓  18 scripts/playwright-webview-smoke.spec.ts:417:11 › VSIX webview bundles › desktop: a cluster panel loading via snapshot then select shows a spinner, never the empty state (#606) (209ms)
  ✓  19 scripts/playwright-webview-smoke.spec.ts:417:11 › VSIX webview bundles › desktop: a cluster panel loading via select then delta shows a spinner, never the empty state (#606) (198ms)
  ✓  20 scripts/playwright-webview-smoke.spec.ts:417:11 › VSIX webview bundles › desktop: a cluster panel loading via analysis running, then select and snapshot shows a spinner, never the empty state (#606) (203ms)
  ✓  21 scripts/playwright-webview-smoke.spec.ts:417:11 › VSIX webview bundles › desktop: a cluster panel loading via analysis errored, then select and snapshot shows a spinner, never the empty state (#606) (203ms)
  ✓  22 scripts/playwright-webview-smoke.spec.ts:417:11 › VSIX webview bundles › desktop: a cluster panel loading via facet filter and filters first shows a spinner, never the empty state (#606) (208ms)
  ✓  23 scripts/playwright-webview-smoke.spec.ts:417:11 › VSIX webview bundles › narrow: a cluster panel loading via select then snapshot shows a spinner, never the empty state (#606) (202ms)
  ✓  24 scripts/playwright-webview-smoke.spec.ts:417:11 › VSIX webview bundles › narrow: a cluster panel loading via snapshot then select shows a spinner, never the empty state (#606) (207ms)
  ✓  25 scripts/playwright-webview-smoke.spec.ts:417:11 › VSIX webview bundles › narrow: a cluster panel loading via select then delta shows a spinner, never the empty state (#606) (201ms)
  ✓  26 scripts/playwright-webview-smoke.spec.ts:417:11 › VSIX webview bundles › narrow: a cluster panel loading via analysis running, then select and snapshot shows a spinner, never the empty state (#606) (206ms)
  ✓  27 scripts/playwright-webview-smoke.spec.ts:417:11 › VSIX webview bundles › narrow: a cluster panel loading via analysis errored, then select and snapshot shows a spinner, never the empty state (#606) (204ms)
  ✓  28 scripts/playwright-webview-smoke.spec.ts:417:11 › VSIX webview bundles › narrow: a cluster panel loading via facet filter and filters first shows a spinner, never the empty state (#606) (205ms)
  ✓  29 scripts/playwright-webview-smoke.spec.ts:443:7 › VSIX webview bundles › a report with no clusters ends the loading state with the empty state, not a spinner forever (#606) (179ms)
  ✓  30 scripts/playwright-webview-smoke.spec.ts:455:7 › VSIX webview bundles › keyboard: ? toggles help, j / k move the focused location, Enter opens it (189ms)
  ✓  31 scripts/playwright-webview-smoke.spec.ts:484:7 › VSIX webview bundles › selecting a cluster renders its detail, never the empty state (#254) (170ms)
  ✓  32 scripts/playwright-webview-smoke.spec.ts:501:7 › VSIX webview bundles › navigating to a cluster whose canonical is unsaved keeps every peer comparable (307ms)
  ✓  33 scripts/playwright-webview-smoke.spec.ts:571:7 › VSIX webview bundles › a cluster renders no pair scores with or without a signal source (166ms)

  33 passed (7.8s)

> deslop-webview-ui@0.0.0-dev build
> node build.mjs


Webview line coverage by file:
    92.5%  components/FilterSelect.tsx
    97.7%  report/main.tsx
    99.2%  components/HelpBubble.tsx
    99.2%  duplication/main.tsx
   100.0%  store.ts
   100.0%  theme.ts
   100.0%  components/MetricHeading.tsx
   100.0%  components/MetricContext.tsx
   100.0%  components/ClusterBadge.tsx
   100.0%  components/LanguageBadge.tsx
   100.0%  cluster/OccurrenceList.tsx
   100.0%  cluster/main.tsx


```
Coverage summary:

```json
{
  "lines": {
    "total": 1258,
    "covered": 1248,
    "skipped": 0,
    "pct": 99.2
  },
  "statements": {
    "total": 1258,
    "covered": 1248,
    "skipped": 0,
    "pct": 99.2
  },
  "functions": {
    "total": 78,
    "covered": 69,
    "skipped": 0,
    "pct": 88.46
  },
  "branches": {
    "total": 255,
    "covered": 225,
    "skipped": 0,
    "pct": 88.23
  },
  "branchesTrue": {
    "total": 0,
    "covered": 0,
    "skipped": 0,
    "pct": "Unknown"
  }
}
```
