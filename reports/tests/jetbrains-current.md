# JetBrains test verification

Commands (offline, one Gradle worker):

- `DESLOP_LSP_REAL_BINARY=<repo>/target/release/deslop-lsp ./gradlew :deslop-shared:test :deslop-lsp4ij:test :deslop-lsp4ij:integrationTest --offline --no-daemon --max-workers=1`
- `DESLOP_LSP_REAL_BINARY=<repo>/target/release/deslop-lsp ./gradlew :deslop-shared:test --offline --no-daemon --max-workers=1 --rerun-tasks`

Both exited **0**. The second invocation forces the real-binary contract to execute rather than reusing cached test results.

Source: `c9a59782f044e6958db627e21472b06dadbc9dc7`.

| Suite | Tests | Failures | Errors | Skipped |
| --- | ---: | ---: | ---: | ---: |
| deslop-shared:test | 36 | 0 | 0 | 0 |
| deslop-lsp4ij:test | 2 | 0 | 0 | 0 |
| deslop-lsp4ij:integrationTest | 1 | 0 | 0 | 0 |

## Real-binary assertions

- realBinaryIsRejectedWhenManifestExpectsDifferentVersion()
- realBinaryResolvesAgainstRepoManifest()

Result XML SHA-256: `b047d53a9104ccd16bae1fc89eb65f2cedf1bb7ec40cd848ab3af26aadacda10`.

## Java 21 and complete package verification

Both modules now use Java 21, matching CI and the IntelliJ 2024.3 requirement. Local verification uses the existing Homebrew JDK 21 with offline dependency resolution.

Combined Gradle invocation: `:deslop-shared:test :deslop-lsp4ij:test :deslop-lsp4ij:integrationTest :deslop-lsp4ij:buildPlugin verifyPluginProjectConfiguration verifyPluginStructure --offline --no-daemon --max-workers=1`. Exit status: **0**.

The prior packaging invocation failed Gradle dependency validation because `verifyPluginStructure` consumed the binary-copy task output without a declared dependency. Adding that dependency makes the unchanged combined invocation pass.

```text
> Task :deslop-lsp4ij:buildPlugin
> Task :deslop-lsp4ij:verifyPluginProjectConfiguration
> Task :deslop-shared:verifyPluginProjectConfiguration
> Task :deslop-lsp4ij:verifyPluginStructure

BUILD SUCCESSFUL in 1m 42s
43 actionable tasks: 32 executed, 1 from cache, 10 up-to-date
Configuration cache entry stored.
Verified JetBrains package /Users/christianfindlay/Documents/Code/Deslop/clients/jetbrains/deslop-lsp4ij/build/distributions/deslop-lsp4ij-0.1.0.zip for darwin-arm64
```

- `deslop-shared:test`: 36 tests, no failures or errors.

- `deslop-lsp4ij:test`: 2 tests, no failures or errors.

- `deslop-lsp4ij:integrationTest`: 1 tests, no failures or errors.
