# Chapter 7 — Keep the working set live

> **Scaffold status:** Editorial structure established. Live captures await the edition release pin.

## Reader outcome

Use focused editor and agent views to observe the same changing repository, then verify that an edit removed or reduced the intended duplicate group.

## Planned sections

### One changing corpus

The editor server watches source changes and refreshes the report. Human surfaces and MCP queries consume that live state rather than maintaining separate truths.

### Focus on the active file or range

File and range reports keep an investigation small enough for the current decision. Whole-repository output belongs in audit and triage, not every authoring step.

### Force a rescan deliberately

Large external changes may justify a rescan. Normal authoring relies on the reactive path.

### Confirm the intended effect

After reuse or consolidation, query the affected file and stable group ID. The group should be gone or smaller for the reason the change intended—not hidden, excluded, or cosmetically reshaped.

### Diagnose stale state

Root mismatch, inactive editor server, and binary drift each produce different symptoms. The chapter will show evidence from the pinned integration.

## Workshop checkpoint

Open the Workshop group in the human view, change one occurrence through the agent, observe the live refresh, and compare the focused file report before and after.

## Agent handoff

```text
After a change that should affect duplication, query the touched file and stable group. Do not wait for CI to reveal whether the working set moved as intended.
```

## Source keys

- `deslop-ai-integration`
- `deslop-for-ai`
