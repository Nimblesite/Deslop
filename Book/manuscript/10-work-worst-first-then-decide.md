# Chapter 10 — Inspect the largest duplicate groups first

Start with the duplicate groups at the top of the report, then inspect the code before deciding whether to merge it.

## What you will be able to do

Use `top-offenders` to select a large duplicate group, inspect every occurrence with `cluster-by-id`, and record whether to merge the group, leave it separate, or wait for more information.

## Start at the top of the report

Deslop puts the groups with the largest reported impact first. Request a short `top-offenders` list, choose one stable group ID, then call `cluster-by-id` to get every occurrence and the supporting scores.

Do not choose a smaller group only because you recognise one of its files. Use the report order unless the task already names a specific group.

## Read every occurrence

The same helper name can hide different behavior, and different names can hide one implementation. For each occurrence, record:

- the owning module and caller;
- input and output contracts;
- error and fallback behavior;
- side effects;
- performance constraints;
- tests that pin the behavior; and
- the differences Deslop's label asks you to inspect.

Deslop selects one occurrence as the reference copy. Choose the final shared implementation based on which module owns the behavior and which dependencies are allowed, not on which occurrence Deslop lists first.

## Duplication worth merging

Kevin Moore's protocol lists common candidates: copied helpers or decoders, repeated interfaces, repeated iterations over the same data, repeated sequences of commands, and generator code that can take inputs instead of being copied.

Ask whether the occurrences implement one concept and whether separate copies could receive different fixes over time. If one shared implementation can preserve types, behavior, performance, and clarity, the group is worth merging.

## Duplication that should stay separate

Similar code can be correct to retain. Examples include specialized hot loops whose unification introduces overhead, unrelated entry points where an abstraction hides more than it clarifies, statically distinct structures that would require unsafe casting, and test fixtures whose repetition is the subject of the test.

Record the decision and technical reason, then leave the source unchanged. Do not hide the group merely because it should remain separate.

## Decision record

```text
group id:
visible label:
rank and weight at baseline:
all occurrences inspected: yes / no
canonical comparison anchor:
candidate shared owner:
drift risk if retained:
type, behavior, performance, clarity, and test risks if merged:
decision: merge / keep separate / wait for more information
rationale:
verification plan:
```

Choose “wait for more information” only when a specific fact or missing test blocks the decision. Record what is missing.

## Workshop exercise

Inspect two Workshop groups. Merge one copied decoder whose differences can be expressed as named inputs. Keep another group separate when merging it would weaken unrelated static types. Record both decisions with stable IDs.

## Instruction for coding agents

```text
Start with `top-offenders`, inspect every occurrence in the selected group, and record whether to merge it. Do not recommend a merge from rank, score, or one occurrence alone.
```

## What came from the practitioner protocol

This chapter uses Kevin Moore's distinction between duplication worth merging and duplication that should remain separate. It combines that decision with Deslop's `top-offenders` tool and visible duplicate labels. A Deslop finding does not automatically mean the code is wrong.

## Source keys

- `kevmoo-duplication-audit`
- `deslop-for-ai`
- `deslop-taxonomy`
