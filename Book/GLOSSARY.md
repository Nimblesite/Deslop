# Glossary

This glossary is the vocabulary authority for *The Deslop Book*. Clone labels reproduce the current human-facing titles and guidance in the Deslop UX. The canonical product sources are `clients/vscode/src/types/report.ts` and `docs/specs/taxonomy.md`.

The manuscript uses plain-language labels. It does not substitute numbered academic taxonomy for the names readers see in Deslop.

## Clone labels

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

The occurrence Deslop returns as the stable reuse or comparison anchor for a group. “Canonical” means the tool selected an anchor; it does not mean the implementation is automatically the best architectural owner.

### Duplicate code

The book's umbrella term for source fragments that Deslop places in the same duplicate group. The label attached to the group explains what kind of similarity Deslop observed.

### Duplicate group

The human-facing collection of related occurrences shown together in Deslop. Machine-readable reports and MCP tools commonly call the same collection a `cluster`.

### Cluster

The report and tool name for a duplicate group. Use “duplicate group” in explanatory prose and `cluster` when naming a field, tool response, or stable ID.

### Occurrence

One source range that belongs to a duplicate group. A group contains two or more occurrences.

### Stable group ID

The deterministic identifier used to refer to one group across repeated runs. Use it in audit notes; do not use the group's rank as an identity because ranks move when the repository changes.

### Similarity evidence

The structural, textual, and optional semantic measurements Deslop combines when relating source fragments. Evidence supports a decision; it does not make the refactoring decision.

### Fused score

The combined similarity value exposed as `signals.fused`. Agent workflows use it as a decision aid: strong values block an uninspected copy, borderline values require reading the nearest occurrence, and weak values permit authoring followed by a recheck.

### Weight

The impact score used to order duplicate groups worst-first. Weight helps choose where investigation begins. It does not prove that a refactor is safe.

### Worst offender

The highest-weight duplicate group in the current report or filtered view.

### Duplication percentage

The repository-level duplicated-line measure compared with an optional configured ceiling. It is a gate metric, not a target to manipulate by hiding evidence.

### Hidden occurrence

An analysed occurrence excluded from the headline metric by a `report_hide` rule. Hidden does not mean unanalysed; it preserves evidence such as hand-written code matching generated code.

## Workflow terms

### Address a finding

Investigate a duplicate group and give it an explicit outcome: reuse an existing owner, move ownership, delete a redundant path, generate required repetition, or retain the copies with a recorded technical reason. Addressing a finding does not automatically mean creating an abstraction.

### DRY principle

Don't Repeat Yourself is a design principle about giving knowledge or intent one authoritative representation. The book does not use DRY as a synonym for textual deduplication and does not describe Deslop as a DRY enforcement engine.

### Prevention loop

The authoring sequence: describe the proposed code, call `find-similar`, inspect the nearest occurrence, reuse or proceed, and recheck the changed working set.

### Cleanup loop

The remediation sequence: establish a baseline, start with `top-offenders`, inspect a group with `cluster-by-id`, record an actionability verdict, make a bounded change, run tests and analysis, and rescan.

### Actionable duplication

An editorial audit verdict: the repeated code represents one maintainable concept, carries drift risk, and can be consolidated without degrading types, behavior, performance, or clarity. This is a human decision, not a Deslop clone label.

### Necessary or deliberate duplication

An editorial audit verdict: repetition is retained because unifying it would damage a more important property or because the repetition is intentionally part of a fixture or boundary. The decision is recorded with a reason rather than hidden silently.

### Baseline

The test, static-analysis, and Deslop state recorded before remediation. A passing baseline makes a meaningful before/after claim possible.

### Duplication ceiling

The configured maximum duplication percentage allowed by the repository gate. After verified cleanup, the ceiling can ratchet downward; it is never widened merely to make a failing run pass.

### Read-only discovery

An audit run that writes reports to dedicated scratch storage, disables repository cache writes, and makes no source changes.

### Surgical consolidation

A bounded refactor that touches only the ownership, parameterization, call sites, and tests required to remove one justified duplicate group.
