# Deslop JetBrains Plugin

IntelliJ Platform plugin for Deslop. Rider is the first product target, but the implementation stays on the platform LSP API so IntelliJ IDEA, PyCharm, WebStorm, RustRover, and CLion can follow (commercial Ultimate-tier IDEs only — the plugin depends on `com.intellij.modules.ultimate`, so Community editions cannot load it).

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
