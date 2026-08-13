# Chapter 2 — Put Deslop in the loop

> **Scaffold status:** Editorial structure established. Installation commands and captures await the edition release pin.

## Reader outcome

Explain how the agent, editor, CLI, and CI consume one Deslop analysis engine and choose the right surface for prevention, live inspection, or cold-start auditing.

## Opening repository state

The Workshop repository is open in an editor. A coding agent has access to repository files but no Deslop tool. The team already runs a duplication scan in CI, so every duplicate is discovered after authoring, tests, and review have spent time on it.

The missing piece is not another scanner. It is a live path into the authoring decision.

## Planned sections

### One engine, several jobs

The MCP server answers focused agent queries. The LSP server carries live analysis into the editor. The CLI handles one-shot audits and CI. They consume the same underlying report model rather than inventing independent meanings.

### The agent surface

`find-similar` belongs in the authoring loop. `top-offenders`, `cluster-by-id`, file reports, and range reports belong in investigation and cleanup.

### The human surface

The live bubble, tree, hover, and report let a maintainer see the same duplicate groups in the vocabulary defined by the glossary.

### The cold-start surface

The CLI is the fallback for CI and read-only audits. It can catch a newly written duplicate quickly, but it cannot query a snippet that does not yet exist.

### Root and version must agree

The edition will show how to confirm that the editor server and MCP process point at the same workspace and compatible installed artifact.

## Workshop checkpoint

Record the repository root, the editor integration, the MCP binary provenance, and the CLI version. Confirm that a focused query and a human view describe the same duplicate group.

## Agent handoff

```text
Use MCP before authoring, focused live reports while editing, and the CLI for cold-start audits or CI. Do not treat those jobs as interchangeable.
```

## Source keys

- `deslop-ai-integration`
- `deslop-for-ai`
