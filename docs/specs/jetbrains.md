# JetBrains IDEs — IntelliJ Platform plugin

The JetBrains client is a **thin IntelliJ Platform plugin** over `deslop-lsp`. It is not a second analysis engine, not a ReSharper backend, and not a fork of the VSIX UI. The first product target is **Rider**, because Deslop's first production language is C#, but the implementation must stay platform-shaped so the same plugin can later run in IntelliJ IDEA, PyCharm, WebStorm, RustRover, CLion, GoLand, and other JetBrains IDEs that expose the IntelliJ Platform LSP API.

Official platform constraints: JetBrains' LSP API is exposed through `com.intellij.modules.lsp`, stdio support starts at 2023.2, pull diagnostics at 2025.1/2025.2, and code lens at 2026.1. The Deslop plugin targets the 2026.1 platform line for the first public build so diagnostics, hover, and code lens can map to the existing [lsp.md](lsp.md) contract without native reimplementation. See JetBrains' official [Language Server Protocol](https://plugins.jetbrains.com/docs/intellij/language-server-protocol.html) and [IntelliJ Platform Gradle Plugin](https://plugins.jetbrains.com/docs/intellij/configuring-gradle.html) docs.

### [JETBRAINS-PRINCIPLES] Client principles

1. **One engine.** The plugin launches `deslop-lsp` and consumes standard LSP diagnostics, hover, code lens, document links, and commands. No clone detection, ranking, byte-range conversion, or bucket routing lives in Kotlin.
2. **Rider first, platform always.** The plugin is tested first in Rider because C# users are the immediate market, but source code must avoid Rider-only APIs unless a spec section explicitly allows them.
3. **Native first.** JetBrains users should see Deslop through familiar IDE surfaces: editor highlighting, Problems, hover, code lens, status widget, and later a Tool Window. The plugin does not import the VSIX webview UI.
4. **Offline install.** Public plugin zips must include `shipwright.json` and the `deslop-lsp` binary for every supported OS/architecture, because JetBrains Marketplace cannot publish OS-specific plugin zips and activation must not download executable code.
5. **No silent model work.** Startup embeddings follow [LSP-EMBEDDING-CONSENT]. Fresh installs launch with `--embeddings off`; model selection is a user action.

### [JETBRAINS-TARGET] Supported products

The first supported matrix is:

| Tier | Product | Purpose |
|---|---|---|
| Primary | Rider 2026.1+ | First real user target for C# duplication. |
| Build baseline | IntelliJ Platform 2026.1+ | Keeps the plugin on platform LSP APIs rather than Rider-only APIs. |
| Smoke later | IntelliJ IDEA, PyCharm, WebStorm, RustRover, CLion | Validate the same plugin as Rust/Python support matures. |

The plugin descriptor depends on `com.intellij.modules.lsp` and `com.intellij.modules.ultimate`, matching the platform LSP requirement. Android Studio is out of scope because JetBrains does not expose this LSP integration there.

### [JETBRAINS-LSP] LSP server integration

`clients/jetbrains` registers one `LspServerSupportProvider` through `com.intellij.platform.lsp.serverSupportProvider`. When a supported file opens (`.cs`, `.rs`, `.py`), the provider starts a project-wide `ProjectWideLspServerDescriptor` named `Deslop`.

The descriptor launches:

```text
deslop-lsp <workspace-root> --min-nodes <n> --embeddings <mode>
  --embedding-provider <provider> --embedding-model <model>
  --embedding-endpoint <endpoint>
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
- Language Services status widget entry named `Deslop`.

Post-scaffold UX:

- Tool Window named `Duplicate Clusters` with Top Offenders, Focused File, and Session tabs.
- Worst-offender action from Search Everywhere / Find Action.
- Embedding model picker using JetBrains' native popup list.
- Compare-with-canonical action using the IDE diff viewer.

The Tool Window consumes the canonical `Report` from `deslop/reportGet`; it never re-ranks clusters or recomputes buckets.

### [JETBRAINS-MCP] MCP relationship

The JetBrains plugin does not embed MCP in v1. Agents inside Rider can use `deslop-mcp` through their own MCP host, while the IDE plugin owns human editor feedback through LSP. A later JetBrains-specific MCP bridge may be added only if there is a concrete agent host inside JetBrains IDEs that cannot launch the normal `deslop-mcp` binary.

### [JETBRAINS-PACKAGING] Packaging

`clients/jetbrains` builds a JetBrains plugin zip through the IntelliJ Platform Gradle Plugin. GitHub Release packaging eventually attaches:

```text
deslop-jetbrains-<version>.zip
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
