# Chapter 12 — Verify the cleanup and prevent new copies

> **Current status:** This chapter is an outline. I will include real Deslop reports and test results from before and after cleanup.

## What you will be able to do

I will show you how to prove that the repository's normal checks still pass, the intended duplicate group changed, and you lowered the configured duplication limit without hiding findings.

## Planned sections

### Run the repository's normal checks

Repeat static analysis and tests after the refactor. Include code-generation or expected-output checks when the changed code produces generated files. Compare the result with the commands recorded before editing instead of relying on memory.

### Run Deslop again

Rescan, then locate the original stable group ID and affected files. The group should disappear or shrink because the shared implementation now has one owner.

### Investigate an unexpected result

If the group remains, receives a different label, or disappears because configuration changed, you have not verified the cleanup. Read the new report before changing anything else.

### Record the exact results

Record the group before and after, the repository's test and static-analysis results, the affected files, and the number of lines changed. Removing lines is not enough; the code must still behave correctly.

### Lower the duplication limit

When the measured duplication percentage falls, lower the configured limit to the new accepted level. Do not change exclusions or report hiding just to produce a smaller number.

### Tell future agents which implementation to reuse

Record the shared implementation and its owning module. Confirm that the repository instructions still require `find-similar` before an agent proposes another copy.

## Workshop exercise

Complete the audit record:

```text
group id before:
visible label before:
occurrences before and after:
static analysis before and after:
tests before and after:
Deslop baseline before and after:
retained implementation and owner:
deliberate duplication retained:
duplication limit before and after:
```

## Check your understanding

1. Which results must be compared before and after the cleanup?
2. What should you investigate if the group disappears after a configuration change?
3. When should the configured duplication limit be lowered?

## Instruction for coding agents

```text
Finish the cleanup only after the repository's normal checks pass, Deslop shows the expected group change, any remaining duplication is explained, and the configured limit is lowered to the accepted result.
```

## How Kevin Moore's protocol is used

Kevin Moore's protocol requires actual test results before and after the change and a detailed final report. This book also compares the stable group ID, runs Deslop again, records any duplication that remains, and updates the agent instructions.

## Sources used for this chapter

- `kevmoo-duplication-audit`
- `deslop-for-ai`
- `deslop-configuration`
- `deslop-github-action`
