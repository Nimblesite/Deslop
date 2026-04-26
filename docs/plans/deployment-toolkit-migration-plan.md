# Deployment Toolkit Migration Plan

## Scope

Implement the Deslop Deployment Toolkit migration tracked by:

- [#37](https://github.com/Nimblesite/Deslop/issues/37) -
  `DTK-MIG-DESLOP-VERSION-CONTRACT`
- [#38](https://github.com/Nimblesite/Deslop/issues/38) -
  `DTK-MIG-DESLOP-VSCODE-RESOLVER`
- [#39](https://github.com/Nimblesite/Deslop/issues/39) -
  `DTK-MIG-DESLOP-VSIX-PACKAGE`
- [#40](https://github.com/Nimblesite/Deslop/issues/40) -
  `DTK-MIG-DESLOP-JETBRAINS-RESOLVER`
- [#41](https://github.com/Nimblesite/Deslop/issues/41) -
  `DTK-MIG-DESLOP-CI-GATES`

This is deployment work. It does not change duplicate-code detection, cluster
ranking, or UI grouping.

## Source Material

Agents must read the issue bodies and the private Deployment Toolkit docs before
implementation. The private docs are not public browser references; use
authenticated `gh` access:

```bash
gh auth status
gh repo view MelbourneDeveloper/deployment_toolkit --json nameWithOwner,isPrivate,url,defaultBranchRef
```

Required private references:

- `docs/specs/binary-version-contract.md`
- `docs/specs/ide-extension-deployment.md`
- `docs/specs/acceptance-gates.md`
- `docs/agents/product-repo-adoption-guide.md`
- `docs/plans/product-migration-tickets.md`
- `schemas/version-manifest.schema.json`
- `fixtures/manifests/deslop.json`

## Implementation Order

### Phase A - Contract Baseline

Confirm `deployment-toolkit.json` is the Deslop source of truth and matches the
private `fixtures/manifests/deslop.json` shape. Any deliberate divergence must
be reflected in the private fixture as part of the same migration.

The local spec contract is [DEPLOY-MANIFEST](../specs/deployment.md).

### Phase B - Binary Version Contract (#37)

Add contract tests first for:

- `deslop --version`
- `deslop-lsp --version`
- `deslop-mcp --version`
- `deslop --version --json`
- `deslop-lsp --version --json`
- `deslop-mcp --version --json`

Plain output must be exactly `<component-id> 0.1.0`. JSON output must validate
against the Deployment Toolkit version schema and include the Deslop product id.

Implementation must handle version flags before CLI workspace parsing, tracing,
LSP/MCP startup, file discovery, cache writes, or network access.

Add protocol metadata tests proving:

- LSP initialize reports `serverInfo.name = "deslop-lsp"` and version `0.1.0`.
- MCP initialize reports `serverInfo.name = "deslop-mcp"` and version `0.1.0`.

### Phase C - VS Code Resolver (#38)

Replace bespoke activation resolution with manifest-backed verification.

Startup must:

1. Load packaged `deployment-toolkit.json`.
2. Read `hosts.vscode.activationVerifies`.
3. Resolve `deslop-lsp` and `deslop-mcp` for the current platform.
4. Run `--version` on the selected candidate without shell interpolation.
5. Compare component id and version against the manifest.
6. Start LSP/MCP integrations only after required checks pass.
7. Surface actionable VS Code errors for missing or mismatched binaries.

Deslop-specific mismatch rules from #38:

- `deslop.lspPath` mismatch blocks activation.
- `deslop.mcpPath` mismatch blocks activation.
- `DESLOP_LSP_PATH` and `DESLOP_MCP_PATH` mismatches block activation.
- `DESLOP_BINARY_DIR` required-component mismatches block activation.
- A stale PATH binary is skipped when a matching bundled binary exists.
- Bundled mismatch blocks activation as a release/package failure.

### Phase D - VSIX Package Verification (#39)

Update packaging so the produced VSIX includes:

- `extension/deployment-toolkit.json`
- manifest-listed `deslop`
- manifest-listed `deslop-lsp`
- manifest-listed `deslop-mcp`

Add tests against the generated `.vsix` archive. The test must fail for missing
manifest, missing required binary, extra undeclared executable under
`extension/bin/<platform>/`, non-executable binary where applicable, and
wrong-version binary.

### Phase E - JetBrains Resolver And Package (#40)

Update the JetBrains plugin to load the manifest from plugin root before it
creates or starts the LSP descriptor.

The plugin must read `hosts.jetbrains.activationVerifies`, verify
`deslop-lsp`, and report failures through a JetBrains notification or Event Log
entry with expected version, found version, component id, and path/source.

Package checks must prove the plugin root contains `deployment-toolkit.json` and
that any bundled helper under `bin/<platform>/` is manifest-listed.

### Phase F - CI And Release Gates (#41)

Wire the release path so deployment drift fails before publish:

- Manifest validation.
- Binary plain and JSON version verification for `deslop`, `deslop-lsp`, and
  `deslop-mcp`.
- LSP and MCP initialize metadata verification.
- VSIX package contents verification.
- JetBrains package contents verification (implemented but temporarily
  deferred to GitHub #55 pending the Gradle validation work in GitHub #56).

Use shared `deploy-toolkit` commands when available. Until they are published,
add product-local tests that prove the same behavior.

Release/publish docs must state that Deployment Toolkit is private and agents
need authenticated `gh` access for referenced docs and fixture updates.

## Critical Invariants — Never Break These

The following rules are non-negotiable. Every future Deployment Toolkit migration,
test harness change, or CI update must preserve them. Violations here have
historically been subtle (tests passing green while testing the wrong binary) and
are therefore called out explicitly.

### Rule 1: Binaries must be bundled inside the extension package

Every extension that requires `deslop-lsp` or `deslop-mcp` must bundle those
binaries inside the extension archive for every supported platform. A platform that
lacks a bundled binary fails with a hard error at activation — it does not silently
fall back to PATH. This is enforced by:

- `_vsix-stage-bundled-binaries` (Makefile) — copies `target/release/deslop-lsp`
  and `target/release/deslop-mcp` into `clients/vscode/bin/<platform>/` before
  packaging and before running any VSIX tests.
- `scripts/verify-vsix-package.mjs` — extracts the `.vsix` archive, verifies each
  `activationVerifies` binary is present and executable, and runs `--version` on
  the extracted binary to confirm its identity.
- `BundledBinaryMissingError` in `binary.ts` — the resolver throws this error with
  `hardFailure: true`, so there is no path-fallback when the bundled binary is
  absent.

### Rule 2: Extension tests must use the bundled binary, not target/release or PATH

The resolver priority chain is:
`user-setting → env-path → env-dir (DESLOP_BINARY_DIR) → bundled → path`

`env-dir` is position 3 and beats `bundled` at position 4. Setting
`DESLOP_BINARY_DIR` in the test environment to `target/release` (or any directory
other than the staged extension bin) bypasses the bundled path entirely and makes
tests meaningless for proving deployment correctness.

**The law:**

- `.vscode-test.mjs` must clear `DESLOP_BINARY_DIR: ""`, `DESLOP_LSP_PATH: ""`,
  and `DESLOP_MCP_PATH: ""` so the resolver reaches the `bundled` candidate.
- `_vsix-stage-bundled-binaries` must run before every `vsix-test`, `vsix-coverage`,
  and `vsix-package` invocation so the staged binaries exist.
- At least one E2E test must assert `resolvedLsp.source === "bundled"` and
  `resolvedMcp.source === "bundled"` via the `ExtensionApi` returned by
  `activate()`. This assertion is in `bubble.e2e.test.ts`.

### Rule 3: make test must eliminate PATH-installed binaries before running

`delete-path-binaries` (called by `make test`, `make vsix-test`, `make vsix-coverage`,
and `make vsix-package`) removes cargo-installed copies of `deslop`, `deslop-lsp`,
and `deslop-mcp` from `~/.cargo/bin`. It then checks `command -v <binary>` and
fails the build if any binary is still reachable from PATH, EXCEPT binaries found
inside a VS Code or Cursor extension directory (`*/.vscode/extensions/*` etc.) —
those are extension-bundled copies that the resolver's `bundled` candidate already
beats.

If this target is ever changed to a no-op, softer check, or exception-added, the
author must update this section with a dated rationale.

---

## Open Decisions

- Whether `--version --format json` should be accepted as an alias for the
  issue-pinned `--version --json` form.
- Whether JetBrains native binaries are bundled for all platforms in one plugin
  zip, or staged through a marketplace-compatible helper flow later.

## TODO

- [x] Fetch and re-read all five GitHub issues plus private Deployment Toolkit
      docs before implementation.
- [x] Compare local `deployment-toolkit.json` with private
      `fixtures/manifests/deslop.json` and reconcile drift.
- [x] Add failing tests for `deslop`, `deslop-lsp`, and `deslop-mcp` plain
      version output.
- [x] Add failing tests for `--version --json` schema output for all three
      binaries.
- [x] Implement version flags before any runtime startup or workspace parsing.
- [x] Add LSP initialize metadata tests for `deslop-lsp`.
- [x] Add MCP initialize metadata tests for `deslop-mcp`.
- [x] Replace VS Code binary resolution with manifest-backed startup
      verification for `deslop-lsp` and `deslop-mcp`.
- [x] Add VS Code resolver tests for user setting, env path, env directory,
      PATH fallback, bundled success, missing binary, component mismatch, and
      version mismatch.
- [x] Ensure VSIX packaging includes `deployment-toolkit.json` at extension
      root.
- [x] Add generated `.vsix` archive verification for manifest-listed binaries
      and undeclared executables.
- [x] Force VSIX tests to stage and resolve bundled extension binaries instead
      of `target/release` or PATH-installed binaries.
- [x] Make test entry points scrub installed Deslop binaries from PATH before
      tests run.
- [x] Update JetBrains packaging to include `deployment-toolkit.json` at plugin
      root.
- [x] Add JetBrains resolver checks before LSP descriptor startup.
- [x] Add JetBrains tests for env directory, PATH, bundled, missing, mismatch,
      and notification/Event Log behavior.
- [x] Add CI gates for manifest validation and built binary version checks.
- [x] Add CI gates for VSIX package verification.
- [ ] Re-enable JetBrains package archive verification in `make
      jetbrains-package` (GitHub #55).
- [ ] Restore a reliable local JetBrains Gradle validation path (GitHub #56).
- [x] Inspect private Deployment Toolkit fixtures and Deslop migration docs;
      no product manifest drift required.
- [x] Update release/publish docs with private Deployment Toolkit access
      requirements.
