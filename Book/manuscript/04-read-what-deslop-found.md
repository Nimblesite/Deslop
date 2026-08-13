# Chapter 4 — Read what Deslop found

Deslop reports evidence in five plain-language labels. Each label tells you what kind of relationship was observed and how cautiously to act.

## Reader outcome

Explain a duplicate group using the current UX title, its occurrences, evidence, and rank before proposing any code change.

## The five labels

### Identical code

Every copy is the same under Deslop's source-equivalence proof. This is the clearest extraction candidate, although ownership still requires a human or repository-aware agent decision.

### Nearly identical code

The copies are strongly alike, but small differences may matter. Inspect those differences and decide whether they are parameters, policy, drift, or evidence that the code should remain separate.

### Same shape, different content

Only the code shape is supported strongly enough. Sibling boilerplate can look like a reusable implementation while carrying unrelated content. Read the occurrences before extracting.

### Loosely similar code

The text overlaps weakly. Treat the group as a hint that can guide search, not as a consolidation plan.

### Same behavior, different code

Semantic analysis suggests two different-looking implementations perform the same job. Read both. Different code often encodes a reason the model cannot infer from similarity alone.

## A group is a collection, not a verdict

The human report calls related ranges a duplicate group. Tool responses and JSON commonly call it a cluster. Each range is an occurrence. The canonical occurrence is the tool's comparison and reuse anchor, not an architectural endorsement.

Rank answers **where should investigation begin?** It does not answer **is this refactor safe?** The highest-weight group is the worst offender because it offers the largest potential impact, not because every occurrence is automatically wrong.

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

This note forces the label and the engineering decision to remain separate.

## Workshop checkpoint

Choose one Workshop group and explain it without using a numbered academic label, source-code enum, or vague synonym such as “basically the same.” Use the exact visible title, name every occurrence, and write one reason to consolidate and one reason to retain it.

## Agent handoff

```text
Report the product's plain-language clone label first. Treat rank as triage and the canonical occurrence as an anchor. Read every occurrence before recommending a refactor.
```

## Source keys

- `deslop-taxonomy`
- `deslop-vscode-labels`
