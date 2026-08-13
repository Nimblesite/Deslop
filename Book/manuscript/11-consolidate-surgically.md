# Chapter 11 — Consolidate surgically

> **Scaffold status:** Editorial structure established. Executable examples await the Workshop fixture and edition release pin.

## Reader outcome

Make the smallest consolidation that removes one justified duplicate group without weakening contracts, behavior, performance, or clarity.

## Planned sections

### Prove the baseline first

Run the repository's real dependency, static-analysis, and test commands before editing. A broken baseline changes what the cleanup can claim and may block safe remediation.

### Choose one owner

Place the shared implementation in the module that owns the behavior or in a neutral dependency that both callers can legitimately use. Avoid a generic “utils” destination chosen only because it accepts imports from everywhere.

### Parameterize meaningful variation

Nearly identical code often differs in policy. Turn a real variation into a named input or strategy only when callers can state it clearly. Avoid boolean flags that reconstruct two opaque copies inside one function.

### Preserve necessary specialization

Do not trade static contracts or hot-path behavior for a smaller line count. A rejected consolidation remains a successful audit outcome when the rationale is explicit.

### Keep the diff bounded

Touch the shared owner, call sites, and tests required by the verdict. Do not combine the cleanup with broad formatting or adjacent rearchitecture.

## Workshop checkpoint

Run the passing baseline, extract the chosen decoder into its correct owner, adapt both callers, and add assertions that preserve each meaningful policy difference.

## Agent handoff

```text
Consolidate one accepted group at a time. Preserve every contract and test-pinned difference; keep unrelated cleanup outside the diff.
```

## Practitioner source

Kevin Moore's protocol supplies the empirical baseline and surgical-modification gates. The final examples will use the Workshop repository's native toolchain rather than assuming Dart- or Flutter-specific commands.

## Source keys

- `kevmoo-duplication-audit`
- `deslop-for-ai`
