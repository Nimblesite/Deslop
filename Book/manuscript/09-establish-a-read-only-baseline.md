# Chapter 9 — Measure existing duplication without changing code

Before changing code, record what Deslop and the repository's tests currently report.

Kevin Moore's Deslop duplication audit protocol uses this order: run a read-only scan, present the largest findings, and then choose which groups to investigate. This prevents an initial report from turning into an unplanned repository-wide refactor.

![A proposed duplicate is intercepted before entering a layered repository while existing repeated manuscripts converge into one retained implementation.](assets/illustrations/prevent-then-clean.png)

*The agent check can stop a new copy. A read-only scan records existing copies before cleanup begins.*

## What you will be able to do

Record the current duplication results without modifying the target repository, then present enough information to choose one focused cleanup task.

## Verify the environment first

Record the Deslop version and installation, repository root, language tools, dependency command, static-analysis command, and test command. These details show exactly which repository and tools produced the results before and after cleanup.

The finished chapter will use Deslop's current installation instructions. Kevin Moore's protocol provides the order of work, while Deslop's own documentation provides the commands.

## Keep audit artifacts outside the target

A read-only discovery run sends report output to a dedicated scratch directory and disables incremental cache writes. The target checkout receives no `.deslop` cache and no source edit.

A generalized command shape is:

```sh
deslop /path/to/repository \
  --output /path/to/scratch/deslop-reports/repository/report \
  --no-incremental \
  --no-fail-over \
  --log-to-console \
  --log-level warn
```

The exact flags are verified against the edition binary before publication. The important properties are durable: external report destination, no repository cache, and no threshold failure interrupting discovery.

## Save both the JSON and human-readable reports

The JSON report contains the complete fields used by agents. The HTML or text report makes the largest duplicate groups easier for developers to read. Save the JSON report and use stable group IDs when summarising findings.

## Record the starting results

The discovery note should include:

```text
repository and revision:
Deslop version and artifact:
analysis configuration:
analysed source size:
duplication percentage:
duplicate groups:
duplicated files:
highest-impact group IDs and visible labels:
test and static-analysis commands:
report artifact paths and hashes:
```

The number of repeated lines can help choose which group to inspect first. It does not prove that the code should share an abstraction.

## Review the results before choosing a refactor

The report now shows where the largest duplicate groups are. It does not show which ones should be merged. Present the starting results and choose the specific groups to inspect before editing. If the task or repository instructions already name the groups, record that scope in the cleanup note.

## Workshop exercise

Create a temporary scratch directory, run the Workshop discovery audit without repository writes, and fill the baseline record. Confirm the target source and configuration remain unchanged.

## Instruction for coding agents

```text
For the first cleanup scan, write reports outside the target repository and do not change source files. Present the starting results and the selected duplicate groups before proposing shared code.
```

## What came from the practitioner protocol

This chapter uses Kevin Moore's read-only discovery and review steps. Deslop's current documentation remains the source for supported installation and command behavior. Moore's protocol provides the order: measure first, review the findings, then change code.

## Source keys

- `kevmoo-duplication-audit`
- `deslop-for-ai`
- `deslop-configuration`
