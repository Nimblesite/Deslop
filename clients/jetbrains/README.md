# Deslop JetBrains Plugin

IntelliJ Platform plugin for Deslop, shipped as **a single artifact** that runs across every JetBrains IDE family via Red Hat's [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) client:

- **`deslop-lsp4ij`** — depends only on `com.intellij.modules.platform` + `com.redhat.devtools.lsp4ij`, so the one build reaches **Android Studio and IntelliJ Community** (which do not ship the platform's native LSP API) and, with the LSP4IJ plugin installed, **Rider / IntelliJ IDEA Ultimate** too.

The surface module shares one `deslop-shared` module (binary resolution, settings, launch) and starts the `deslop-lsp` server, so the live analysis is identical everywhere.

[![The Deslop VS Code reference client on a live workspace: a worst-first Top Offenders tree and a per-directory Duplication breakdown in the sidebar, a live clone warning in the editor, and a side-by-side Compare diff against the canonical occurrence.](../../site/src/assets/img/screenshot.webp)](https://deslop.live/docs/vscode-cluster-panel/)

The screenshot above is the **VS Code reference client**. The JetBrains plugins start the same `deslop-lsp` server, so they surface the identical live analysis through each IDE's LSP pipeline. Full panel-by-panel walkthrough: [VS Code Cluster Panel](https://deslop.live/docs/vscode-cluster-panel/).

## Using the plugin

Once installed, the plugin runs automatically — minimal configuration:

1. **Open a supported file** (`.cs`, `.rs`, `.py`, or `.dart`). This starts `deslop-lsp`. Nothing runs until a supported file is open — opening a project alone does nothing.
2. **Read the live warnings.** Duplicated regions are underlined in the editor and listed in the **Problems** tool window with source `deslop`.
3. **Open the full report.** Click the **Deslop** tool window (right-hand stripe) — it shows the worst-offenders report and renders it on first open. It then **refreshes live**: as you edit, the server pushes `deslop/reportChanged` and the panel re-renders in place (without stealing focus), exactly like the VS Code client. The toolbar **Refresh** button forces a full re-analysis, and `Tools` → **Deslop: Open HTML Report** opens the same tool window.

> Can't find the panel? It's on the **right** edge, not the bottom-left `…` overflow. Open it with **Find Action** (`Cmd/Ctrl+Shift+A` → type `Deslop: Open HTML Report`) or `View → Tool Windows → Deslop`.

Deslop only flags *duplicated* code, so a project with no clones shows no warnings and an empty report — that is the correct result, not a failure.

To confirm the server is running, open the **Language Servers** tool window (provided by LSP4IJ) and check that **Deslop** is started. If it is stopped with a binary-resolution error, the bundled `deslop-lsp` was not staged into the zip — rebuild with `DESLOP_BINARY_DIR` set (see the local smoke path below).

Modules:

- **`deslop-shared`** — binary resolution, settings, the `deslop-lsp` command line, and the shared report UI. Compiled against IntelliJ IDEA Community 2024.3 (the declared `since-build` floor) using only `com.intellij.modules.platform` APIs, so it loads in every IDE family. Owns the tests.
- **`deslop-lsp4ij`** — registers a `com.redhat.devtools.lsp4ij.LanguageServerFactory` mapped to `*.cs;*.rs;*.py;*.dart`, plus the **Deslop** tool window.

The surface starts `deslop-lsp` for `.cs`, `.rs`, `.py`, and `.dart` files, resolves the binary from the bundled plugin `bin/<platform>/` directory first (then `PATH`), and launches with embeddings off until a settings page and picker land. `DESLOP_BINARY_DIR` (host binary) and `DESLOP_LSP_BUNDLE_DIR` (all-platform release layout) are build-time staging variables, not runtime resolver sources.

Build and verify the plugin zip (the public package gate):

```bash
make jetbrains-package
```

That composes `_jetbrains-build` (builds the zip), `_jetbrains-verify`
(Gradle project + archive-structure checks), and
`scripts/verify-jetbrains-package.mjs`. To build, install, and wire up the
plugin (plus its LSP4IJ dependency) in one step on macOS, use:

```bash
make android-studio-rebuild
```

The granular steps are internal `_`-prefixed targets — hidden from the IDE
task list, run them directly when iterating:

- `make _jetbrains-build` — build the plugin zip.
- `make _jetbrains-verify` — Gradle project + archive-structure checks.
- `make _jetbrains-test` — run the `deslop-shared` resolver tests.
- `make _jetbrains-real-binary-test` — resolver tests plus the real-binary
  contract proof (accepts the released `deslop-lsp`, rejects manifest drift).

Gradle is invoked via the checked-in wrapper at `clients/jetbrains/gradlew`
(or `gradlew.bat` on Windows). A fresh checkout only needs a JDK on PATH —
the wrapper downloads its own Gradle distribution. Override the binary by
setting `GRADLE=/path/to/gradle` if you need a different runtime.

Local smoke path (host platform only):

```bash
cargo build --release -p deslop-lsp
DESLOP_BINARY_DIR="$PWD/target/release" make _jetbrains-build
```

`make _jetbrains-build` writes the single zip:

```text
clients/jetbrains/deslop-lsp4ij/build/distributions/deslop-lsp4ij-*.zip   # all JetBrains IDE families
```

Install it from disk into Android Studio / IntelliJ Community (or Rider / IDEA Ultimate). Every target also needs the [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) plugin installed.
