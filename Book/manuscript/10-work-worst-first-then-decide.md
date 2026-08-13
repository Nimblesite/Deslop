# Chapter 10 — Work worst-first, then decide

The top of the report is where investigation begins. It is not where judgment ends.

## Reader outcome

Use `top-offenders` to select a high-impact duplicate group, inspect every occurrence with `cluster-by-id`, and record whether consolidation is actionable, rejected, or deferred.

## Start with impact

Deslop ranks groups worst-first so cleanup effort begins where repeated source has the largest measured footprint. Pull a short top-offenders view, choose one stable group ID, then request its complete member list and evidence.

Do not start in the middle because one file happens to be familiar. Familiarity is not repository impact.

## Read every occurrence

The same helper name can hide different behavior, and different names can hide one implementation. For each occurrence, record:

- the owning module and caller;
- input and output contracts;
- error and fallback behavior;
- side effects;
- performance constraints;
- tests that pin the behavior; and
- the differences Deslop's label asks you to inspect.

The canonical occurrence is a comparison anchor. Choose the retained owner based on dependency direction and domain responsibility, not whichever range the report lists first.

## Actionable duplication

Kevin Moore's protocol identifies common positive cases: copied helpers or decoders, shared contracts, redundant iterations, repeated process orchestration, and generator scaffolding that can be parameterized without changing output.

The general test is:

> Do these occurrences represent one concept whose independent maintenance creates drift risk?

If yes, and one shared owner can preserve types, behavior, performance, and clarity, consolidation is actionable.

## Necessary or deliberate duplication

Similar code can be correct to retain. Examples include specialized hot loops whose unification introduces overhead, unrelated entry points where an abstraction hides more than it clarifies, statically distinct structures that would require unsafe casting, and test fixtures whose repetition is the subject of the test.

The correct outcome is not to hide the group. Record a rejected verdict with the technical reason and leave the source untouched.

## Verdict record

```text
group id:
visible label:
rank and weight at baseline:
all occurrences inspected: yes / no
canonical comparison anchor:
candidate shared owner:
drift risk if retained:
type, behavior, performance, clarity, and test risks if merged:
verdict: consolidate / reject / defer
rationale:
verification plan:
```

“Defer” means evidence or coverage is missing. It is not a euphemism for an unrecorded decision.

## Workshop checkpoint

Inspect two Workshop groups. Consolidate one copied decoder whose differences are explicit parameters. Reject one group whose unification would force unrelated static contracts behind a weaker interface. Record both verdicts with stable IDs.

## Agent handoff

```text
Work from top-offenders, inspect the complete group, and record an actionability verdict. Never recommend consolidation from rank, score, or one occurrence alone.
```

## What came from the practitioner protocol

This chapter generalizes Kevin Moore's actionable-versus-necessary architectural gate. The book adds Deslop's current worst-first MCP workflow and UX labels while preserving the protocol's core restraint: a finding is not automatically a bug.

## Source keys

- `kevmoo-duplication-audit`
- `deslop-for-ai`
- `deslop-taxonomy`
