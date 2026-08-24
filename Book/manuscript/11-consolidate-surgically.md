# Chapter 11 — Merge one duplicate group safely

> **Current status:** This chapter is an outline. I will include tested code changes from the Workshop repository.

## What you will be able to do

I will show you how to merge one duplicate group with the smallest practical code change. You will preserve public interfaces, behavior, performance, and readability.

## Planned sections

### Run the repository's tests before editing

Run the repository's normal dependency, static-analysis, and test commands before editing. If they already fail, record the failures. You cannot claim that the cleanup caused or fixed a result without a before-and-after comparison.

### Choose where the shared code should live

Place the shared implementation in the module responsible for the behavior. If neither current module is suitable, use the smallest shared module that both callers are allowed to depend on. Do not move the code to a general `utils` file merely because every module can import it.

### Give real differences clear inputs

Nearly identical code often contains small but important policy differences. Represent a real difference with a clearly named input or separate strategy only when each caller can describe it. Avoid boolean flags that hide two unrelated implementations inside one function.

### Keep separate code when sharing would cause harm

Do not weaken static types or slow performance-sensitive code merely to remove lines. Keeping the code separate is a valid result when the cleanup note explains why.

### Change only what the merge requires

Change the shared implementation, its callers, and the tests required by the decision. Do not combine the cleanup with broad formatting or unrelated design changes.

## Workshop exercise

Run the passing baseline, extract the chosen decoder into its correct owner, adapt both callers, and add assertions that preserve each meaningful policy difference.

## Check your understanding

1. Why must the repository's tests run before cleanup begins?
2. How do you choose the module that should own the shared implementation?
3. When is keeping two specialised implementations safer than sharing code?

## Instruction for coding agents

```text
Merge one accepted duplicate group at a time. Preserve every public interface and every difference required by tests. Leave unrelated cleanup for a separate change.
```

## How Kevin Moore's protocol is used

Kevin Moore's protocol requires real test results before editing and limits the change to the selected duplicate group. The finished examples will use the Workshop repository's own tools instead of assuming Dart- or Flutter-specific commands.

## Sources used for this chapter

- `kevmoo-duplication-audit`
- `deslop-for-ai`
