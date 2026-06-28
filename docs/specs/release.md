# Release Gates

Deslop uses `shipwright.json` as the source of truth for release
artifacts and IDE startup checks.

Source-controlled project versions intentionally stay at the placeholder
`0.0.0-dev`. Release and test workflows stamp the tag version into the working
tree with:

```bash
node scripts/stamp-release-version.mjs 1.2.3
```

The release workflow must never commit that stamped tree back to Deslop. The
tagged commit is the source identity; the stamped working tree is only the
build/package input.

Before publishing, run:

```bash
make deployment-verify
make vsix-package
make jetbrains-package
```

Test entry points remove cargo-installed `deslop`, `deslop-lsp`, and
`deslop-mcp` binaries before running. VSIX tests stage the release binaries
inside `clients/vscode/bin/<platform>/` and clear resolver override
environment variables so activation proves the extension bundle, not PATH.
VSIX release artifacts are platform-specific and must be packaged with
`vsce package --target`; release filenames use
`deslop-live-X.Y.Z-<target>.vsix`.

`make jetbrains-package` builds the JetBrains plugin zip, runs Gradle project
configuration and plugin structure verification, then runs
`scripts/verify-jetbrains-package.mjs` against the generated archive. The
archive verifier checks the root `shipwright.json`, the manifest-listed
host `deslop-lsp` binary, executable mode on Unix platforms, `--version`
identity, and undeclared native files under the shipped `bin/<platform>/`
directory. On Unix hosts the Makefile uses `gradle` from `PATH` when available,
then falls back to a cached Gradle 9.0.0 distribution under
`~/.gradle/wrapper/dists`; set `GRADLE=/path/to/gradle` to override it.

The shared Deployment Toolkit repository is private:
`Nimblesite/Shipwright` (formerly `MelbourneDeveloper/deployment_toolkit`).
Agents working from Deployment Toolkit migration issues must use authenticated
`gh` access before relying on its docs or fixtures:

```bash
gh auth status
gh repo view Nimblesite/Shipwright --json nameWithOwner,isPrivate,url,defaultBranchRef
```

When Deslop changes its deployment contract, update the private toolkit fixtures
for `fixtures/manifests/deslop.json` and the Rust version-output fixtures in the
same release workflow.

**CI and supply-chain gates.**

These gates run in `.github/workflows/` and `.deslop.toml` and fail the pipeline
on drift. Coverage floors are owned by `coverage-thresholds.json`
(`REPO-STANDARDS-SPEC [COVERAGE-THRESHOLDS-JSON]`).

- **[CI-DESLOP] Self-hosted duplication gate** — Deslop dog-foods its own
  detector: the `build` job runs the just-built release binary
  (`./target/release/deslop . --no-color`) against this repository. The binary
  reads the ratcheted `[threshold] max_duplication_percent` from `.deslop.toml`
  (the single source of truth — never hardcoded in CI) and exits 3
  ([pipeline.md §EXIT-CODES](pipeline.md)) the moment repo-wide weighted
  duplication climbs past it. The same threshold surfaces as a single LSP startup
  warning ([CI-DESLOP] is a CLI-only *gate*; the live LSP surface only *warns*).
- **[GITHUB-CODE-SCANNING] CodeQL** — `codeql.yml` runs CodeQL
  `security-extended` to feed GitHub code-scanning alerts (PRs to `main`, `v*`
  tags, weekly), across the `rust` / `javascript-typescript` / `actions` matrix
  with `build-mode: none`, gated on public-repo visibility. It is the sole owner
  of vulnerable-code detection; `java-kotlin` (JetBrains) is deferred until a
  `build-mode: manual` Gradle step exists.
- **[GITHUB-DEP-REVIEW] Dependency review** — the `security` job in `ci.yml` runs
  `actions/dependency-review-action` on every `pull_request`, blocking merges that
  add a dependency with a known vulnerability at `fail-on-severity: high`. It is
  the repo's only dependency vulnerability gate.
- **[GITHUB-DEPENDABOT] Dependabot** — `.github/dependabot.yml` raises weekly
  grouped updates for every ecosystem (github-actions, cargo, npm ×3, gradle).
  Routine version bumps target the long-lived `dependabot-upgrades` staging branch
  and are auto-squash-merged by `dependabot-automerge.yml`, so the expensive
  CI + CodeQL matrix runs once on the single `dependabot-upgrades → main`
  consolidation PR; security updates open against `main` directly.
- **[SWR-SEC-ACTION-PINNING] Action SHA pinning** — security-critical workflows
  pin third-party GitHub Actions to a full 40-character commit SHA with a trailing
  `# vX.Y.Z` comment, because a floating tag can be re-pointed at malicious code
  after review; `codeql.yml` is pinned today and the `github-actions` Dependabot
  group keeps the pins current while the standard is rolled out to the remaining
  workflows.

**Distribution channels.**

A `v*` tag fans out to every channel from one workflow
(`.github/workflows/release.yml`):

- **VS Code Marketplace** ([DEPLOY-VSCE-MARKETPLACE]) — `publish-marketplace` runs one `vsce publish` per
  platform-specific VSIX using Microsoft Entra OIDC, not a stored Marketplace
  PAT. The job runs in the protected `release` environment with
  `id-token: write`, signs in through the shared
  `Nimblesite-VSCode-Marketplace` app, mints a short-lived Azure DevOps access
  token, and passes it to pinned `@vscode/vsce@3.9.2` through `VSCE_PAT`.
  Environment secrets required: `AZURE_CLIENT_ID` and `AZURE_TENANT_ID`.
  Re-runs use `--skip-duplicate` so an already-published platform package does
  not block the remaining VSIXes.
  The Marketplace forbids a SemVer prerelease suffix in the version field, so
  `stamp-release-version.mjs` stamps the VSIX (`clients/vscode/package.json` +
  lockfile) with the core `MAJOR.MINOR.PATCH` while every other project keeps
  the full tag version; a hyphenated tag is published with `--pre-release`.
- **Open VSX** — `publish-openvsx` runs independently of Marketplace publish
  and uploads every platform-specific VSIX with pinned `ovsx@1.0.0`. Open VSX
  does not currently support OIDC trusted publishing, so the protected
  `release` environment must provide a separate `OPEN_VSX_PAT` token. Re-runs
  also use `--skip-duplicate`.
- **Homebrew tap** — `publish-homebrew` renders `Formula/deslop.rb` and pushes
  to `Nimblesite/homebrew-tap` (secret `HOMEBREW_TAP_TOKEN`).
- **Scoop bucket** — `publish-scoop` renders `bucket/deslop.json` and pushes to
  `Nimblesite/scoop-bucket` (secret `SCOOP_BUCKET_TOKEN`).
- **GitHub release** — `release` uploads every platform archive, the VSIXes, and
  `SHA256SUMS`.
- **Website** — stable tags call `deploy-pages` after the GitHub release is
  created. The Eleventy build loads `site/src/_data/releases.js`, fetches
  GitHub Releases with `GITHUB_TOKEN`, and renders `/releases/` plus
  `/zh/releases/` from the current release metadata on every website publish.

**Binary resolution — bundled, no fallback.**

The VSIX bundles `deslop`, `deslop-lsp`, and `deslop-mcp` per platform under
`bin/<platform>/`. The editor host resolves `deslop-lsp`/`deslop-mcp` from
exactly two sources, in order: the user override
(`deslop.lspPath` / `deslop.mcpPath`) and then the bundled binary. There is no
PATH, env-var, cargo-bin, package-manager, or GitHub-release fallback — the
extension runs the binary it shipped with, or the one the user explicitly
pointed at, or activation fails loudly. This is the `["user-setting", "bundled"]`
source list in `shipwright.json`, applied by the `candidates()` ordering in
`clients/vscode/src/binary.ts`. See ADR-0002 (no silent PATH fallback) and
[DEPLOY-RESOLVER].
