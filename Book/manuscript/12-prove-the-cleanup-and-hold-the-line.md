# Chapter 12 — Verify the cleanup and prevent new copies

A cleanup is finished when behavior is protected and the intended copies are gone.

## Repeat the baseline checks

Run the same static checks and tests as before the change. Include generated-output checks when relevant. Record pre-existing failures separately; do not claim a clean result while they remain.

Refresh Deslop and inspect the changed paths. Locate the original group when its ID still exists. If the ID changed, follow the source ranges and remaining occurrences. Confirm that callers use the retained implementation.

## Explain the measurement

Compare the generated reports using the same build and configuration. A lower percentage alone does not prove removal: added source can enlarge the denominator, and changed filters can hide findings.

If the group remains, read it. If it disappears, explain why. Do not alter exclusions, hidden paths, or included kinds to make the cleanup look successful.

## Leave a reusable owner

Record the shared implementation and its module. Keep the before-writing rule in the agent instructions. Lower the duplication ceiling only when the verified measurement supports it.

Use this handoff record alongside the generated reports:

```text
Selected group and source ranges:
Decision and retained owner:
Occurrences before and after:
Static checks and tests before and after:
Generated report paths:
Remaining duplication and reason:
Configuration or ceiling changes:
```

**Try it:** ask another maintainer to verify your result from the reports, code, and test output alone. They should not need your chat history.
