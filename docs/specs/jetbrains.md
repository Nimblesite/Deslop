# JetBrains IDEs — IntelliJ Platform plugin

The JetBrains client is a **thin IntelliJ Platform plugin** over `deslop-lsp`. It is not a second analysis engine, not a ReSharper backend, and not a fork of the VSIX UI. It ships as a **single artifact built on Red Hat's [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) client**, which every JetBrains IDE family supports: **Android Studio and IntelliJ Community** (which do not expose the platform's native LSP API) and — with the LSP4IJ plugin installed — Rider, IntelliJ IDEA Ultimate, PyCharm, WebStorm, RustRover, CLion, and GoLand. Android Studio is a first-class target because it is Flutter/Dart's home IDE. The implementation stays platform-shaped: it uses only `com.intellij.modules.platform` APIs plus LSP4IJ, never Rider-only or Ultimate-only APIs.

Official platform constraints: the IntelliJ Platform's *native* LSP API (`com.intellij.modules.lsp`) is Ultimate/Rider-only and gates pull diagnostics and code lens behind newer platform versions — which is exactly why Deslop does not use it. LSP4IJ provides diagnostics, hover, code lens, and `workspace/executeCommand` on its own, independent of the host IDE's platform version, so the plugin maps to the existing [lsp.md](lsp.md) contract without native reimplementation and runs on the **2024.3 platform line (build 243) and newer** — the floor every shipping Android Studio satisfies. See JetBrains' official [Language Server Protocol](https://plugins.jetbrains.com/docs/intellij/language-server-protocol.html) and [IntelliJ Platform Gradle Plugin](https://plugins.jetbrains.com/docs/intellij/configuring-gradle.html) docs.

### [JETBRAINS-PRINCIPLES] Client principles

1. **One engine.** The plugin launches `deslop-lsp` and consumes standard LSP diagnostics, hover, code lens, document links, and commands. No clone detection, ranking, byte-range conversion, or bucket routing lives in Kotlin.
2. **One artifact, platform always.** A single LSP4IJ-based plugin serves every IDE family. Source code uses only `com.intellij.modules.platform` APIs plus LSP4IJ — never Rider-only or Ultimate-only APIs — so the one build loads everywhere.
3. **Native surfaces.** JetBrains users see Deslop through familiar IDE surfaces: editor highlighting, Problems, hover, code lens, the LSP4IJ status surface, and a **Deslop Tool Window** hosting the live report. The plugin renders the engine's HTML report in an embedded browser; it does not import the VSIX webview UI.
4. **Offline install.** Public plugin zips must include `shipwright.json` and the `deslop-lsp` binary for every supported OS/architecture, because JetBrains Marketplace cannot publish OS-specific plugin zips and activation must not download executable code.
5. **No silent model work.** Startup embeddings follow [LSP-EMBEDDING-CONSENT]. Fresh installs launch with `--embeddings off`; model selection is a user action.

### [JETBRAINS-TARGET] Supported products

The first supported matrix is:

| Tier | Product | Purpose |
|---|---|---|
| Primary | Android Studio 2024.3+ (Meerkat) | Flutter/Dart's home IDE; the first real Android Studio target. |
| Build baseline | IntelliJ Platform 2024.3+ (build 243) | The compile base and `since-build` floor; keeps the plugin on `com.intellij.modules.platform` + LSP4IJ APIs. |
| Also supported | IntelliJ Community, Rider, IDEA Ultimate, PyCharm, WebStorm, RustRover, CLion, GoLand | The same single artifact, with the LSP4IJ plugin installed. |

Deslop ships **one plugin artifact**:

- **`deslop-lsp4ij`** (id `nimblesite.deslop.jetbrains.community`) depends on `com.intellij.modules.platform` and Red Hat's `com.redhat.devtools.lsp4ij`. Because LSP4IJ exists in every IDE family, this single build reaches **Android Studio and IntelliJ Community** (which do not expose the native LSP API) as well as Rider/Ultimate. There is deliberately **no** separate native-LSP (`com.intellij.modules.ultimate`) artifact — it would duplicate the bridge for the sole benefit of not requiring LSP4IJ on commercial IDEs, which is not worth a second build to maintain and publish.

Non-surface code — binary resolution, settings, launch, and the shared report view — lives in the `deslop-shared` module bundled into the zip under `lib/modules/`.

### [JETBRAINS-LSP] LSP server integration

`clients/jetbrains` is a Gradle build with two modules: a shared library (`deslop-shared`) and one thin LSP surface over it (`deslop-lsp4ij`) producing the plugin zip. All binary resolution, settings, launch, and report-view logic live in the shared module.

- **LSP4IJ surface (`deslop-lsp4ij`).** Registers a `com.redhat.devtools.lsp4ij.LanguageServerFactory` (extension namespace `com.redhat.devtools.lsp4ij`) plus a `fileNamePatternMapping` of `*.cs;*.rs;*.py;*.dart` to that server, launching `deslop-lsp` through an `OSProcessStreamConnectionProvider`. A test asserts the glob equals the shared supported-extension set so the two cannot drift.

The surface launches the binary with the workspace root only — min-node and embedding settings are read by the LSP from `.deslop.toml`, never passed as flags (#83):

```text
deslop-lsp <workspace-root>
```

Initial scope:

- `textDocument/diagnostic` lights up duplicate occurrences through JetBrains' native error/warning/highlight pipeline.
- `textDocument/hover` displays cluster id, interpretation, signals, and occurrences once the LSP implementation provides them.
- `textDocument/codeLens` carries inline clone summaries on 2026.1+ IDEs.

The plugin must not parse hover markdown to recover structured data. Native Tool Window and settings work must call the `deslop/*` custom LSP methods once the IntelliJ LSP client wrapper is extended for custom requests.

### [JETBRAINS-BINARY] Binary resolution

The plugin loads `shipwright.json` from the plugin root before it registers or starts the LSP descriptor. `hosts.jetbrains.activationVerifies` is the authority for required startup components; the first public build verifies `deslop-lsp` for the current platform before any LSP process is launched.

Resolver inputs mirror [DEPLOY-RESOLVER] and [VSIX-BINARY-VERSIONING]:

1. Explicit user-configured `deslop-lsp` path, once the settings UI exposes it.
2. `DESLOP_LSP_PATH`.
3. `DESLOP_BINARY_DIR/deslop-lsp[.exe]`.
4. Bundled `bin/<platform>/deslop-lsp[.exe]` inside the plugin zip.
5. `deslop-lsp[.exe]` on `PATH` for Homebrew/Scoop/system installs.

Each candidate is executed directly, without shell interpolation, as `deslop-lsp --version`; the first stdout line must be exactly `deslop-lsp <expectedVersion>` from the manifest. The JSON version output and LSP `initialize` `serverInfo.version` must match the same expected version per [DEPLOY-VERSION-CONTRACT] and [DEPLOY-PROTOCOL-VERSION].

An explicit configured path or environment path that resolves to the wrong binary, wrong version, or non-executable file blocks LSP startup and reports a clear JetBrains notification/Event Log entry. A stale `PATH` binary does not override a matching bundled binary. A bundled mismatch blocks startup because the plugin package is invalid.

Release packaging must stage:

| Platform | Directory |
|---|---|
| macOS arm64 | `bin/darwin-arm64/` |
| macOS x64 | `bin/darwin-x64/` |
| Linux x64 | `bin/linux-x64/` |
| Linux arm64 | `bin/linux-arm64/` |
| Windows x64 | `bin/win32-x64/` |

### [JETBRAINS-SETTINGS] Settings contract

The plugin persists project-level Deslop settings through `DeslopSettings` and
validates them before building the `deslop-lsp` launch command. The stored
contract mirrors the VSIX setting names so workspace state stays portable:

- `deslop.minNodes`
- `deslop.embedding.provider`
- `deslop.embedding.model`
- `deslop.embedding.endpoint`
- `deslop.embedding.mode`
- `deslop.incremental`

Fresh installs keep `deslop.embedding.mode = off`; the model picker or a future
settings UI must be the user action that flips it to `auto` or `required`.
Invalid `minNodes`, provider ids, endpoint URLs, blank model ids, and embedding
modes must block startup before the LSP process is launched.

When the plugin adds model selection, it must persist the same workspace embedding settings described in [LSP-EMBEDDING-CONSENT]. The LSP and MCP must still converge through one setting contract.

### [JETBRAINS-UX] Native IDE surfaces

First public UX:

- Editor highlighting via LSP diagnostics.
- Problems panel entries with `source = "deslop"` and stable cluster ids.
- Hover detail and code lens when provided by the LSP.
- LSP4IJ **Language Servers** tool window entry named `Deslop` (start/stop + status).
- A **Deslop** tool window hosting the live worst-offenders report, rendered by the engine and shown in an embedded JCEF browser. A toolbar **Refresh** action re-runs `deslop.lsp.renderHtmlReport`. The same `Tools → Deslop: Open HTML Report` action opens/refreshes this tool window, and the JCEF view is shared with the editor-tab renderer ([DeslopReportBrowser]) so report rendering is never duplicated.

Post-scaffold UX:

- A richer native-tree tool window with Top Offenders, Focused File, and Session tabs, consuming the canonical `Report` from `deslop/reportGet` instead of HTML — it would never re-rank clusters or recompute buckets.
- Worst-offender action from Search Everywhere / Find Action.
- Embedding model picker using JetBrains' native popup list.
- Compare-with-canonical action using the IDE diff viewer.

### [JETBRAINS-MCP] MCP relationship

The JetBrains plugin does not embed MCP in v1. Agents inside Rider can use `deslop-mcp` through their own MCP host, while the IDE plugin owns human editor feedback through LSP. A later JetBrains-specific MCP bridge may be added only if there is a concrete agent host inside JetBrains IDEs that cannot launch the normal `deslop-mcp` binary.

### [JETBRAINS-PACKAGING] Packaging

`clients/jetbrains` builds **one** JetBrains plugin zip through the IntelliJ Platform Gradle Plugin, bundling the shared `deslop-shared` jar under `lib/modules/`. GitHub Release packaging attaches it:

```text
deslop-lsp4ij-<version>.zip   # LSP4IJ — all JetBrains IDE families
```

The plugin zip includes `shipwright.json` at the plugin root and any native helpers only under manifest-approved `bin/<platform>/` directories. Package verification must prove the manifest is present, required binaries exist for each shipped platform, no undeclared executable is present under `bin/<platform>/`, and each binary reports the manifest `expectedVersion`.

The version is lock-step with the Rust binaries and VSIX. Marketplace publishing is manual until publisher credentials, signing, package verification, and approval flow are set up.

### [JETBRAINS-TESTING] Testing

Testing follows the repository rule: **no fake LSP/MCP**. Acceptable test layers:

- Gradle `verifyPluginProjectConfiguration` and `verifyPluginStructure`.
- Headless IntelliJ Platform tests for pure Kotlin helpers like binary resolution.
- Rider/IntelliJ UI tests that launch the real `deslop-lsp` binary against fixture workspaces and assert native IDE diagnostics or Tool Window rows.
- Manifest-backed startup tests cover environment directory, `PATH`, bundled success, missing binary, component-name mismatch, version mismatch, and notification/Event Log reporting.
- Plugin archive package tests prove the root manifest and manifest-declared platform binaries are present and no undeclared native executable is shipped.

The first scaffold may ship with structure verification only. Before a public plugin zip, CI must exercise at least one real IDE test path against the real `deslop-lsp` binary.
