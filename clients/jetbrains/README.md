# Deslop JetBrains Plugin

IntelliJ Platform plugin for Deslop. Rider is the first product target, but the implementation stays on the platform LSP API so IntelliJ IDEA, PyCharm, WebStorm, RustRover, and CLion can follow (commercial Ultimate-tier IDEs only — the plugin depends on `com.intellij.modules.ultimate`, so Community editions cannot load it).

[![The Deslop VS Code reference client on a live workspace: a worst-first Top Offenders tree and a per-directory Duplication breakdown in the sidebar, a live clone warning in the editor, and a side-by-side Compare diff against the canonical occurrence.](../../site/src/assets/img/screenshot.webp)](https://deslop.live/docs/vscode-cluster-panel/)

The screenshot is the **VS Code reference client** — the sidebar's worst-first **Top Offenders** tree and per-folder **Duplication** breakdown (left), the editor's live clone warning naming the canonical copy (centre), and the **Compare** diff against that canonical occurrence (right). This JetBrains plugin starts the same `deslop-lsp` server, so it surfaces the identical live analysis through the IDE's native LSP diagnostics. Full panel-by-panel walkthrough: [VS Code Cluster Panel](https://deslop.live/docs/vscode-cluster-panel/).

Current slice:

- Registers a `com.intellij.platform.lsp.serverSupportProvider`.
- Starts `deslop-lsp` for `.cs`, `.rs`, and `.py` files.
- Resolves the `deslop-lsp` binary from the bundled plugin `bin/<platform>/` directory first, then falls back to `PATH`. (`DESLOP_BINARY_DIR` is a build-time staging variable used to embed the binary into the plugin zip, not a runtime resolver source.)
- Launches with embeddings off until a settings page and picker land.

Build:

```bash
make jetbrains-build
```

Verify plugin metadata and structure:

```bash
make jetbrains-verify
```

Build the release zip and run all local package gates:

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

Then install `clients/jetbrains/build/distributions/deslop-jetbrains-*.zip` into Rider 2026.1+ from disk.
