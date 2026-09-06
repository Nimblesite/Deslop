# Proposal: Change-sensitive release (`SWR-REL-CHANGES`)

```
Status: Proposed (implemented in Deslop as the reference adopter)
Target: Shipwright release-pipeline spec (SWR-REL-*)
```

A `v*` release today rebuilds and republishes **every** component even when only
one changed. For a portfolio that ships a 5-platform binary matrix (the macOS legs
bill at 10× Linux), this is the dominant cost. This proposal makes a release
**sensitive to what changed since the prior release** without weakening any gate.

## Constraint: the single-version contract bounds what can ship alone

`SWR-REL-VERSION` stamps one version per tag, and every host `activation-verify`
is `onMismatch: error` (VSIX bundles lsp+mcp, JetBrains bundles lsp). So a shipped
component must bundle binaries **at the tag version** — you cannot ship a new VSIX
without binaries at that version. Therefore:

- **rust changed → full release.** Every downstream artifact bundles a binary.
- **vscode / jetbrains changed (rust unchanged) → the binary matrix still runs**
  (the artifact needs binaries at the new version), but the *other* registries
  (brew/scoop, the other extension) are skipped.
- **website changed → the only fully decoupled surface.** Skip the entire matrix.

A future extension (per-component effective version / "minimum-compatible binary
version") could let a vscode-only release reuse the prior release's signed,
notarized binaries and skip the matrix entirely — that is a contract change to
`activation-verify` and is intentionally **out of scope** here.

## Classifier contract

`scripts/release-changes.py` diffs `prior-v*-tag .. <tag>` with **native git**
(never `tj-actions/changed-files` — `SWR-SEC-ACTION-PINNING`) and emits:

| output | meaning |
|---|---|
| `rust` / `vscode` / `jetbrains` / `website` | that surface changed |
| `full` | `rust` OR release-infra changed OR an unrecognized path → build+publish all (fail safe) |
| `any_component` | `full \|\| vscode \|\| jetbrains` → the binary build matrix must run |

Fail-safe defaults: no prior tag, a missing manifest, or any unclassified path →
`full` (never a silent empty release). Surface gating then drives the workflow
DAG (`changes` job outputs → per-job `if:`), with `!cancelled()` + explicit
`needs.<job>.result` guards so a skipped upstream job never wrongly skips a
downstream publish.

## Path → surface mapping belongs in the manifest

Deslop's mapping is currently in the classifier. The generic Shipwright form adds
an optional per-component `sourcePaths` glob list to `shipwright.json`, so the
manifest is the single source of truth and the tool stays product-agnostic:

```jsonc
{ "id": "deslop-vscode", "kind": "extension-vscode",
  "sourcePaths": ["clients/vscode/**"] }
```

Until the schema carries `sourcePaths`, the classifier keeps a built-in map keyed
to the repo layout and treats everything else as neutral (no surface) or, if
genuinely unrecognized, as `full`.
