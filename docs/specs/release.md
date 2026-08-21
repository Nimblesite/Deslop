# Release Gates

Deslop uses `shipwright.json` as the source of truth for release
artifacts and IDE startup checks.

Source-controlled project versions intentionally stay at the placeholder
`0.0.0-dev`. Release and test workflows stamp the tag version into the working
tree with:

```bash
node scripts/release/stamp-release-version.mjs 1.2.3
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
`scripts/deployment/verify-jetbrains-package.mjs` against the generated archive. The
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
  Provenance is contract-tested by `scripts/repository/dup-gate-source.test.mjs`, which
  `make lint` runs: `dup-gate` must depend on `build` and invoke
  `./target/release/deslop`, `make build` must compile the workspace rather than
  download a release archive, `ci.yml` must run `make build` before `make
  dup-gate`, and no workflow may reach for the Marketplace action — which
  installs a *published* release — to check this repository. The single
  exemption is `action-selftest.yml`, whose whole purpose is proving the
  published action works and which scans the `examples/` fixtures, never this
  tree. A gate running last month's binary would report last month's percentage.
- **[TEST-SELECTION] No test is selected by name** — the release gate
  (`make test`) runs `cargo llvm-cov --workspace --all-targets` with no test
  filter at all. `cargo test --skip` matches a *substring of the test name*, so
  the previous `--skip ollama_ --skip corpus_` dropped every hermetic test whose
  name merely mentioned a service: the corpus gate's own precision, scope and
  confidence self-tests in `deslop-test-support`, the mock-Ollama embedding
  suites, and the tests that assert graceful degradation when Ollama is
  unreachable — the exact tests that prove the gate works (gh #412). The Rust
  embedding suites need no daemon; they drive an in-process mock server or a
  deliberately dead endpoint, so they belong in the gate. `make test-ollama`
  covers only the VSIX suite, which does need a live daemon. The one suite that
  must stay out — the clone-and-scan corpus gate ([corpus.md §CORPUS-PIN](corpus.md))
  — is excluded structurally: `crates/deslop/Cargo.toml` declares its test target
  with `required-features = ["corpus-repos"]`, so cargo does not build it unless
  `make test-corpus` asks for the feature. Gating is by cost, never by name:
  `corpus_manifest_contract` reads the pinned manifests off disk and runs in the
  gate. Contract-tested by `scripts/repository/test-selection.test.mjs`, which
  `make lint` runs. Gating costs coverage of the corpus suite's *execution*,
  never of its *compilation*: `make lint` passes `--features deslop/corpus-repos`
  to clippy so the target is still built and linted on every run. Without that,
  a refactor elsewhere can leave the gate uncompilable and nothing notices until
  someone runs `make test-corpus` — commit `77bcbaed5` deleted two constants the
  suite still read and did exactly that.
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
  and are swept into it by `dependabot-automerge.yml` on the `pull_request`
  event, so the expensive CI + CodeQL matrix runs once on the single
  `dependabot-upgrades → main` consolidation PR. The sweep's base filter is
  `dependabot-upgrades` **only**, and `main` must never be added back: the job
  can only be narrowed to Dependabot by a job-level `if:`, and GitHub reports an
  `if:`-skipped job as a `skipped` check run, so subscribing to PRs against
  `main` hangs a dead check on every human PR — one that by construction can
  never run. Security updates are the cost of that filter: GitHub ignores
  `target-branch` for them and always opens them against `main`, where they are
  merged or retargeted by a human rather than swept, still gated by the
  `security` dependency-review job in `ci.yml`.
- **[SWR-SEC-ACTION-PINNING] Action SHA pinning** — security-critical workflows
  pin third-party GitHub Actions to a full 40-character commit SHA with a trailing
  `# vX.Y.Z` comment, because a floating tag can be re-pointed at malicious code
  after review; `codeql.yml` is pinned today and the `github-actions` Dependabot
  group keeps the pins current while the standard is rolled out to the remaining
  workflows.

**Distribution channels.**

A `v[0-9]+.[0-9]+.[0-9]+*` tag fans out to every channel from one workflow
(`.github/workflows/release.yml`). The pattern is MAJOR.MINOR.PATCH-only rather
than `v*` so a bare major alias can never re-fire the pipeline — see
[ACTION-VERSION].

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

## GitHub Marketplace action

**[ACTION-METADATA] One action, at the repository root.** The Marketplace lists
exactly one metadata file per repository and only at the root, so `action.yml`
sits beside `Cargo.toml`. It is a **composite** action: Deslop already publishes
five prebuilt platform archives, so a Docker action (Linux-only, per-run image
cost) and a JavaScript action (a bundled-node build for no gain) are both
strictly worse. Branding is a Feather icon plus one of the nine colours the
metadata schema permits — `icon: copy`, `color: purple`.

**The listing name is `Deslop.live`, and cannot be `Deslop`.** GitHub refuses a
Marketplace name that matches any user or organization login unless that account
is the publisher, and a dormant unrelated organization has held `deslop` (id
6157270) since 2013. `Nimblesite` is not a member, so the bare product name is
permanently unlistable from this repo — no tag, no agreement, and no category
choice changes that. `Deslop.live` is the product's own name and the shipping
domain, and a dot is not a legal character in a GitHub login, so no account can
ever claim it — the collision cannot recur. The listing name is
independent of the repository slug, so `uses: Nimblesite/Deslop@vX.Y.Z` is
unaffected and no consumer workflow changes. `test-action-contract.mjs` asserts
the whole `name:` line, because a substring check accepts the rejected name and
the rejection would otherwise surface only in the publish form, after the tag
was cut.

**[ACTION-VERSION] The action and the CLI it installs are the same version.**
The default `version` input is `github.action_ref` with the leading `v`
stripped, so `uses: Nimblesite/Deslop@vX.Y.Z` installs `deslop` X.Y.Z and the two
cannot drift. `stamp-release-version.mjs` deliberately does not commit its
output, so a stamped default in `action.yml` would never reach the tag a
consumer resolves — deriving from the ref sidesteps that entirely. A commit-SHA
or branch pin carries no version and is a hard error naming the fix, never a
silent fall back to "latest".

**[ACTION-VERSION-DOCS] No documented pin commits a version.** Because the stamp
never reaches the commit, whatever `uses:` version is committed is what the tag
carries — and the tag's README is the body of the Marketplace listing. v0.30.0
shipped a listing advertising `@v0.27.0`, so every visitor who copied the
quickstart installed a three-release-old CLI. A committed version cannot be kept
true, only audited, and the audit ran on push to `main` — which a tag push is
not — so the pins were free to rot for a full release cycle before anything
noticed.

So the pins name no version. `site/src/_data/releases.js` resolves the newest
published release at build time and the Action doc pages render it as
`{{ releases.pin }}`; the site deploys after the release exists, so the number
is current by construction and lives nowhere in git. GitHub serves `README.md`
raw with no build step, so it names the `X.Y.Z` placeholder its VSIX snippet
already uses, and points at `/releases/latest`. When the API is unreachable the
site falls back to the same placeholder — never a bare `@v`.

`test-action-contract.mjs` asserts both halves offline, on every PR: every pin is
one `resolveVersion` **refuses** to derive a version from — proven against the
resolver the action runs, not a second copy of its rule — and each surface uses
the form it can resolve. `test-release-version-stamping.mjs` asserts the stamper
leaves all three files byte-identical, so the rewrite path cannot return.

**No mutable major alias.** Marketplace consumers conventionally pin `@v1` and
the publisher re-points it each release. Deslop does not publish one: it would
contradict [SWR-SEC-ACTION-PINNING], and `v1` would match the release trigger
and publish a release named "1". The trigger pattern is narrowed to
MAJOR.MINOR.PATCH so this is structurally impossible rather than a convention.
Consumers pin exact versions; Dependabot bumps them.

**[ACTION-RESOLVE] Runner to release asset.** `runner.os` × `runner.arch` maps
to the `artifact_name` published by the release matrix — `linux-x64`,
`linux-arm64`, `macos-x64`, `macos-arm64`, `windows-x64`. These are release
*asset* names and are deliberately distinct from the Shipwright platform ids
(`darwin-arm64`, `win32-x64`) used by the manifest verifiers. An unsupported
pair is a hard error naming the pair; there is no silent fallback. This is the
`github-release` source already declared for the `deslop` component in
`shipwright.json`, so the action opens no new distribution channel.

**[ACTION-VERIFY] Checksums are verified before extraction.** The action
downloads the archive and its published `.sha256` sidecar and compares digests
before anything is unpacked. The digest is computed in Node so there is no
three-way branch between `sha256sum`, `shasum -a 256`, and Windows. Extraction
runs from inside the staging directory with a relative archive name — the step
shell on Windows is Git Bash, whose GNU tar parses the `D:` drive prefix of an
absolute path as a remote-host archive (`tar: Cannot connect to D:`). Windows
extracts with the runner's bsdtar (`%SystemRoot%\System32\tar.exe`), the only
tar in that shell that reads the `.zip`; every other platform reads the
`.tar.gz` with plain `tar -xf`.

**[ACTION-ENVPATH] Only runner-owned constants reach `$GITHUB_PATH` and
`$GITHUB_ENV`.** The runner replays both files as the *next* step's PATH and
environment, so a value built from an action input, a step output, or a
`${{ }}` expression hands whoever sets that input a say in where later steps
resolve their executables — CodeQL's `actions/envpath-injection`. The install
step therefore moves the extracted `deslop-<version>-<artifact>` directory to a
fixed `bin` name and exports `${RUNNER_TEMP}/deslop/bin`, a constant; the `mv`
doubles as the layout assertion, failing loudly if a release is ever repackaged
without that top-level directory. `scripts/actions/verify-env-path-writes.mjs` enforces
this with an error across `action.yml` and every workflow, in `make lint` so it
runs on every CI job rather than behind a path filter, and
`verify-env-path-writes.test.mjs` proves the gate rejects the tainted form.

**[ACTION-GATE] Exit codes are surfaced, never reinterpreted.** The run step
captures the CLI status with an `||` guard — GitHub injects `-e` into every
composite `shell: bash` step, so an unguarded non-zero status would abort the
step before the status reaches `GITHUB_OUTPUT` — the report step publishes the
measurements, the artifact is uploaded, and only then does the gate step
re-raise. Exit `3` fails with a message naming the measured percentage and the
ceiling; `1` and `2` fail with distinct messages so a misconfigured input is
never mistaken for a duplication breach. Ownership of the codes themselves
stays with [pipeline.md §EXIT-CODES](pipeline.md).

Every input reaches its script through `env`, never through `${{ }}`
interpolated into a `run` body, so a crafted input cannot inject shell.

**[ACTION-CACHE] The parse store survives between runs.**

> **Status: shipped.** Pinned by the action contract suite (`scripts/actions/test-action-contract.mjs`) and the two-runner `cache-seed`/`cache-warm` self-test.

The action restores `.deslop/cache` under the scan root before the run step and saves it afterwards with the SHA-pinned `actions/cache/restore` and `actions/cache/save` steps ([SWR-SEC-ACTION-PINNING]). The path derives from the `path` input, never the repository root — the store lives beside the scan root by contract ([pipeline.md §PIPELINE-INCREMENTAL]). The key is `deslop-<resolved version>-<runner os>-<run id>` with a `restore-keys` prefix that drops the run id: the store mutates every pass and an exact-key hit is never re-saved, so the per-run key plus prefix fallback is what lets each run restore the newest same-version store and save its own successor. Keying on the resolved CLI version keeps superseded partitions from riding between runs, and the post-pass retention sweep bounds every save at 2 GiB ([pipeline.md §PIPELINE-INCREMENTAL-RETENTION]), well under the 10 GiB repository ceiling Actions evicts against. Correctness never rests on the restore: every blob is digest-verified against its full address before a payload byte is decoded, and anything stale, foreign, or tampered is refused into a plain miss and rebuilt from source ([pipeline.md §PIPELINE-INCREMENTAL-INTEGRITY]) — the worst a bad cache entry can cost is a re-parse. A `cache: "false"` input skips both steps and changes nothing else.

**[ACTION-PUBLISH] Listing the action is a manual, human step.** Draft a release
from the `action.yml` page, tick *Publish this Action to the GitHub
Marketplace*, choose the categories, and publish with 2FA. It requires the
Marketplace Developer Agreement accepted on the `Nimblesite` org and a unique
`name`. The listing resolves metadata from the tag, not from `main`, so the
first listed version must be a tag whose commit already contains `action.yml`.

**[ACTION-TESTS] Two layers.** `scripts/actions/test-action-contract.mjs` runs in
`make deployment-verify` and proves what a runner cannot cheaply re-prove per
PR: the asset mapping against the real `release.yml` matrix, version derivation,
checksum rejection, output extraction, and the static shape of `action.yml`
(including that the nested `upload-artifact` stays pinned to a 40-character
SHA). `.github/workflows/action-selftest.yml` then exercises the action exactly
as a consumer would, on the four runner pools GitHub still operates (macOS
Intel has no hosted leg — GitHub retired the `macos-13` pool, so the job
queues for hours instead of running; the `macos-x64` asset is still
cross-compiled and published by the release matrix, and the contract layer
proves its runner mapping), against the fixtures in `examples/` —
asserting a clean run publishes a finite percentage and the installed
`deslop --version` matches the requested version, a breach fails the step but
still leaves a browsable report, and an out-of-range threshold fails without
rendering one.

The matrix installs an explicit `version:`, because a pull request cannot install
the release for its own tag — that release does not exist yet. So the derivation
path a Marketplace consumer actually takes is proven by a separate
`derived-version` job that pins `Nimblesite/Deslop@vX.Y.Z` with no `version:` at
all and asserts the CLI that lands is the tag. It pins by tag rather than by SHA
on purpose: a SHA carries no version and is a documented hard error, so a SHA pin
would prove the opposite. Its tag is not required to be the newest release — any
tag carrying `action.yml` proves the derivation, and coupling it to the release
cadence would redden `main` for a pin bump.

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
