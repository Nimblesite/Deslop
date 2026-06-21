# JetBrains E2E Plan

## Scope

Add real Rider / IntelliJ end-to-end coverage for the JetBrains plugin.

Repository testing rules apply: no fake LSP/MCP and no UI tests that simulate
the server. Tests must launch the real `deslop-lsp` binary against fixture
workspaces.

## Test Targets

- Plugin loads on the target IntelliJ Platform version.
- Opening a supported `.cs`, `.rs`, `.py`, or `.dart` file starts `deslop-lsp`.
- Native diagnostics appear for a known duplicate fixture.
- Hover and code lens are available when the platform exposes them.
- Settings changes restart or reconfigure the LSP with the expected arguments.
- Tool Window rows match the canonical report order once the Tool Window lands.

## TODO

- [ ] Choose the JetBrains UI test harness and pin versions in the Gradle
      project.
- [ ] Add a fixture workspace copied to a temp directory for each test run.
- [x] Build `deslop-lsp` for tests instead of relying on a developer-local
      binary. (`make _jetbrains-real-binary-test` + the CI `jetbrains` job build
      the release binary and run `DeslopRealBinaryContractTest` against it.)
- [ ] Launch Rider or IntelliJ with the plugin installed from the build output.
- [ ] Assert native diagnostics on the C# duplicate fixture.
- [ ] Assert the LSP process is launched with settings-derived arguments.
- [ ] Add Tool Window assertions after the native UX plan lands.
- [ ] Wire the E2E suite into a Make target and CI job that can run where the
      JetBrains test environment is available.
