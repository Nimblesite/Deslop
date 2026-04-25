# Deployment contract

Deslop adopts Deployment Toolkit as the release authority for every shippable
binary and IDE package. This spec covers the product contract introduced by
GitHub issues #37, #38, #39, #40, and #41.

### [DEPLOY-MANIFEST] Manifest authority

`deployment-toolkit.json` at the repository root is the single source of truth
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

IDE hosts must load `deployment-toolkit.json` before reporting ready or
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
- A mismatched PATH candidate is skipped when a matching bundled binary exists.
- A mismatched bundled binary is a package/release failure and blocks
  activation.

Errors must include product/extension version, component id, expected version,
found version or `not found`, candidate path/source, and the next action.

### [DEPLOY-VSIX-PACKAGE] VSIX package contract

The VSIX artifact must include:

- `extension/deployment-toolkit.json`.
- `deslop`, `deslop-lsp`, and `deslop-mcp` for the target platform under
  `extension/bin/<platform>/`.
- No undeclared executable under `extension/bin/<platform>/`.

Package tests must inspect the produced `.vsix`, not only the staging
directory. They must fail on a missing manifest, missing binary, extra binary,
non-executable binary where executability is meaningful, or wrong-version
binary.

### [DEPLOY-JETBRAINS-PACKAGE] JetBrains package contract

The JetBrains plugin package must include `deployment-toolkit.json` at plugin
root and verify `deslop-lsp` before creating or starting the LSP descriptor.

If the package bundles native helpers under plugin-root `bin/<platform>/`, each
helper must be listed in the manifest. Startup failure must surface through a
JetBrains notification or Event Log entry with the same expected/found/path
details required by [DEPLOY-RESOLVER].

### [DEPLOY-CI-GATES] CI and release gates

CI and release jobs must fail fast on deployment drift.

Required gates:

- Validate `deployment-toolkit.json` with the Deployment Toolkit schema.
- Verify `deslop`, `deslop-lsp`, and `deslop-mcp` plain and JSON version
  output after release binaries are built.
- Verify LSP and MCP initialize metadata against the manifest version.
- Verify the produced VSIX package contents.
- Verify the produced JetBrains package contents before publishing.

When shared `deploy-toolkit` CLI commands are available, Deslop should call
them directly. Until then, product-local tests must prove the same behavior.

### [DEPLOY-PRIVATE-DTK-DOCS] Private Deployment Toolkit docs

Deployment Toolkit documentation and fixtures live in the private
`MelbourneDeveloper/deployment_toolkit` repository. Agents working these issues
must use authenticated `gh` access to read the docs and fixtures; they must not
rely on local absolute paths or assume the GitHub URLs are public.

Relevant private docs include:

- `docs/specs/binary-version-contract.md`
- `docs/specs/ide-extension-deployment.md`
- `docs/specs/acceptance-gates.md`
- `docs/agents/product-repo-adoption-guide.md`
- `docs/plans/product-migration-tickets.md`
- `schemas/version-manifest.schema.json`
- `fixtures/manifests/deslop.json`
