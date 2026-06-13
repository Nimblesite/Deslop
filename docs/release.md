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

## Distribution channels

A `v*` tag fans out to every channel from one workflow
(`.github/workflows/release.yml`):

- **VS Code Marketplace** — `publish-marketplace` runs one `vsce publish` per
  platform-specific VSIX. The PAT lives in the `VSCE_PAT` secret (Marketplace →
  Manage scope). The Marketplace forbids a SemVer prerelease suffix in the
  version field, so `stamp-release-version.mjs` stamps the VSIX
  (`clients/vscode/package.json` + lockfile) with the core `MAJOR.MINOR.PATCH`
  while every other project keeps the full tag version; a hyphenated tag is
  published with `--pre-release`.
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

## Binary resolution — bundled, no fallback

The VSIX bundles `deslop`, `deslop-lsp`, and `deslop-mcp` per platform under
`bin/<platform>/`. The editor host resolves `deslop-lsp`/`deslop-mcp` from
exactly two sources, in order: the user override
(`deslop.lspPath` / `deslop.mcpPath`) and then the bundled binary. There is no
PATH, env-var, cargo-bin, package-manager, or GitHub-release fallback — the
extension runs the binary it shipped with, or the one the user explicitly
pointed at, or activation fails loudly. This is the `["user-setting", "bundled"]`
source list in `shipwright.json` and `VSIX_HOST_SOURCES` in
`clients/vscode/src/deployment/sources.ts`. See ADR-0002 (no silent PATH
fallback) and [DEPLOY-RESOLVE-SOURCES].
