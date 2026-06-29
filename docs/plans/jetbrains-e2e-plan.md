# JetBrains E2E Plan

## Scope

Add real Android Studio / IntelliJ end-to-end coverage for the single LSP4IJ
plugin — the analogue of the VS Code `@vscode/test-cli` suite, which launches a
real editor against a fixture repo with the real bundled binaries and asserts the
rendered output.

Repository testing rules apply: no fake LSP/MCP and no UI tests that simulate the
server. Tests must launch the real `deslop-lsp` binary against fixture workspaces.

## Harness

The IntelliJ analogue of `@vscode/test-electron` is the IntelliJ Platform Gradle
Plugin's `intellijPlatformTesting` test tasks driving a real IDE — either the
**Starter framework** (`com.jetbrains.intellij.tools:ide-starter`) with the Driver
for a launched sandbox IDE, or a headless `BasePlatformTestCase` that starts the
LSP4IJ server against a fixture. Pin the IDE + LSP4IJ versions in the Gradle
project, stage the real `deslop-lsp`, and assert.

## Test Targets

- Plugin loads on the target platform version (`since-build` 243). *(Descriptor +
  structure tests guard the registrations today; a launch test would prove load.)*
- Opening a supported parser file (`.cs`, `.rs`, `.py`, `.dart`, `.js`, `.mjs`, `.cjs`, `.jsx`, `.ts`, or `.tsx`) starts `deslop-lsp`.
- Native diagnostics appear for a known duplicate fixture.
- Code lens is available when the platform exposes it.
- Settings changes restart or reconfigure the LSP with the expected arguments.
- The **Deslop** tool window renders the report; rows match canonical order once
  native panels land (see [jetbrains-ux-plan.md](jetbrains-ux-plan.md)).

## TODO

- [x] Build `deslop-lsp` for tests instead of relying on a developer-local
      binary. (`make _jetbrains-real-binary-test` + the CI `jetbrains` job build
      the release binary and run `DeslopRealBinaryContractTest` against it.)
- [x] Pin the descriptor surface: `DeslopPluginDescriptorTest` asserts the shipped
      `plugin.xml` registers the tool window, render service, action, notification
      group, and LSP4IJ server — the cheap regression guard for "the plugin shows
      nothing."
- [x] Prove the report panel against a real IDE Application:
      `DeslopReportPanelTest` boots a headless Application (`TestApplicationManager`)
      and asserts `DeslopReportPanel` builds, accepts the engine's report HTML
      without error, and degrades to a readable message when JCEF is unavailable.
      Application-only on purpose — a project fixture runs background indexing whose
      SVG parser is absent from the slim test classpath.
- [x] Prove the live-refresh wiring: `DeslopLanguageClientTest` pins the
      `@JsonNotification("deslop/reportChanged")` handler and the
      `createLanguageClient` override, so the server→panel reactive leg cannot
      silently regress. (Server emission is covered by deslop-lsp's `cache_seed` /
      `execute_command` tests.)
- [x] Prove the rendered report carries real content, not an empty shell:
      `execute_command.rs` asserts the `renderHtmlReport` output contains a populated
      `cluster-card` for the `csharp-small` clone (title, two-occurrence count, a
      real source path) — the exact artifact the panel displays.
- [x] Wire the suite into a Make target and CI: `make _jetbrains-test` runs both
      `:deslop-shared:test` and `:deslop-lsp4ij:test`; the CI `jetbrains` job runs
      the real-binary contract proof.
- [ ] Full-IDE launch (Starter or project-backed `BasePlatformTestCase`) once the
      slim-classpath SVG-indexing gap is resolved: copy a duplicate fixture to a
      temp workspace, launch Android Studio / IntelliJ with the plugin + bundled
      `deslop-lsp`, and assert native diagnostics plus the settings-derived LSP
      arguments. (Today's Application-level test proves the panel; this proves the
      end-to-end LSP4IJ pipeline in a running IDE.)
