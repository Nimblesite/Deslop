# Chapter 6 — Use Deslop evidence to decide what to do

> **Scaffold status:** Editorial structure established. Numeric captures await the edition release pin.

## What you will be able to do

Use Deslop's scores, source locations, stable group IDs, and guidance to decide what to inspect. Do not let one score automatically trigger a refactor.

## Planned sections

### How Deslop builds the score

Deslop compares parsed code structure, text overlap, and optional behavior-based similarity. It combines those results in the `signals.fused` score. The visible duplicate label explains the kind of match in the same words used by the editor.

### What the score ranges mean

The score ranges tell an agent whether to stop, read the closest occurrence, or proceed and check again. The developer must still decide code ownership and check types, behavior, and performance.

### Byte ranges identify the exact code

Line numbers help developers find code in an editor. Byte ranges identify the exact section in Deslop's machine-readable responses, even when edits above it change the line number.

### Group IDs stay stable while report order changes

Use the stable group ID in notes and before-and-after comparisons. A group's position can move after cleanup because Deslop sorts the updated report again.

### Each label requires a different check

Identical code is the clearest candidate for sharing one implementation. Nearly identical code requires the developer to list every difference. Same-shape, loosely-similar, and same-behavior findings require more inspection before any merge.

## Workshop exercise

Create a note for one group. Record its visible label, stable ID, source ranges, current rank, reference occurrence, and the facts you still need before deciding whether to merge it.

## Instruction for coding agents

```text
Use the fused score to choose the next inspection step. Use source evidence and repository ownership to choose the code change.
```

## Source keys

- `deslop-for-ai`
- `deslop-taxonomy`
