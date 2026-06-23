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
- Opening a supported `.cs`/`.rs`/`.py`/`.dart` file starts `deslop-lsp`.
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
      `plugin.xml` registers the tool window, render service, action, and LSP4IJ
      server — the cheap regression guard for "the plugin shows nothing."
- [ ] Choose the IDE-launch harness (Starter vs headless `BasePlatformTestCase`)
      and pin versions in the Gradle project.
- [ ] Add a fixture workspace copied to a temp directory for each test run.
- [ ] Launch Android Studio / IntelliJ with the plugin + bundled `deslop-lsp`.
- [ ] Assert native diagnostics on a duplicate fixture.
- [ ] Assert the LSP process is launched with settings-derived arguments.
- [ ] Assert the Deslop tool window renders the report; add row assertions after
      the native panels land.
- [ ] Wire the E2E suite into a Make target and CI job that can run where the
      JetBrains test environment is available.
