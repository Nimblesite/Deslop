# Chapter 9 — Establish a read-only baseline

Cleanup starts with evidence, not an extraction.

Kevin Moore's Deslop duplication audit protocol makes the separation explicit: discover in read-only mode, present the highest-impact findings, and only then choose whether remediation belongs in scope. That sequencing prevents a scanner result from quietly becoming an architectural rewrite.

![A proposed duplicate is intercepted before entering a layered repository while existing repeated manuscripts converge into one retained implementation.](assets/illustrations/prevent-then-clean.png)

*Prevention stops the next copy; a read-only audit gives existing copies a safe path toward consolidation.*

## Reader outcome

Produce a duplication baseline without modifying the target repository, then present enough evidence for a bounded cleanup decision.

## Verify the environment first

Record the Deslop artifact, repository root, language toolchain, dependency command, static-analysis command, and native test command. The goal is not ceremony. A cleanup claim needs to say which repository and tool state produced both the before and after evidence.

The long-form edition will use current Deslop installation guidance. The practitioner's protocol contributes the process gates, not a frozen distribution command.

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

## Use each renderer for its reader

The canonical JSON carries complete fields for agents and reproducible analysis. The human report makes the highest-impact duplicate groups easier to inspect. A good audit preserves the canonical artifact and links every summary claim back to a stable group ID.

## Present a dense baseline

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

Potential code savings can help prioritize, but line count never proves an abstraction is good.

## Stop before architecture

The audit now knows where the largest groups are. It does not yet know which are actionable. Present the baseline and agree on the bounded investigation scope before editing. In an autonomous repository workflow, that scope may already come from the task and repository instructions; the gate still exists as a written boundary.

## Workshop checkpoint

Create a temporary scratch directory, run the Workshop discovery audit without repository writes, and fill the baseline record. Confirm the target source and configuration remain unchanged.

## Agent handoff

```text
For cleanup discovery, write reports outside the target repository and make no source changes. Present the baseline and selected group scope before proposing an abstraction.
```

## What came from the practitioner protocol

This chapter adapts Kevin Moore's read-only discovery and alignment phases. Deslop's current documentation remains authoritative for supported installation and command behavior; the protocol supplies the empirical sequencing.

## Source keys

- `kevmoo-duplication-audit`
- `deslop-for-ai`
- `deslop-configuration`
