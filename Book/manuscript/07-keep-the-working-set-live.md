# Chapter 7 — Check the result after every code change

After reusing code, check that the expected copies changed. Wait for live analysis, then call `duplicates` with the affected `path`. Add `start_byte` and `end_byte` only when they describe the refreshed source.

## Inspect the result

Read the remaining occurrences. Confirm that the implementation now has the intended owner and that callers use it. Search by affected path as well as the old group ID: changing code can change the ID.

A disappearing finding needs an explanation. Removed code, a renamed group, a changed filter, and a hidden occurrence are different outcomes.

## Recover an outdated view

Check the repository root, bundle, configuration, and source state. If the watcher appears behind, call `rescan` with the changed `paths`. Its change summary describes that forced refresh, not every edit since your last query.

Disk reports are snapshots. The live seed is not rewritten after every incremental edit, so it cannot establish that a finding is absent now. See the [recovery guide](https://deslop.live/docs/for-ai/).

**Try it:** make one reversible change in a disposable example, observe the editor refresh, and query the same file through MCP. Compare the occurrences, not just the headline count.
