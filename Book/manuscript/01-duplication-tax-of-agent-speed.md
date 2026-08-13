# Chapter 1 — Why coding agents duplicate code

An agent receives a specific task: validate an incoming request, normalize two fields, and return a domain value. It reads the files named in the task, writes a clear implementation, adds tests, and finishes quickly.

The implementation is correct. It is also the second copy of a validator already living in a neighbouring package.

Nothing in the new function announces the mistake. The name is different. The surrounding types are different. The tests pass. The agent reasoned well about the code it could see. It did not check whether the behavior already existed elsewhere in the repository.

The problem is simple: agents can write code faster than they can check every relevant file. The workflow therefore needs an explicit repository-wide duplicate check.

## What you will be able to do

By the end of this chapter, you should be able to:

- distinguish the DRY principle from Deslop's job;
- explain why large regions of identical code always deserve a response;
- separate “address this finding” from “invent an abstraction”;
- identify the cheapest point to stop an agent-created duplicate; and
- trace Deslop's approach to the primary research that underpins it.

## DRY deserves an accurate reading

Don't Repeat Yourself has earned a devoted following. In some teams that devotion becomes a reflex: two similar-looking pieces of code appear, so an abstraction must appear immediately. That reflex is understandable, but it is not the full principle.

Andy Hunt and Dave Thomas define DRY in *The Pragmatic Programmer* as follows:

> “DRY is about the duplication of knowledge, of intent.”

Their [official excerpt](https://media.pragprog.com/titles/tpp20/dry.pdf) explicitly says that avoiding copied source lines is only a small part of DRY. Two pieces of code can look the same while representing independent knowledge. Conversely, one rule can be duplicated across code, documentation, a schema, and a test even though none of those representations looks alike.

DRY is therefore a design principle. It asks:

> Where is the single authoritative representation of this knowledge?

That question can lead to excellent abstractions. Applied too early, however, it can also ask one abstraction to own concepts that merely happen to resemble one another today. Sandi Metz captured the trade-off in her essay [*The Wrong Abstraction*](https://sandimetz.com/blog/2016/1/20/the-wrong-abstraction):

> “duplication is far cheaper than the wrong abstraction”

That is not an argument for careless copying. It is an argument for waiting until the shared concept is understood. The strongest version of DRY does not require every textual repetition to share an owner, and responsible critics of premature abstraction are not defending undisciplined repositories.

Deslop asks a different question.

![DRY asks where knowledge belongs; Deslop asks what source already repeats and supplies evidence for a response.](assets/diagrams/01-dry-vs-deslop.png)

*Figure 1.1 — DRY helps developers decide where knowledge should live. Deslop finds repeated source code. A team can use both without treating every match as a reason to create an abstraction.*

## Deslop is not a DRY enforcement engine

Deslop does not decide whether two business concepts ought to share an abstraction. It analyses the repository and reports repeated source ranges using the labels readers see in the product: **identical code**, **nearly identical code**, **same shape, different content**, **loosely similar code**, and **same behavior, different code**.

It answers a concrete question:

> What source already repeats, where are the occurrences, and how strong is the evidence connecting them?

That distinction matters because Deslop often finds large slabs of identical code. At that point, no one is speculating about a future abstraction. The copy already exists. The repository already has multiple places where the same source can be read, reviewed, fixed, secured, and allowed to drift.

You can reject DRY as a universal refactoring trigger and still take that evidence seriously. You can prefer a little duplication while a design emerges and still refuse to let an agent paste a hundred-line implementation beside an existing one. These are compatible positions.

Deslop's current UX tells the reader that identical code is “Safe to extract — every copy is the same.” The [taxonomy specification](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/taxonomy.md) requires proof that the source ranges are equivalent. A structural score by itself is not enough for this label.

“Safe to extract” describes the sameness of the copies. It does not choose the final owner, approve a dependency direction, or prove that a new helper is the best design. The evidence is strong; the architectural decision remains yours.

## Identical slabs must be addressed

A large identical slab is always worth addressing, regardless of your position on DRY. In this book, **address** means investigate the group and give it an explicit outcome. It does not mean mechanically extract every match.

There are several legitimate outcomes:

1. **Reuse the existing owner.** One occurrence is already in the right module. Remove the copy and call or import the owner.
2. **Move ownership.** Neither occurrence is in a neutral place. Move the implementation to the narrowest shared boundary, then update both callers.
3. **Delete a redundant path.** The copy belongs to dead, superseded, or unreachable code. Remove it instead of abstracting it.
4. **Generate the repetition.** A schema, protocol, or platform boundary requires repeated output. Keep one source of generation and verify the products.
5. **Retain it deliberately.** Isolation, performance, fixtures, or a boundary may justify separate copies. Record the reason and the duplicate group's stable ID so the next maintainer does not repeat the investigation.

The fifth outcome is important. Hunt and Thomas give an example in which identical validation code represents different knowledge. Kevin Moore's [Deslop Duplication Audit Protocol](https://github.com/kevmoo/kevmoo_skills/blob/main/skills/deslop-duplication-audit/SKILL.md) likewise separates duplication worth merging from duplication that should stay separate and requires a written technical reason before editing.

This does not mean every copy becomes a helper. It means every large identical block is inspected. The developer then reuses an existing implementation, moves the code, deletes a redundant path, generates required repetition, or records why the copies must remain separate.

![Three identical source slabs enter an evidence review and leave with one explicit ownership decision.](assets/diagrams/01-identical-code-needs-a-verdict.png)

*Figure 1.2 — Identical code proves that the repository contains repeated source. The developer still chooses whether to reuse, move, delete, generate, or deliberately retain it.*

## Nearly identical code requires more judgment

Identical code is the cleanest case because the copied source is proven equivalent. Other Deslop labels deliberately slow the reader down.

- **Nearly identical code** means the locations are strongly alike, but small differences may matter. Name every difference before consolidating.
- **Same shape, different content** means the structure lines up without enough content support. It is often sibling boilerplate. Inspect before extracting.
- **Loosely similar code** is a hint, not a refactoring instruction.
- **Same behavior, different code** is an optional semantic signal. Read both implementations and their tests before merging them.

Here the design argument returns. Two similar validators may encode separate policies that currently coincide. Two decoders may share a stable algorithm with one parameterized difference. Deslop supplies the locations and similarity evidence; DRY, coupling, ownership, performance, and domain boundaries influence the decision.

The glossary separates the Deslop result from the developer's decision:

- a **duplicate group** is evidence returned by Deslop;
- **duplication worth merging** is a developer's decision that the copies should share an implementation; and
- **duplication that should stay separate** is a decision to keep repetition for a recorded technical reason.

If a team confuses the Deslop result with the decision, it can make either of two mistakes. It may ignore a large copied block because some duplication is intentional, or it may create a poor abstraction because two sections look similar. Read the result first, then decide what the code should share.

## Why agents create copies more often

Copying code predates coding agents. The foundational literature describes programmers reusing code by copying a fragment and adapting it to a new context. Agents change the production rate and the mechanics.

An agent typically works from a prompt, several open files, search results, repository instructions, and tool responses. The repository contains more code than the agent has read. If the existing implementation sits elsewhere, generating another implementation may be the shortest path to a passing result.

This does not mean the agent is incompetent. Code can work correctly and still duplicate code elsewhere in the repository.

Recent research makes the risk concrete. Liu and colleagues studied 19 code-generating models across three benchmarks and found that “repetition is pervasive” at character, statement, and block granularity. Their paper, [*Code Copycat Conundrum*](https://arxiv.org/abs/2504.12608), examines repetition inside generated outputs. Deslop addresses the adjacent repository problem: whether proposed or committed code repeats source that is already elsewhere in the codebase.

The practical response is to give the agent a repository-level question at the moment it matters:

```text
Before writing this code unit, does an equivalent implementation already exist?
```

That is the role of `find-similar`. The agent describes the proposed source before adding it, reads the strongest existing occurrence, and reuses or adapts the owner when the evidence supports doing so. The agent does not need to read the whole repository. It needs to run a repository-wide check before writing the new code.

## Research methods used by Deslop

Deslop is product engineering built on a long research lineage, not a visual search bolted onto an opinion about clean code. The current implementation maps its techniques to source files in Deslop's [Research Background](https://deslop.live/docs/research-background/). The primary papers establish the foundations:

1. **Compare parsed program structure.** Baxter and colleagues' 1998 paper, [*Clone Detection Using Abstract Syntax Trees*](https://doi.org/10.1109/ICSM.1998.738528), presented methods for detecting identical and edited repetitions over arbitrary program fragments using abstract syntax trees. Deslop similarly parses supported languages and normalizes syntax before structural comparison.
2. **Fingerprint syntax trees for efficient exact matching.** Chilowicz, Duris, and Roussel proposed [“a simple and scalable architecture based on AST fingerprinting”](https://doi.org/10.1109/ICPC.2009.5090050). Deslop computes bottom-up fingerprints for normalized subtrees and extends coverage across neighbouring statements.
3. **Recover edited copies at repository scale.** Sajnani and colleagues' [*SourcererCC*](https://arxiv.org/abs/1512.06448) uses indexed token evidence and filtering to find edited copies in very large corpora. Deslop adapts that line of work with normalized syntax-kind sequences, compact similarity signatures, and locality-sensitive indexing.
4. **Estimate overlap without comparing everything to everything.** Deslop's compact signatures and indexing draw on Broder's [resemblance-and-containment work](https://doi.org/10.1109/SEQUEN.1997.666900) and Indyk and Motwani's [locality-sensitive hashing research](https://doi.org/10.1145/276698.276876). These methods make it practical to retrieve plausible neighbours without an exhaustive all-pairs comparison.
5. **Add semantic recall without pretending it is proof.** Gul Aftab Ahmed and colleagues' [SSCD research](https://doi.org/10.1002/spe.3355) studies neural representations with approximate-neighbour search for large industrial codebases. Deslop's optional embedding pass contributes the **same behavior, different code** signal; the UX explicitly tells the reader to inspect both implementations before merging.

These methods find different kinds of repetition. Exact structural fingerprints find equivalent parsed code. Indexed text overlap finds copies that have been edited. Optional behavior-based similarity can find code that looks different. Deslop combines the results and presents the five plain-language labels used in its editor and reports.

Research supports the detection mechanisms. It does not absolve the maintainer from choosing ownership, preserving behavior, or running tests.

## Checking before writing requires the least work

There are three opportunities to deal with an agent-created copy:

| When you check | Repository state | Required work |
|---|---|---|
| Before authoring | The duplicate is only a proposal | Query, inspect, reuse |
| Immediately after the edit | The copy exists but has not accumulated history | Scan, compare, replace, retest |
| After divergence | Copies have separate fixes, tests, and callers | Reconstruct intent, reconcile differences, consolidate, verify |

The first path has the fewest decisions. The agent can abandon a proposal without migrating callers or proving that two histories still agree. The second path is still cheap enough to make a good fallback. The third is the cleanup problem addressed in Part III: valuable, necessary, and much more demanding.

This is why Deslop checks proposed code. Cleanup removes copies that already exist. The `find-similar` check can stop the next copy before it is added.

## Workshop exercise

The Workshop agent has been asked to add a request validator. Do not write it yet. Create a proposed-code note outside the repository:

```text
Behavior: validate an incoming request and normalize two fields
Inputs: request payload plus repository policy
Output: validated domain value or typed failure
Likely location: request-handling package
Search neighbours: sibling transport package; shared domain package
Potential owner: existing request decoder or validator
```

Now answer these questions:

1. Which parts describe stable domain knowledge, and which parts are transport-specific?
2. If Deslop finds a large identical validator, what must be addressed before any new code is written?
3. If it finds nearly identical code, what differences must be named before consolidation?
4. What evidence would justify retaining two copies deliberately?

Expected conclusion: an identical match blocks uninspected authoring. Read the existing occurrence and decide whether to reuse it, move ownership, delete a redundant path, generate the repeated output, or record why isolation is necessary. A weaker match requires investigation, not automatic extraction.

Chapter 3 turns this note into a live `find-similar` query. The important point is that Deslop can check the proposed code before the agent adds it to a file.

## Instruction for coding agents

```text
Deslop is not a DRY enforcement rule. Before authoring a new code unit,
describe the proposed source and call find-similar. A large identical match
must be addressed: reuse, relocate, delete, generate, or retain with a recorded
technical reason. For every weaker label, inspect the occurrences and name the
differences before deciding whether to consolidate.
```

## Main points

- DRY remains a principle about authoritative knowledge and intent.
- Deslop remains an evidence system for repeated source already in the repository.
- Premature abstraction is a design risk; an existing identical slab is an observed fact.
- Addressing a duplicate does not always mean extracting a helper.
- Agents need a repository-wide duplicate check; you cannot assume they have read every relevant file.
- Prevention before authoring is cheaper than reconciliation after copies diverge.
- Deslop's structural, overlap, indexing, and optional semantic techniques have a traceable scholarly lineage.

## Authoritative sources

- Andy Hunt and Dave Thomas, [*The Pragmatic Programmer*, “The Evils of Duplication” official excerpt](https://media.pragprog.com/titles/tpp20/dry.pdf).
- Sandi Metz, [“The Wrong Abstraction”](https://sandimetz.com/blog/2016/1/20/the-wrong-abstraction).
- Ira D. Baxter et al., [“Clone Detection Using Abstract Syntax Trees”](https://doi.org/10.1109/ICSM.1998.738528), ICSM 1998.
- Michel Chilowicz, Étienne Duris, and Gilles Roussel, [“Syntax Tree Fingerprinting for Source Code Similarity Detection”](https://doi.org/10.1109/ICPC.2009.5090050), ICPC 2009.
- Hitesh Sajnani et al., [“SourcererCC: Scaling Code Clone Detection to Big Code”](https://arxiv.org/abs/1512.06448), ICSE 2016.
- Andrei Z. Broder, [“On the Resemblance and Containment of Documents”](https://doi.org/10.1109/SEQUEN.1997.666900), 1997.
- Piotr Indyk and Rajeev Motwani, [“Approximate Nearest Neighbors”](https://doi.org/10.1145/276698.276876), STOC 1998.
- Gul Aftab Ahmed et al., [“Nearest-neighbor, BERT-based, scalable clone detection”](https://doi.org/10.1002/spe.3355), *Software: Practice and Experience*, 2024.
- Mingwei Liu et al., [“Code Copycat Conundrum”](https://arxiv.org/abs/2504.12608), 2025 preprint.
- Kevin Moore, [*Deslop Duplication Audit Protocol*](https://github.com/kevmoo/kevmoo_skills/blob/main/skills/deslop-duplication-audit/SKILL.md).
- Deslop, [Research Background](https://deslop.live/docs/research-background/) and [canonical UX taxonomy](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/taxonomy.md).
