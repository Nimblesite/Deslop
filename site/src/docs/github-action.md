---
layout: layouts/docs.njk
title: GitHub Action — gate CI on duplicated code
description: Full configuration for the Deslop.live GitHub Action — every input and output, the exit-code contract, threshold precedence, measure-without-gating, supported runners, artifact handling, and how the pinned tag selects the CLI version.
keywords: deslop, github action, ci gate, duplicate code, fail-over, duplication threshold, code quality, continuous integration
eleventyNavigation:
  key: GitHub Action
  order: 7
icon: rule
---

# GitHub Action

[**Deslop.live** on the GitHub Marketplace](https://github.com/marketplace/actions/deslop-live) installs the released `deslop` CLI on the runner, analyses the workspace, renders the reports, and fails the job when duplication breaches your ceiling.

It is a composite action. There is no container to pull and no post-install download — it fetches the prebuilt archive for the runner, verifies its published SHA-256 before extracting, and puts `deslop` on `PATH`.

## Quickstart

```yaml
name: deslop
on: [push, pull_request]

jobs:
  duplication-gate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: Nimblesite/Deslop@v{{ releases.pin }}
        with:
          fail-over: "5.0"   # or omit to use [threshold] in .deslop.toml
```

The action needs no token and no permissions beyond the default `contents: read`.

## Pinning and the CLI version

**The tag you pin is the CLI version you get.** The `version` input defaults to `github.action_ref` with the leading `v` stripped, so `uses: Nimblesite/Deslop@v{{ releases.pin }}` installs that same `deslop` release. The two cannot drift.

Pin an exact version rather than a mutable ref — Dependabot bumps it for you. There is deliberately **no `@v1` alias**: a mutable major tag is exactly the supply-chain shape that pinning exists to avoid. Every snippet on this page names {% if releases.latest %}**{{ releases.pin }}**, the newest release{% else %}the newest release{% endif %} — the number is resolved when the page is built, never committed, so it cannot fall behind the CLI it installs.

If you pin to a commit SHA or a branch, the ref carries no version, so `version` becomes **required**. Its absence is a hard error naming the fix, never a silent fall back to "latest":

```yaml
      - uses: Nimblesite/Deslop@8f4c1e2a9b7d3f6a5c8e1b4d7a0f3c6e9b2d5a8f
        with:
          version: "{{ releases.pin }}"
```

## Inputs

| Input | Default | Purpose |
| --- | --- | --- |
| `path` | `.` | Directory to analyse |
| `version` | the pinned tag | CLI release to install. Required when you pin this action to a commit SHA |
| `fail-over` | *(unset)* | Percentage above which the job fails. Unset honours `.deslop.toml` |
| `no-fail-over` | `false` | Clear any configured threshold for this run |
| `min-nodes` | `30` | Minimum AST subtree node count for a clone candidate |
| `config` | *(unset)* | Explicit `.deslop.toml` path |
| `diff` | *(unset)* | Unified diff whose added lines scope the report — a patch file path, or `-` for stdin. The whole tree is still analysed; the diff scopes what is reported, never what is scanned |
| `only-changed` | `false` | Report only the clusters the diff touches and gate on the diff-scoped percentage — duplicated added lines over added lines — so legacy debt cannot fail a pre-merge check. Requires `diff` |
| `cache` | `true` | Carry the parse store between runs through the Actions cache, so a warm run re-parses only what changed |
| `output` | `deslop-report` | Report path prefix; `.json`, `.txt`, `.html` are appended |
| `nojson` / `notext` / `nohtml` | `false` | Suppress an output format |
| `log-level` | `info` | `error`, `warn`, `info`, `debug` or `trace` |
| `upload-artifact` | `true` | Upload the rendered reports |
| `artifact-name` | `deslop-report` | Name of the uploaded artifact |

## Outputs

| Output | Meaning |
| --- | --- |
| `duplication-percent` | Duplicated lines as a percentage of analysed lines |
| `cluster-count` | Clusters contributing to the duplicated line count |
| `threshold-percent` | The ceiling this run was measured against |
| `exit-code` | `0` clean, `1` runtime error, `2` usage error, `3` breached |
| `report-json` / `report-text` / `report-html` | Paths to the rendered reports |

Outputs are published **even when the gate trips**, so a later step can comment the number or ratchet a budget. Setting `nojson: true` leaves them empty — they are read from the JSON report.

## Threshold precedence

`fail-over` takes precedence over `[threshold] max_duplication_percent` in [`.deslop.toml`](/docs/configuration/). Leave it unset to honour the config file, which is the better default for a repo that already commits its own ceiling.

- `fail-over: "0"` fails on **any** duplication.
- `no-fail-over: "true"` clears the configured ceiling for this run, so the job measures without ever failing. It is mutually exclusive with `fail-over`.
- The percentage must be a finite number in `[0.0, 100.0]`. Anything else exits `2`.

## Measure without gating

Report the number on every PR and let a human decide, rather than blocking the merge:

```yaml
      - uses: Nimblesite/Deslop@v{{ releases.pin }}
        id: deslop
        with:
          no-fail-over: "true"   # measure without gating
      - run: echo "{% raw %}${{ steps.deslop.outputs.duplication-percent }}{% endraw %}% duplicated"
```

This is the recommended way to introduce Deslop to an existing codebase: run it ungated for a few weeks, watch where the number settles, then set `fail-over` just below it and ratchet down.

## Exit codes

The action **surfaces** the CLI's status; it never reinterprets it.

| Code | Meaning |
| --- | --- |
| `0` | Analysis succeeded and duplication was within the threshold (or no threshold was set). |
| `1` | Runtime error — bad scan path, parse/I-O failure, or an unreachable `required` embedding provider. Never a panic. |
| `2` | Usage error — unknown flag, or an out-of-range / non-finite threshold value. |
| `3` | **Duplication threshold breached.** The full report is still written so CI can surface the offenders. |

A breach fails the step with a message naming the measured percentage and the ceiling. `1` and `2` fail with distinct messages, so a misconfigured input is never mistaken for a duplication breach.

Crucially, the reports are rendered and the artifact is uploaded **before** the gate re-raises the status — a failing build still gives you the offenders.

## Reports and artifacts

By default the action writes `deslop-report.json`, `deslop-report.txt` and `deslop-report.html` and uploads all three as a workflow artifact named `deslop-report`.

```yaml
      - uses: Nimblesite/Deslop@v{{ releases.pin }}
        with:
          output: reports/duplication
          artifact-name: duplication-reports
          nohtml: "true"        # JSON + text only
```

The HTML report is the one to open as a human; the JSON report is the one to parse. See [Report output](/docs/configuration/#report-output) for the shape of each.

## Supported runners

| `runner.os` | `runner.arch` | Release asset |
| --- | --- | --- |
| `Linux` | `X64` | `linux-x64` |
| `Linux` | `ARM64` | `linux-arm64` |
| `macOS` | `X64` | `macos-x64` |
| `macOS` | `ARM64` | `macos-arm64` |
| `Windows` | `X64` | `windows-x64` |

Any other pair is a hard error naming the unsupported combination. There is no Windows ARM64 build.

## Supply-chain notes

- The archive and its published `.sha256` sidecar are both downloaded, and the digest is **verified before anything is extracted**. A mismatch aborts the job.
- Every input reaches its script through `env`, never interpolated into a shell body, so a crafted input cannot inject shell.
- Only runner-owned constants are written to `$GITHUB_PATH` and `$GITHUB_ENV`, so no caller-supplied value can influence where later steps resolve executables from.

## Not using GitHub Actions?

Self-hosted runners, non-GitHub CI, or an image that already carries the CLI — drive the binary directly:

```bash
brew install nimblesite/tap/deslop                                       # macOS / Linux
scoop bucket add nimblesite https://github.com/Nimblesite/scoop-bucket   # Windows
scoop install deslop

deslop . --fail-over 5.0
```

Exit code `3` fails the step like any other non-zero status. Archives for every platform are on the [Releases page](https://github.com/Nimblesite/Deslop/releases).

Agents driving CI should read the [AI Agents](/docs/ai-integration/) guide for the same gate plus how to parse the JSON report.
