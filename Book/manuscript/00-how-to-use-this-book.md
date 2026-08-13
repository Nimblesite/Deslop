# How to use this book

I wrote this book because coding agents can produce working code without reading every relevant file in a repository. That creates a predictable problem: an agent can write a good helper, decoder, fixture, or repository method that already exists somewhere else.

I will show you how to check the whole repository before the agent writes another copy. I will also show you how to remove copies that have already accumulated without forcing unrelated code into a bad abstraction.

## Why this matters when agents write code

A developer who has worked in one repository for years remembers many existing helpers and past design decisions. A coding agent usually starts with a prompt, a few selected files, search results, and repository instructions. It may not see a matching implementation in another package.

Agents also produce complete changes quickly. A new helper can gain callers and tests before a reviewer notices that the repository already contains the same code. The practical response is not to ask the agent to remember everything. Give it a repository-wide check at the point where it decides whether to write new code.

I built Deslop to provide that check. It also gives you a structured way to inspect and remove copies that previous changes have already introduced.

![The book first explains how to check proposed code, then how to remove existing duplication and lower the allowed duplication limit.](assets/diagrams/00-reading-journey.png)

*The book begins with a check before writing code. It ends by lowering the configured duplication limit so CI rejects future increases.*

## The book starts by stopping new copies

Checking before code is written takes less work than removing a copy later. Part I shows how an agent asks Deslop whether matching code already exists. When it does, the agent can reuse that code before adding another implementation and more tests.

Part II makes the check part of every change. Repository instructions tell every agent when to call `find-similar`, what to read in the response, what to do when the live agent connection is unavailable, and how to check the files it changed.

Part III deals with duplication that is already in the repository. First, save a Deslop report and run the repository's tests without changing code. Then inspect the largest duplicate groups, decide which ones should share an implementation, make one small change, and rerun the tests and Deslop.

## What Deslop reports and what you decide

Deslop finds related sections of source code and orders the groups by size and impact. It labels the code as identical, nearly identical, the same shape with different content, loosely similar, or the same behavior written differently. It also reports file locations, a stable ID for each group, similarity scores, and guidance for that label.

Deslop does not decide that the code should share a helper or class. It cannot choose the correct module, decide whether an extra function call affects performance, or know that a test fixture repeats code on purpose. The developer still makes those decisions.

Deslop shows you the matching code and related scores. You decide whether to change it, which code should remain, and where shared code should live.

## The Workshop repository

Each chapter returns to a small service repository after several agents have changed it. It contains validators with different names, request decoders with slightly different defaults, command wrappers copied into several programs, and test fixtures that may repeat code on purpose.

I have not committed unverified duplicate examples in this structural edition. I will add them chapter by chapter after I pin the Deslop release. Every checkpoint will include:

- the source state before the action;
- the exact product evidence;
- the action and its rationale;
- native static checks and tests; and
- the Deslop state after the action.

## Read the labels you actually see

The book uses the same human-facing clone names as the Deslop UX. It does not teach a parallel academic shorthand and ask you to translate while making a maintenance decision.

The [Glossary](#glossary) defines the terms used throughout the book. When a tool field uses a machine value such as `nearly_identical`, the glossary connects it to the visible title “Nearly identical code” and its current UX guidance.

## Show where the evidence came from

A screenshot shows what the exact Deslop version used for the book actually displayed. A diagram explains a workflow or relationship. An editorial illustration can establish a non-factual concept, but it cannot invent product output.

Before I publish this edition, I will record the exact Deslop release and file hashes in `book.json`. I will record how I captured every screenshot in `figures.json`. I will use `evidence.json` to record where the specification, implementation, UX, and tests agree.

## How to use a chapter

Start with the repository state and predict what you would do. Read the Deslop evidence before the explanation. Complete the Workshop checkpoint, then copy the chapter's agent instruction or audit record into your own workflow.

If you are starting a new repository that uses coding agents, read Parts I and II first. If the repository already contains duplication, read Chapters 4 and 6 to understand Deslop's report, then start Part III by recording the current report and test result.

You need both parts. Prevention does not remove copies that already exist. Cleanup does not stop an agent from adding new copies later.
