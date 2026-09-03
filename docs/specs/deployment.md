# Deployment contract

Deslop adopts Deployment Toolkit as the release authority for every shippable
binary and IDE package. This spec covers the product contract introduced by
GitHub issues #37, #38, #39, #40, and #41.

### [DEPLOY-MANIFEST] Manifest authority

`shipwright.json` at the repository root is the single source of truth
for deployable components, expected versions, host startup checks, package
contents, and CI release gates.

The Deslop manifest must describe, at minimum:

- `deslop` as the CLI component.
- `deslop-lsp` as the LSP component.
- `deslop-mcp` as the MCP component.
- `deslop-vscode` as the VS Code extension component.
- `deslop-jetbrains` as the JetBrains extension component.
- `hosts.vscode.activationVerifies = ["deslop-lsp", "deslop-mcp"]`.
- `hosts.jetbrains.activationVerifies = ["deslop-lsp"]`.
- `hosts.cli.activationVerifies = ["deslop"]`.

Release code must not maintain a second hand-written manifest for VSIX,
JetBrains, CI, or package verification. Derived copies inside artifacts must be
the same contract.

### [DEPLOY-VERSION-CONTRACT] Binary version contract

Each required executable component must support:

```text
<binary> --version
```

The first stdout line must be exact:

```text
<component-id> <semantic-version>
```

For Deslop `0.1.0`, that means:

- `deslop 0.1.0`
- `deslop-lsp 0.1.0`
- `deslop-mcp 0.1.0`

The version path must exit before tracing setup, workspace parsing, LSP startup,
MCP startup, file scanning, cache writes, or network access.

Each required executable must also support:

```text
<binary> --version --json
```

The JSON output must validate against Deployment Toolkit's
`schemas/version-manifest.schema.json` and include at minimum
`manifestVersion`, `name`, `version`, `kind`, `language`, and
`product = "deslop"`.

### [DEPLOY-PROTOCOL-VERSION] Protocol initialize version

Long-running protocol binaries must report the same version during initialize
as they report through `--version`.

- `deslop-lsp` must set `InitializeResult.serverInfo.name = "deslop-lsp"` and
  `InitializeResult.serverInfo.version` to the manifest version.
- `deslop-mcp` must set initialize `serverInfo.name = "deslop-mcp"` and
  `serverInfo.version` to the manifest version.

Tests must fail if package version, manifest version, plain version output,
JSON version output, or protocol metadata drift apart.

### [DEPLOY-RESOLVER] Host resolver contract

IDE hosts must load `shipwright.json` before reporting ready or
starting required integrations. The host then reads its
`activationVerifies` list and verifies every required component for the current
platform.

Resolver inputs are not bypasses. A configured path, environment path,
environment directory, PATH candidate, or bundled binary must still prove the
expected component id and version.

For Deslop's migration issues, mismatch behavior is:

- An explicit user setting mismatch is a hard activation error.
- `DESLOP_LSP_PATH`, `DESLOP_MCP_PATH`, and `DESLOP_BINARY_DIR` mismatches are
  hard activation errors for required components.
- VS Code does not probe PATH for activation binaries. It resolves explicit
  user/env overrides or the bundled path under the installed extension.
- A mismatched bundled binary is a package/release failure and blocks
  activation.

Errors must include product/extension version, component id, expected version,
found version or `not found`, candidate path/source, and the next action.

A version probe that never replies is **inconclusive, not a mismatch**. Every
bundled binary in a freshly installed package is on its first execution, and
macOS validates an unsigned multi-megabyte binary before running it
(Gatekeeper / `syspolicyd`) — hundreds of milliseconds, and more on a loaded
machine. A host that reports that stall as a version mismatch fails the very
first activation after install, with an error naming a cause that is not true.
Hosts must therefore retry a timed-out probe once on a budget wide enough for
first-exec validation before concluding anything, and keep the warm budget
tight so a genuinely hung binary is still caught quickly. Only a probe that
replies with the wrong id/version is a mismatch.

### [DEPLOY-VSIX-PACKAGE] VSIX package contract

Each platform-specific VSIX artifact must be built with `vsce package --target`
for the target platform and must include:

- `extension/shipwright.json`.
- `deslop`, `deslop-lsp`, and `deslop-mcp` for exactly one target platform under
  `extension/bin/<platform>/`.
- No undeclared executable under `extension/bin/<platform>/`.
- No binaries for any other platform.

Package tests must inspect the produced `.vsix`, not only the staging
directory. They must fail on a missing manifest, missing binary, extra binary,
non-executable binary where executability is meaningful, or wrong-version
binary.

### [DEPLOY-EXTERNAL-MCP-CONSUMER] External MCP clients consume the VSIX-bundled binary

Every MCP client that runs outside the VS Code host process — Claude Code (CLI),
Claude Desktop, Codex, Cursor, Continue — must reference `deslop-mcp` by an
**absolute path into the unpacked VSIX**:

```
~/.vscode/extensions/nimblesite.deslop-live-<VERSION>-<platform>/bin/<platform>/deslop-mcp
```

The unpacked VSIX is the canonical distribution surface per [DEPLOY-VSIX-PACKAGE].
Pointing an MCP client at any other binary breaks [DEPLOY-VERSION-CONTRACT] +
[DEPLOY-PROTOCOL-VERSION]: a locally-built `target/release/deslop-mcp` would
shadow the shipright-versioned bundle and silently drift the agent's analysis
off the extension's wire contract. PATH-resolved deslop is supported only from
release-locked installers: Homebrew and Scoop, which version the binary
lock-step with the release, and the published fail-closed curl installer
([DEPLOY-DOCS-INSTALLER-FAILCLOSED]), which pins a `DESLOP_TAG` and verifies a
SHA-256 before anything reaches `~/.local/bin`.

Consequences for this repo:

- The Makefile must not provide a target that puts source-built `deslop`,
  `deslop-lsp`, or `deslop-mcp` binaries onto the user's `PATH`. There is no
  `make install-binary` target. `cargo install --path crates/deslop-*` is
  forbidden.
- `make _delete-path-binaries` is invoked from every `_vsix-*` and `test` target
  so a developer machine that previously leaked binaries onto `PATH` is
  scrubbed before tests run, and the rule is verifiable on a fresh checkout.
- Every doc that shows an MCP wiring snippet (`README.md`,
  `clients/vscode/README.md`, `docs/snippets/agents-md-recipe.md`,
  `site/src/docs/ai-integration.md`) leads with the absolute VSIX path and
  documents the release-locked PATH forms (brew/scoop, the published curl
  installer) as the only secondary alternatives.

### [DEPLOY-EXTENSION-BUNDLED-TESTS] Extension tests must use bundled binaries

IDE extension tests must run against binaries bundled inside the extension
artifact or extension development directory. They must not point resolver
environment variables at `target/release`, cargo installs, package-manager
installs, or any other PATH-visible binary.

For VS Code, `vsix-test`, `vsix-coverage`, and `vsix-test-ollama` must stage
`deslop`, `deslop-lsp`, and `deslop-mcp` under
`clients/vscode/bin/<platform>/` before activation, clear
`DESLOP_BINARY_DIR`, `DESLOP_LSP_PATH`, and `DESLOP_MCP_PATH`, and assert that
the resolved LSP and MCP sources are `bundled`.

Before test entry points run, the build must remove cargo-installed Deslop
binaries and fail if `deslop`, `deslop-lsp`, or `deslop-mcp` still resolve on
`PATH`. This keeps extension tests honest: a missing or stale bundle cannot be
masked by a developer machine install.

A rebuild must also scrub *running* Deslop processes before it cleans anything. A `deslop`, `deslop-lsp`, or `deslop-mcp` left behind by an editor session or an abandoned test shadows the bundle the rebuild is about to produce, starves socket-bound integration tests, and on Windows — where the loader keeps an open handle to every running image — stops `cargo clean` from emptying `target/release` at all. Matching is by exact process name, so `cargo build -p deslop-lsp` survives untouched; a process that outlives a forced kill fails the target rather than being ignored. The matching, the terminate-then-force sequence, and the fail-closed re-check live in `scripts/repository/kill-deslop-processes.sh` so they can be tested without the target's destructive side effect, and `scripts/repository/kill-deslop-processes.test.mjs` (a `make lint` gate) drives that detection against a fixture process it owns and reaps.

Every recipe in the `Makefile` is POSIX shell, so Windows runs them under Git Bash, found by absolute path through the overridable `GIT_BASH` variable. Resolving `bash.exe` by name instead finds WSL's copy in `System32`, which mounts a different filesystem and cannot see the checkout; handing the recipes to `powershell.exe` cannot work either, since PowerShell parses none of the `case`, `for`, `[ -f ]`, or `||` the recipes are built from. The same gate asserts both, together with the POSIX spelling of the `RM` and `MKDIR` helpers that recipes interpolate into those constructs.

### [DEPLOY-JETBRAINS-PACKAGE] JetBrains package contract

The JetBrains plugin package must include `shipwright.json` at plugin
root and verify `deslop-lsp` before creating or starting the LSP descriptor.

If the package bundles native helpers under plugin-root `bin/<platform>/`, each
helper must be listed in the manifest. Startup failure must surface through a
JetBrains notification or Event Log entry with the same expected/found/path
details required by [DEPLOY-RESOLVER].

### [DEPLOY-DOCS-INSTALLER-FAILCLOSED] Published curl installer fails closed

The curl installer snippet on the docs pages (`site/src/docs/index.md`, `site/src/zh/docs/index.md`) is a published contract: when version resolution, either download, or SHA-256 verification fails, nothing may be extracted and nothing may be installed. The snippet therefore runs entirely inside one subshell under `set -euo pipefail`, downloads into a `mktemp -d` work directory removed by an `EXIT` trap, and only reaches `tar` and `install` after the checksum check passes. The subshell's exit status must not be tested by the surrounding command (no trailing `&& …`): bash 3.2 — the `/bin/bash` macOS ships — disables `set -e` inside a subshell whose status is tested, which silently reopens the fail-open path. `DESLOP_TAG` pins a release; `DESLOP_RELEASE_BASE` overrides the release mirror and doubles as the offline test seam. The no-sudo alternative creates `~/.local/bin` before installing. Both locales stay functionally identical; comments are the only translated lines. Enforced by `scripts/deployment/installer-snippet.test.mjs` (a `make lint` gate), which runs the exact published snippets against a local fixture release and proves a bad checksum yields a non-zero exit with no extraction, no install, and no leaked work directory.

### [DEPLOY-CI-GATES] CI and release gates

CI and release jobs must fail fast on deployment drift.

Required gates:

- Validate `shipwright.json` with the Deployment Toolkit schema.
- Verify `deslop`, `deslop-lsp`, and `deslop-mcp` plain and JSON version
  output after release binaries are built.
- Verify LSP and MCP initialize metadata against the manifest version.
- Verify the produced VSIX package contents.
- Verify the produced JetBrains package contents before publishing.

When shared `deploy-toolkit` CLI commands are available, Deslop should call
them directly. Until then, product-local tests must prove the same behavior.

**Private Deployment Toolkit docs.**

Deployment Toolkit documentation and fixtures live in the private
`Nimblesite/Shipwright` repository (formerly `MelbourneDeveloper/deployment_toolkit`).
Agents working these issues must use authenticated `gh` access to read the docs
and fixtures; they must not rely on local absolute paths or assume the GitHub
URLs are public.

Relevant private docs include:

- `docs/specs/binary-version-contract.md`
- `docs/specs/ide-extension-deployment.md`
- `docs/specs/acceptance-gates.md`
- `docs/agents/product-repo-adoption-guide.md`
- `docs/plans/product-migration-tickets.md`
- `schemas/version-manifest.schema.json`
- `fixtures/manifests/deslop.json`
