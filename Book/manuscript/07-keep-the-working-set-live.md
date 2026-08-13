# Chapter 7 — Check the result after every code change

> **Scaffold status:** Editorial structure established. Live captures await the edition release pin.

## What you will be able to do

Use the editor and focused agent reports to check the same repository, then confirm that an edit removed or reduced the expected duplicate group.

## Planned sections

### The editor and agent read the same report

The editor server watches source changes and refreshes the report. The editor views and MCP tools read that report instead of running separate analyses.

### Focus on the active file or range

File and range reports keep an investigation small enough for the current decision. Whole-repository output belongs in audit and triage, not every authoring step.

### Use `rescan` only when needed

Run `rescan` after a large change made outside the editor or when the report has clearly missed a file change. Normal edits update through the running editor server.

### Check that the edit changed the expected group

After reusing or merging code, query the affected file and stable group ID. The group should be gone or contain fewer occurrences because code was actually removed, not because a setting hid it or a trivial rewrite avoided detection.

### Diagnose an outdated report

A wrong repository root, a stopped editor server, and mismatched Deslop versions produce different errors. The chapter will show each error using the exact Deslop release used for the book.

## Workshop exercise

Open the Workshop group in the human view, change one occurrence through the agent, observe the live refresh, and compare the focused file report before and after.

## Check your understanding

1. Why should the editor and agent normally agree on a stable group ID?
2. When is a forced `rescan` appropriate?
3. How can you tell whether a group disappeared because code was removed rather than hidden?

## Instruction for coding agents

```text
After a change that should affect duplication, query the changed file and stable group. Check the result before handing the change to CI.
```

## Source keys

- `deslop-ai-integration`
- `deslop-for-ai`
