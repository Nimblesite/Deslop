# Chapter 4 — Understand a Deslop result

Deslop reports evidence in five plain-language labels. Each label tells you what kind of relationship was observed and how cautiously to act.

## What you will be able to do

Explain a duplicate group using the current UX title, its occurrences, evidence, and rank before proposing any code change.

## The five labels

### Identical code

Deslop has proved that every copy contains equivalent source. This is the clearest candidate for sharing one implementation, although the developer still chooses where that implementation should live.

### Nearly identical code

The copies are strongly alike, but small differences may matter. Inspect those differences and decide whether they are parameters, policy, drift, or evidence that the code should remain separate.

### Same shape, different content

Only the code shape is supported strongly enough. Sibling boilerplate can look like a reusable implementation while carrying unrelated content. Read the occurrences before extracting.

### Loosely similar code

The text overlaps weakly. Treat the group as a hint that can guide search, not as a consolidation plan.

### Same behavior, different code

Semantic analysis suggests two different-looking implementations perform the same job. Read both. Different code often encodes a reason the model cannot infer from similarity alone.

## What a duplicate group contains

The human report calls related sections of code a duplicate group. Tool responses and JSON commonly call the same thing a `cluster`. Each section is an occurrence. Deslop selects one occurrence as the reference copy, called the canonical occurrence. The developer may choose a different occurrence as the final shared implementation.

Rank helps you choose which group to inspect first. It does not say that a refactor is safe. The group at the top has the highest reported impact, but its occurrences may still need to remain separate.

## A useful evidence note

Before editing, capture:

```text
group id:
visible label:
occurrences and owners:
canonical occurrence:
differences that may matter:
why consolidation might reduce drift:
why consolidation might damage the design:
decision: investigate / consolidate / retain
```

This note records what Deslop found separately from the developer's decision about the code.

## Workshop exercise

Choose one Workshop group and explain it without using a numbered academic label, source-code enum, or vague synonym such as “basically the same.” Use the exact visible title, name every occurrence, and write one reason to consolidate and one reason to retain it.

## Instruction for coding agents

```text
Report Deslop's visible duplicate label first. Use rank only to choose what to inspect first. Treat the canonical occurrence as the reference copy, and read every occurrence before recommending a refactor.
```

## Source keys

- `deslop-taxonomy`
- `deslop-vscode-labels`
