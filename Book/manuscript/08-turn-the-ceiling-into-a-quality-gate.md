# Chapter 8 — Turn the ceiling into a quality gate

> **Scaffold status:** Editorial structure established. Configuration and Action captures await the edition release pin.

## Reader outcome

Set a repository-wide duplication ceiling that local runs, agents, and CI share, then ratchet it downward after verified cleanup.

## Planned sections

### Baseline before policy

A ceiling begins with the current measured repository state. Setting an aspirational value below the baseline without a cleanup plan creates a permanently red gate.

### Configuration is a reviewed contract

The ceiling lives with the repository so humans, agents, and CI use the same number and failure meaning.

### Exclude and report hiding answer different questions

Excluded files never enter analysis. Report-hidden occurrences remain available as evidence but do not contribute to the headline. The chapter uses generated code to make the distinction concrete.

### A breach still produces evidence

The gate fails while preserving the canonical report. An agent can explain what crossed the ceiling instead of treating a non-zero exit as an opaque build error.

### Ratchet only after measured improvement

When a verified consolidation reduces the baseline, lower the ceiling in the same bounded change. Never widen it to make a failing run pass.

## Workshop checkpoint

Measure the Workshop baseline, select a ceiling it currently meets, introduce a known duplicate in a disposable checkpoint, observe the breach, remove the copy, and restore a passing gate.

## Agent handoff

```text
Treat the configured ceiling as a monotonic quality contract. Reduce it after real cleanup; never change evidence handling merely to restore a green build.
```

## Source keys

- `deslop-configuration`
- `deslop-github-action`
