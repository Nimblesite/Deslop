# Deslop JetBrains Plugin

IntelliJ Platform plugin for Deslop. Rider is the first product target, but the implementation stays on the platform LSP API so IntelliJ IDEA, PyCharm, WebStorm, RustRover, and CLion can follow.

Current slice:

- Registers a `com.intellij.platform.lsp.serverSupportProvider`.
- Starts `deslop-lsp` for `.cs`, `.rs`, and `.py` files.
- Resolves the binary from `${DESLOP_BINARY_DIR}`, `PATH`, bundled plugin `bin/<platform>/`, then bare `deslop-lsp`.
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

The Makefile uses `gradle` from `PATH` when available. On Unix hosts without a
PATH install it falls back to a cached Gradle 9.0.0 distribution under
`~/.gradle/wrapper/dists`; set `GRADLE=/path/to/gradle` to override it.

Local Rider smoke path:

```bash
cargo build --release -p deslop-lsp
DESLOP_BINARY_DIR="$PWD/target/release" make jetbrains-build
```

Then install `clients/jetbrains/build/distributions/deslop-jetbrains-*.zip` into Rider 2026.1+ from disk.
