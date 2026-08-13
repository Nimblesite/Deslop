# How to use this book

Coding agents can produce working code faster than a maintainer can build a mental index of the repository. That asymmetry creates a predictable failure: the next helper, decoder, fixture, or repository method is locally reasonable and globally redundant.

This book gives that decision back its missing repository context.

![The reading journey moves from prevention through a live authoring routine to evidence-led cleanup and a lower repository ceiling.](assets/diagrams/00-reading-journey.png)

*The book begins before a duplicate is written and ends with a lower ceiling that keeps the cleanup from unravelling.*

## Two jobs, in the right order

The cheapest duplicate is the one an agent never writes. Part I shows how Deslop answers the authoring question: **does an equivalent already exist?** The answer arrives before the new code unit enters the repository, while reuse is still a small decision.

Part II turns that successful check into a routine. Repository instructions tell every agent when to call `find-similar`, which evidence to inspect, how to recover when the live path is unavailable, and how to confirm the working set after an edit.

Part III deals with the repository you already have. Cleanup starts with a read-only baseline, not a refactor. You work from the highest-impact duplicate group, inspect every occurrence, decide whether consolidation is justified, establish a passing test baseline, make the smallest change, and prove both behavior and duplication moved in the intended direction.

## What Deslop decides

Deslop finds and ranks related source ranges. It tells you whether the code is identical, nearly identical, the same shape with different content, loosely similar, or the same behavior written differently. It provides source locations, stable group identities, similarity evidence, and recommendations appropriate to the label.

Deslop does not decide that a shared abstraction is good. It cannot own the domain boundary, choose the clearest module, prove that performance does not matter, or know that a fixture intentionally repeats production structure. Those remain engineering decisions.

The distinction is central:

> A finding restores context. A refactor spends judgment.

## The Workshop repository

Each chapter returns to a small service repository shaped by several agent sessions. The repository contains ordinary forms of drift: validators with different names, request decoders with slightly different defaults, command wrappers copied into multiple entry points, and fixtures whose repetition may be intentional.

The examples are not committed as unverified duplicate code in this structural edition. They will be added chapter by chapter after the Deslop release is pinned. Every checkpoint will include:

- the source state before the action;
- the exact product evidence;
- the action and its rationale;
- native static checks and tests; and
- the Deslop state after the action.

## Read the labels you actually see

The book uses the same human-facing clone names as the Deslop UX. It does not teach a parallel academic shorthand and ask you to translate while making a maintenance decision.

The [Glossary](#glossary) is the vocabulary authority. When a tool field uses a machine value such as `nearly_identical`, the glossary connects it to the visible title “Nearly identical code” and its current UX guidance.

## Evidence has provenance

A screenshot shows what the pinned Deslop build actually displayed. A diagram explains a workflow or relationship. An editorial illustration can establish a non-factual concept, but it cannot invent product output.

Before this scaffold becomes a publishable edition, `book.json` will pin the release and artifact hashes. `figures.json` will record the environment behind every capture, and `evidence.json` will record where specification, implementation, UX, and tests agree.

## How to use a chapter

Start with the repository state and predict what you would do. Read the Deslop evidence before the explanation. Complete the Workshop checkpoint, then copy the chapter's agent instruction or audit record into your own workflow.

If you are starting a new agent-enabled repository, read Parts I and II first. If you have inherited an existing mess, resist the urge to jump directly into extraction: read Chapters 4 and 6 for the evidence vocabulary, then begin Part III at the baseline.

The order is intentional. Prevention without cleanup leaves old risk in place. Cleanup without prevention guarantees another cleanup.
