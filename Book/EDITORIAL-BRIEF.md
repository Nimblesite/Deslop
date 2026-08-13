# Editorial brief

## Positioning

*The Deslop Book* is the field guide between installation documentation and an improvised repository-wide cleanup. It teaches a prevention-first agent workflow, then an empirical method for repairing duplication that already exists.

It is not a list of commands and it is not a promise that every similar block should become an abstraction. A finding is useful because it restores repository context at the moment a human or agent has to decide.

## DRY boundary

The book treats Don't Repeat Yourself respectfully and precisely: DRY is a design principle about giving knowledge or intent one authoritative representation. Deslop is not a DRY enforcement engine. It supplies empirical evidence that source already repeats.

Large identical slabs always require an explicit response, regardless of the reader's position on early abstraction. A response may be reuse, relocated ownership, deletion, generation, or deliberate retention with a recorded technical reason. We never use “address this finding” as a synonym for “extract a helper.”

## Reader

The primary reader can maintain a tested software repository and has used at least one coding agent. They may own repository instructions, editor tooling, or CI, but they do not need prior knowledge of clone-detection research, LSP, MCP, embeddings, or Deslop internals.

Experienced maintainers should still find the audit, actionability, evidence, and threshold-ratchet chapters useful.

## Voice

- Direct, urgent, and technically exact
- Human first, with instructions precise enough for an agent to follow
- Define a term at first use, then use the glossary form consistently
- Prefer an observed repository state over an analogy
- Use numbers only when they come from a captured report or a governing contract
- Never shame an agent for limited context; design the feedback loop that supplies it
- Never imply that similar-looking code is automatically wrong
- Avoid academic clone numbering in manuscript prose
- Never describe Deslop as an automated DRY rule

Paragraphs should usually make one move. Command blocks should fit without horizontal scrolling on a small e-reader. Evidence notes should name the source, edition version, workspace, and capture method.

## Teaching pattern

Start from a decision a developer or agent actually faces:

- Does this helper already exist under another name?
- Should this duplicate group become one implementation?
- Which occurrence should become canonical?
- Is this repetition deliberate, or is it drift waiting to happen?
- Did the refactor preserve behavior and reduce the reported group?

Then show evidence, explain the smallest governing principle, take one bounded action, and check the result. Ask readers to predict what Deslop will report before revealing the capture whenever prediction exposes their mental model.

## Prevention-first law

Every authoring chapter reinforces one sequence:

1. Describe the code unit about to be written.
2. Call `find-similar` before creating it.
3. Read the canonical occurrence when evidence is strong or borderline.
4. Reuse, extract, or proceed based on the evidence.
5. Recheck the changed working set.

The book never presents an after-the-fact scan as equivalent to this loop. CLI fallback can catch a duplicate immediately after writing; only the live snippet query can prevent an unwritten copy.

## Cleanup protocol

Kevin Moore's Deslop duplication audit skill contributes five durable gates to the cleanup half of the book:

1. **Read-only discovery.** Put reports in scratch storage and keep the target repository unchanged.
2. **Alignment before architecture.** Present the highest-impact findings before choosing refactors.
3. **Actionability verdict.** Separate consolidation candidates from repetition that is necessary or deliberately retained.
4. **Empirical baseline.** Run the repository's real tests before changing code; a broken baseline changes what can be claimed.
5. **Surgical change and proof.** Touch only what the consolidation requires, then rerun analysis, static checks, and tests.

The book adapts those gates to Deslop's current live-server workflow, distribution model, repository rules, and UX vocabulary. Source-specific version-control instructions are not generalized into universal policy.

## Agreement gate

A Deslop behavior is publishable only when these sources agree:

1. the governing repository specification;
2. the pinned edition implementation;
3. the human-facing UX copy; and
4. executable tests or captured behavior from that implementation.

If they disagree, omit the claim and capture until the repository resolves the discrepancy. The glossary follows the human-facing UX labels in `clients/vscode/src/types/report.ts` and the canonical definitions in `docs/specs/taxonomy.md`.

## Chapter limits

- 1,900–3,200 words
- Four to seven core sections
- Four to ten short command, report, or code blocks
- Two to four purposeful visuals
- One Workshop checkpoint
- One paste-ready agent rule or evidence template
- Five to seven closing takeaways

## Evidence standards

- Every command result comes from the pinned edition binary.
- Every product visual is a direct capture; conceptual diagrams never imitate product UI.
- Every numeric report example records its source artifact.
- Every consolidation chapter shows the test state before and after.
- Every accepted duplicate names the group and rationale rather than silently hiding it.
- Every UX label agrees with `GLOSSARY.md`.

## Corrections and maintenance

Each edition records its build date, Deslop version, supported surface versions, example test result, link-audit result, and screenshot environment. Readers are sent to the Deslop documentation and repository issue tracker for live corrections.
