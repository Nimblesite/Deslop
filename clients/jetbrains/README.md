# Deslop JetBrains Plugin

IntelliJ Platform plugin for Deslop, built as **two artifacts from one codebase** so it runs across every JetBrains IDE family:

- **`deslop-ultimate`** — the platform's native LSP API (`com.intellij.modules.ultimate` + `com.intellij.modules.lsp`). Rider and IntelliJ IDEA Ultimate.
- **`deslop-lsp4ij`** — Red Hat's [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) client (`com.intellij.modules.platform` + `com.redhat.devtools.lsp4ij`), reaching **Android Studio and IntelliJ Community**, which do not ship the native LSP API.

Both surfaces share one `deslop-shared` module (binary resolution, settings, launch) and start the same `deslop-lsp` server, so the live analysis is identical everywhere.

[![The Deslop VS Code reference client on a live workspace: a worst-first Top Offenders tree and a per-directory Duplication breakdown in the sidebar, a live clone warning in the editor, and a side-by-side Compare diff against the canonical occurrence.](../../site/src/assets/img/screenshot.webp)](https://deslop.live/docs/vscode-cluster-panel/)

The screenshot above is the **VS Code reference client**. The JetBrains plugins start the same `deslop-lsp` server, so they surface the identical live analysis through each IDE's LSP pipeline. Full panel-by-panel walkthrough: [VS Code Cluster Panel](https://deslop.live/docs/vscode-cluster-panel/).

Modules:

- **`deslop-shared`** — binary resolution, settings, and the `deslop-lsp` command line. Compiled against the unified IntelliJ IDEA base using only `com.intellij.modules.platform` APIs, so it loads in every IDE family. Owns the tests.
- **`deslop-ultimate`** — registers a `com.intellij.platform.lsp.serverSupportProvider`.
- **`deslop-lsp4ij`** — registers a `com.redhat.devtools.lsp4ij.LanguageServerFactory` mapped to `*.cs;*.rs;*.py;*.dart`.

Both surfaces start `deslop-lsp` for `.cs`, `.rs`, `.py`, and `.dart` files, resolve the binary from the bundled plugin `bin/<platform>/` directory first (then `PATH`), and launch with embeddings off until a settings page and picker land. `DESLOP_BINARY_DIR` (host binary) and `DESLOP_LSP_BUNDLE_DIR` (all-platform release layout) are build-time staging variables, not runtime resolver sources.

Build:

```bash
make jetbrains-build
```

Verify plugin metadata and structure:

```bash
make jetbrains-verify
```

Build both release zips and run all local package gates:

```bash
make jetbrains-package
```

Run resolver tests:

```bash
make jetbrains-test
```

Run resolver tests AND prove the resolver accepts the real released
`deslop-lsp` binary plus rejects manifest drift:

```bash
make jetbrains-real-binary-test
```

Gradle is invoked via the checked-in wrapper at `clients/jetbrains/gradlew`
(or `gradlew.bat` on Windows). A fresh checkout only needs a JDK on PATH —
the wrapper downloads its own Gradle distribution. Override the binary by
setting `GRADLE=/path/to/gradle` if you need a different runtime.

Local Rider smoke path:

```bash
cargo build --release -p deslop-lsp
DESLOP_BINARY_DIR="$PWD/target/release" make jetbrains-build
```

`make jetbrains-build` writes both zips:

```text
clients/jetbrains/deslop-ultimate/build/distributions/deslop-ultimate-*.zip   # Rider / IDEA Ultimate
clients/jetbrains/deslop-lsp4ij/build/distributions/deslop-lsp4ij-*.zip        # Android Studio / Community
```

Install the matching one from disk: the `deslop-ultimate` zip into Rider 2026.1+, or the `deslop-lsp4ij` zip into Android Studio / IntelliJ Community (which also needs the [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) plugin installed).
