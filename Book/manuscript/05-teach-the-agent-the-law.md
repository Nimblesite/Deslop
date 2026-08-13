# Chapter 5 — Teach the agent the law

> **Scaffold status:** Editorial structure established. The final instruction block will be verified against the pinned MCP schema and installation guidance.

## Reader outcome

Install a prevention contract in repository instructions so every compatible coding agent checks before authoring and uses cleanup tools only for existing code.

## Planned sections

### Name the covered authoring units

The rule explicitly covers functions, methods, classes, helpers, fixtures, test setup, parser branches, error types, route handlers, view models, and other multi-line units. Ambiguous “check for duplication” prose is too easy to satisfy after writing.

### Put `find-similar` before the edit

The instruction makes timing enforceable: proposed snippet first, tool call second, source edit only after the response is understood.

### Separate prevention from cleanup

`find-similar` answers whether proposed or selected code already exists. `top-offenders` and `cluster-by-id` drive an audit of existing groups. The repository rule names both paths.

### Define the failure ladder

The agent restores the live server when possible and uses the CLI immediate-detection loop when not. It never silently skips the evidence check.

### Ban cosmetic compliance

Widening the ceiling, hiding hand-written findings, or splitting a duplicate into trivially different shapes is not cleanup.

## Workshop checkpoint

Add the repository rule to both supported instruction files, begin a fresh agent session, propose a known duplicate, and capture whether the agent calls `find-similar` before editing.

## Agent handoff

The final chapter asset will be a paste-ready, version-verified rule derived from Deslop's repository recipe rather than a second independently maintained policy.

## Source keys

- `deslop-agent-recipe`
- `deslop-for-ai`
