# Chapter 5 — Add the check to agent instructions

> **Scaffold status:** Editorial structure established. The final instruction block will be verified against the pinned MCP schema and installation guidance.

## What you will be able to do

Add a clear rule to the repository instructions so every compatible coding agent checks for matching code before writing. The rule also tells the agent which Deslop tools to use when cleaning up existing duplication.

## Planned sections

### Name the covered authoring units

The rule explicitly covers functions, methods, classes, helpers, fixtures, test setup, parser branches, error types, route handlers, view models, and other multi-line units. An instruction that only says “check for duplication” is unclear because an agent can satisfy it after writing the code. State that the check must happen first.

### State the order explicitly

The instruction gives the exact order: prepare the proposed snippet, call `find-similar`, read the response, and only then edit the source file.

### Separate prevention from cleanup

`find-similar` answers whether proposed or selected code already exists. `top-offenders` and `cluster-by-id` drive an audit of existing groups. The repository rule names both paths.

### Explain what to do when the connection fails

The agent first tries to restore the live editor server. If that is unavailable, it runs the CLI before and after the change and states that the proposed code could not be checked before writing. It never silently skips the check.

### Do not hide the result

Raising the duplication limit, hiding hand-written findings, or making trivial edits to avoid a match does not remove duplicate code.

## Workshop exercise

Add the repository rule to both supported instruction files, begin a fresh agent session, propose a known duplicate, and capture whether the agent calls `find-similar` before editing.

## Instruction for coding agents

The finished chapter will include a paste-ready rule checked against Deslop's official repository instructions and the exact release used for the book.

## Source keys

- `deslop-agent-recipe`
- `deslop-for-ai`
