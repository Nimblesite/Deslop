# Editorial brief

## Positioning

*The Deslop Book* explains how developers can use Deslop during normal coding work. It first shows how an agent checks for existing code before writing more code. It then shows how to find and remove duplication that is already in a repository.

It is not a list of commands and it is not a promise that every similar block should become an abstraction. A finding is useful because it shows matching code that may sit outside the files a human or agent has read.

## DRY boundary

The book treats Don't Repeat Yourself respectfully and precisely. DRY is a design principle about keeping each piece of knowledge in one authoritative place. Deslop does a different job: it finds source code that is already repeated.

Large identical slabs always require an explicit response, regardless of the reader's position on early abstraction. A response may be reuse, relocated ownership, deletion, generation, or deliberate retention with a recorded technical reason. We never use “address this finding” as a synonym for “extract a helper.”

## Reader

The primary reader can maintain a tested software repository and has used at least one coding agent. They may own repository instructions, editor tooling, or CI, but they do not need prior knowledge of clone-detection research, LSP, MCP, embeddings, or Deslop internals.

Experienced maintainers should still find the chapters on cleanup decisions, report evidence, and CI duplication limits useful.

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
- Prefer concrete subjects and actions over compressed metaphors; a less-experienced developer should understand every sentence on the first read
- Use headings that state the task or question; do not use campaign slogans as headings
- Do not use a clever contrast in place of an explanation
- Name the person or tool performing an action instead of hiding the action behind an abstract noun
- Introduce the ordinary phrase first, then the product or protocol term: “the folder Deslop scans (the repository root)”
- Replace “surface” with the actual thing: editor view, agent tool, CLI report, or CI result
- Replace “working set” with the files being changed
- Replace “verdict” with decision, “bounded” with small or focused, and “surgical” with minimal

Paragraphs should usually make one move. Command blocks should fit without horizontal scrolling on a small e-reader. Evidence notes should name the source, edition version, workspace, and capture method.

## Teaching pattern

Start from a decision a developer or agent actually faces:

- Does this helper already exist under another name?
- Should this duplicate group become one implementation?
- Which occurrence should become canonical?
- Is this repetition deliberate, or is it drift waiting to happen?
- Did the refactor preserve behavior and reduce the reported group?

Then show the relevant Deslop result, explain the rule needed for this decision, take one small action, and check the result. When useful, ask readers to predict the result before showing the real capture so they can compare their expectation with the tool output.

## Required check before writing code

Every authoring chapter reinforces one sequence:

1. Describe the code unit about to be written.
2. Call `find-similar` before creating it.
3. Read the canonical occurrence when evidence is strong or borderline.
4. Reuse, extract, or proceed based on the evidence.
5. Recheck the files that changed.

The book does not present an after-the-fact scan as equivalent to this check. A CLI scan can catch a duplicate immediately after writing. Only a live snippet query can check proposed code before it is written.

## Steps for cleaning up existing duplication

Kevin Moore's Deslop duplication audit skill contributes five durable gates to the cleanup half of the book:

1. **Read-only discovery.** Put reports in scratch storage and keep the target repository unchanged.
2. **Review findings before choosing a design.** Present the largest duplicate groups before choosing refactors.
3. **Decide whether each group should be merged.** Separate useful consolidation candidates from repetition that should remain separate.
4. **Run the real tests first.** Record the result before changing code; existing test failures limit what the cleanup can prove.
5. **Make a minimal change and check it.** Touch only what the consolidation requires, then rerun Deslop, static checks, and tests.

The book adapts those gates to Deslop's current live-server workflow, distribution model, repository rules, and UX vocabulary. Source-specific version-control instructions are not generalized into universal policy.

## Accuracy check for product claims

A Deslop behavior is publishable only when these sources agree:

1. the governing repository specification;
2. the pinned edition implementation;
3. the human-facing UX copy; and
4. executable tests or captured behavior from that implementation.

If they disagree, omit the claim and screenshot until the product resolves the discrepancy. The glossary follows the labels in `clients/vscode/src/types/report.ts` and the definitions in `docs/specs/taxonomy.md`.

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
- Every chapter that merges code shows the test state before and after.
- Every accepted duplicate names the group and rationale rather than silently hiding it.
- Every UX label agrees with `GLOSSARY.md`.

## Corrections and maintenance

Each edition records its build date, Deslop version, supported editor and agent integrations, example test result, link-check result, and screenshot environment. Readers are sent to the Deslop documentation and repository issue tracker for current corrections.
