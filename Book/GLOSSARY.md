# Glossary

This glossary defines the terms used in *The Deslop Book*. The five duplicate labels use the exact titles and guidance shown in Deslop. The product sources are `clients/vscode/src/types/report.ts` and `docs/specs/taxonomy.md`.

The book uses the labels developers see in Deslop. It does not replace them with numbered academic jargon.

## Deslop duplicate labels

### Identical code

Copies whose source is equivalent under Deslop's byte-level proof. The UX guidance is: **“Safe to extract — every copy is the same.”** This is the strongest consolidation candidate, but the maintainer still chooses ownership and the final abstraction.

Wire value: `identical`.

### Nearly identical code

Copies that are strongly alike but contain differences that may carry behavior or policy. The UX guidance is: **“Review the locations — small differences may matter.”** Consolidation usually begins by naming those differences explicitly.

Wire value: `nearly_identical`.

### Same shape, different content

Code whose structure lines up while textual or semantic support is insufficient. The UX guidance is: **“Only the code shape matches — usually sibling boilerplate. Verify before extracting.”** This is a prompt to inspect, not a refactoring instruction.

Wire value: `structural_only`.

### Loosely similar code

Code with weak textual overlap. The UX guidance is: **“Loose textual overlap. Treat as a hint.”** It can lead to a useful investigation, but it does not justify consolidation by itself.

Wire value: `loosely_similar`.

### Same behavior, different code

Different-looking implementations that semantic analysis suggests perform the same job. The UX guidance is: **“The AI noticed these do the same thing written two ways — read both before merging.”** This label is available only when the embedding analysis runs.

Wire value: `same_behavior`.

## Report terms

### Canonical occurrence

The occurrence Deslop selects as the reference copy for a duplicate group. Tool responses call it the `canonical` occurrence. It is a useful place to start comparing code, but the developer may choose a different occurrence as the final shared implementation.

### Duplicate code

The book's umbrella term for source fragments that Deslop places in the same duplicate group. The label attached to the group explains what kind of similarity Deslop observed.

### Duplicate group

The human-facing collection of related occurrences shown together in Deslop. Machine-readable reports and MCP tools commonly call the same collection a `cluster`.

### Cluster

The report and tool name for a duplicate group. Use “duplicate group” in explanatory prose and `cluster` when naming a field, tool response, or stable ID.

### Occurrence

One source range that belongs to a duplicate group. A group contains two or more occurrences.

### Stable group ID

An ID that Deslop uses to refer to the same duplicate group across repeated runs. Put this ID in cleanup notes. Do not use the group's position in the report as its ID because that position can change.

### Similarity evidence

The structure, text overlap, and optional behavior-based scores Deslop uses to compare sections of code. These scores explain why code was grouped together. The developer still decides whether to change it.

### Fused score

The combined similarity score in the `signals.fused` report field. Agent instructions use this score to choose the next step: stop and inspect a strong match, read the closest result for a borderline match, or write the new code and check again for a weak or empty result.

### Weight

The score Deslop uses to put larger or more important duplicate groups near the top of the report. Weight helps you choose which group to inspect first. It does not prove that merging the code is safe.

### Worst offender

The highest-ranked duplicate group in the current report or filtered view. Deslop's agent tool for this ranked list is called `top-offenders`.

### Duplication percentage

The percentage of analysed source lines that Deslop counts as duplicated. A repository can set a maximum allowed percentage and fail CI when the result is higher.

### Hidden occurrence

An occurrence that Deslop still analyses but leaves out of the main duplication percentage because it matches a `report_hide` rule. The occurrence remains in the report, which is useful when hand-written code matches generated code.

## Workflow terms

### Address a finding

Investigate a duplicate group and give it an explicit outcome: reuse an existing owner, move ownership, delete a redundant path, generate required repetition, or retain the copies with a recorded technical reason. Addressing a finding does not automatically mean creating an abstraction.

### DRY principle

Don't Repeat Yourself is a design principle about giving knowledge or intent one authoritative representation. The book does not use DRY as a synonym for textual deduplication and does not describe Deslop as a DRY enforcement engine.

### Check before writing

The steps an agent follows before adding code: describe the proposed code, call `find-similar`, inspect the closest existing occurrence, reuse it or write new code, and then check the changed files.

### Cleanup steps

The steps for existing duplication: record the current test and Deslop results, start with `top-offenders`, inspect one group with `cluster-by-id`, decide whether to merge it, make one focused change, run the tests, and run Deslop again.

### Duplication worth merging

A developer's decision that several copies represent one concept and can share an implementation without damaging types, behavior, performance, or clarity. This is a book term, not a Deslop label.

### Duplication that should stay separate

A developer's decision to keep repeated code because sharing an implementation would damage a more important property, or because a test or external boundary requires the repetition. Record the reason instead of silently hiding the finding.

### Baseline

The test, static-analysis, and Deslop results recorded before cleanup. These results let you compare the repository before and after the change.

### Duplication limit

The maximum duplication percentage allowed by the repository configuration. After verified cleanup, lower the limit to the new result. Do not raise it just to make CI pass.

### Read-only scan

A Deslop run that writes reports to a temporary directory, disables cache writes inside the target repository, and does not change source files.

### Minimal consolidation

A refactor that changes only the shared implementation, required inputs, call sites, and tests needed to remove one duplicate group.
