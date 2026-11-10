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
