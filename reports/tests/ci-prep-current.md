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
