# Chapter 8 — Set a duplication limit in CI

> **Scaffold status:** Editorial structure established. Configuration and Action captures await the edition release pin.

## What you will be able to do

Set a maximum duplication percentage that local runs, agents, and CI share. Lower that limit after verified cleanup.

## Planned sections

### Measure the current percentage first

Run Deslop before setting the limit. If you set the limit below the repository's current result without first cleaning up, every CI run will fail.

### Commit the limit with the repository

Store the limit in the repository configuration so developers, agents, and CI use the same number and receive the same failure.

### Exclude and report hiding answer different questions

Excluded files never enter analysis. Report-hidden occurrences remain available as evidence but do not contribute to the headline. The chapter uses generated code to make the distinction concrete.

### A failed limit still writes the report

When the duplication percentage is too high, Deslop returns a failing exit code and still writes the JSON report. The agent can read the report and explain which groups contributed to the failure.

### Lower the limit only after cleanup

When a verified cleanup reduces the duplication percentage, lower the configured limit in the same change. Do not raise the limit just to make a failing run pass.

## Workshop exercise

Measure the Workshop repository, select a limit it currently meets, introduce a known duplicate in a disposable copy, observe the CI failure, remove the duplicate, and confirm that the check passes again.

## Instruction for coding agents

```text
Use the configured duplication limit in local runs and CI. Lower it after real cleanup. Do not hide findings or raise the limit just to make the build pass.
```

## Source keys

- `deslop-configuration`
- `deslop-github-action`
