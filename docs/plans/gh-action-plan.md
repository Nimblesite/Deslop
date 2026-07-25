# Plan — Publish the Deslop GitHub Action to the Marketplace

Ship `Nimblesite/Deslop` as a listed GitHub Marketplace Action that installs the
released `deslop` CLI, runs it, writes the reports, and fails the job when
duplication breaches the threshold.

**Status: T1–T7 implemented. T8 (the Marketplace listing) is a manual human step
and is the only work outstanding. It is blocked on a tag: the listing resolves
`action.yml` from the tag, and every tag through v0.26.0 carries the rejected
`name: Deslop` — see T8.**

Spec group `[ACTION-*]`, added to [docs/specs/release.md](../specs/release.md).

Reference: [Publishing in the GitHub Marketplace](https://docs.github.com/en/actions/how-tos/create-and-publish-actions/publish-in-github-marketplace),
[Metadata syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/metadata-syntax).

---

## T1 — `action.yml` at the repository root

A composite action. `name: Deslop`, `author: Nimblesite`,
`branding: { icon: copy, color: purple }`.

Steps, in order: resolve → download → verify checksum → extract → run → read
outputs → upload artifact.

Inline `run:` blocks stay small; non-trivial logic goes in `scripts/` so the
500-line file and 20-line function rules hold.

### Inputs

| Input | Default | Purpose |
|---|---|---|
| `path` | `.` | Directory to analyse |
| `version` | derived from `github.action_ref` | CLI release to install |
| `fail-over` | unset | Maps to `--fail-over`. Unset honours `[threshold]` in `.deslop.toml` |
| `no-fail-over` | `false` | Maps to `--no-fail-over` |
| `min-nodes` | `30` | Maps to `--min-nodes` |
| `config` | unset | Explicit `.deslop.toml` path |
| `output` | `deslop-report` | Report path prefix |
| `nojson` / `notext` / `nohtml` | `false` | Format suppression passthroughs |
| `log-level` | `info` | Maps to `--log-level` |
| `upload-artifact` | `true` | Upload reports via `actions/upload-artifact` with `if: always()` |
| `artifact-name` | `deslop-report` | Artifact name |

No `embeddings` input. The provider is a loopback Ollama endpoint that does not
exist on a hosted runner, and `off` is already the CLI default.

### Outputs

| Output | Source |
|---|---|
| `duplication-percent` | `Report.metrics` in the JSON report |
| `cluster-count` | length of the cluster array |
| `exit-code` | raw CLI exit status |
| `report-json` / `report-text` / `report-html` | resolved artifact paths |

Outputs are written even when the gate trips.

### Gate semantics

Exit codes are owned by [pipeline.md §EXIT-CODES](../specs/pipeline.md). The
action surfaces them, it does not reinterpret them.

- `0` — step passes.
- `3` — threshold breached. Step fails with a message naming the measured
  percentage and the ceiling. Reports are still written and still uploaded.
- `1` / `2` — runtime and usage error. Step fails with a distinct message so a
  misconfigured input is never mistaken for a duplication breach.

The run step captures `$?`, records the outputs, then re-raises. Nothing wraps
it in anything that swallows the status.

### Version derivation

The default `version` is `github.action_ref` with the leading `v` stripped, so
`uses: Nimblesite/Deslop@v0.1.0` installs `deslop` 0.1.0. When `action_ref` is a
SHA or empty, `version` is required and its absence is a hard error naming the
fix — never a silent fall back to "latest".

### Binary resolution

1. Map `runner.os` × `runner.arch` to `linux-x64`, `linux-arm64`, `macos-x64`,
   `macos-arm64`, or `windows-x64`. Any other pair is a hard error naming the
   unsupported combination.
2. Download `deslop-${version}-${artifact}.{tar.gz,zip}` and its `.sha256` from
   the `Nimblesite/Deslop` release for that version.
3. Verify the checksum before extracting. A mismatch aborts.
4. Extract to a runner temp dir and prepend it to `$GITHUB_PATH`.

This is the `github-release` source already declared for the `deslop` component
in [shipwright.json](../../shipwright.json).

---

## T2 — `scripts/action-resolve-artifact.mjs`

Maps `(os, arch, version)` → `{ url, archive, checksumUrl }`, and derives the
version from `action_ref`. Node, matching the existing `scripts/*.mjs`
convention, so Windows reuses it instead of carrying a second PowerShell
implementation.

## T3 — `scripts/action-read-outputs.mjs`

Reads the JSON report and writes `duplication-percent`, `cluster-count` and the
report paths to `$GITHUB_OUTPUT`. Fails loudly when the report is absent and the
exit code implies it should exist.

## T4 — Tighten the release trigger

In [release.yml](../../.github/workflows/release.yml), change
`on: push: tags: 'v*'` to `'v[0-9]+.[0-9]+.[0-9]+*'` so a bare major tag can
never re-fire the release pipeline.

## T5 — `.github/workflows/action-selftest.yml`

Coarse, black-box, fail-fast, running the real binary. No fake CLI, no stubbed
download.

Matrix: `ubuntu-latest`, `ubuntu-24.04-arm`, `macos-latest`, `macos-13`,
`windows-latest`. Referenced as `./` with `version` pinned to the latest
published release, because on a PR the release for the current tag does not yet
exist.

Assertions, against the fixture repos in [examples/](../../examples/):

- duplicated fixture with `fail-over: 0` → step fails, exit code is `3`, and
  `deslop-report.html` still exists;
- same fixture with `fail-over: 100` → exit `0`;
- `duplication-percent` is a finite number in `[0, 100]` and equals the value in
  the JSON report;
- `cluster-count` is a positive integer for the duplicated fixture;
- `fail-over: 101` → exit `2`, and the message names the valid range rather than
  reporting a duplication breach;
- a corrupted download aborts before extraction.

Assertions state positive human-readable values, not non-empty guards.

## T6 — Version derivation test

A self-test job that pins the action by tag and asserts `deslop --version`
equals that tag with `v` stripped.

## T7 — Documentation

- [README.md:191](../../README.md#L191) — replace the `curl | tar` snippet with
  the action. Keep the raw-CLI form below it for self-hosted and non-GitHub CI.
- Document every input, every output, the exit-code table, and a full workflow
  example. The Marketplace listing requires this.
- [docs/specs/release.md](../specs/release.md) — add `[ACTION-METADATA]`,
  `[ACTION-RESOLVE]`, `[ACTION-VERIFY]`, `[ACTION-GATE]`, `[ACTION-VERSION]`,
  `[ACTION-PUBLISH]`, `[ACTION-TESTS]`. Code and tests carry the IDs so
  `grep [ACTION-` walks spec → code → test.
- Site docs — add the action to the CI guidance alongside the For-AI guide.

## T8 — Publish to the Marketplace

Manual, once, by a human with `Nimblesite` org permission. No agent attempts
this.

The name check is **resolved and no longer a search**: `Deslop` is unlistable.
The blocking rule is not listing-slug collision — `/marketplace/actions/deslop`
is free — but account collision. GitHub refuses a name matching any user or
organization unless that account publishes it, and the dormant unrelated org
`deslop` (id 6157270, created 2013-12-11, zero public repos) holds the login.
`action.yml` therefore declares `name: Deslop.live` — the product's own name and
shipping domain, and a dot is not a legal character in a GitHub login, so no
account can ever claim it. Asserted by `test-action-contract.mjs`. Do not
shorten it back.

Because metadata resolves from the tag, **v0.26.0 cannot be listed** — it carries
`name: Deslop`. A new tag is a hard prerequisite, not a nicety.

1. Confirm the publishing account has 2FA enabled. Publishing is blocked without
   it.
2. Cut a release **after** the `name: Deslop.live` commit. Confirm the tag really
   carries it:
   `gh api "repos/Nimblesite/Deslop/contents/action.yml?ref=<tag>" --jq .content | base64 -d | grep '^name:'`
3. Accept the GitHub Marketplace Developer Agreement. It must be accepted by the
   account that owns the repo — the `Nimblesite` org — not a personal account.
4. Open the new release for editing at
   `https://github.com/Nimblesite/Deslop/releases/edit/<tag>`, or click the
   **Draft a release** banner on the `action.yml` page.
5. Tick **Publish this Action to the GitHub Marketplace**.
6. Resolve every metadata validation error and warning the form reports.
7. Primary category **Code quality**, secondary **Continuous integration**.
8. Enter the release tag, add the title, publish with 2FA.

There is **no programmatic path** — verified against GitHub's OpenAPI (neither
`POST /repos/{owner}/{repo}/releases` nor `PATCH /releases/{id}` has a
marketplace field), the live release resource's 22 keys, the GraphQL schema
(which exposes no release-create/update mutation at all), and `gh release
create|edit`. Release automation must never assume a tag lists the action.

To unpublish: edit each published release, untick the Marketplace checkbox,
update the release.

---

## Sequencing

T1–T3 land together. T5–T6 gate them in CI. Then T4 and T7. T8 runs once, after
the first tag whose commit contains a root `action.yml`.

---

## Out of scope

- SARIF / code-scanning annotations. The CLI renders `.json`, `.txt` and `.html`
  only; a SARIF renderer is `deslop-core` work, not action glue.
- PR comments and duplication delta against the base branch.
- `$GITHUB_STEP_SUMMARY` offenders table.
- Embedding-backed analysis on hosted runners.
