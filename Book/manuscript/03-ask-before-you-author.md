# Chapter 3 — Ask before you author

The prevention loop begins while the proposed implementation is still cheap to abandon.

## Reader outcome

Call `find-similar` before writing a function, method, class, helper, fixture, test setup, parser branch, error type, route handler, view model, or other code unit larger than a few lines. Use the response to reuse, inspect, or proceed.

## Describe behavior before names

Names are weak search keys. Two agents can call the same behavior `parseOrder`, `decodePurchase`, and `orderFromPayload`. Deslop compares the proposed source structure and content rather than depending on the name the current agent happened to choose.

Prepare one of two inputs:

- a proposed snippet plus its language, when the code does not exist yet; or
- a file path and byte range, when the draft already lives in an editor buffer or scratch file.

The first form is prevention. The second can still catch a copy immediately while the change is in working memory.

## Use a decision ladder

The current agent guidance uses three score bands:

| Evidence | Action |
|---|---|
| Fused score at or above `0.85` | Do not write the copy. Read and reuse the canonical occurrence, or extract a shared implementation. |
| Fused score from `0.6` up to `0.85` | Read the nearest occurrence before deciding. Bias toward reuse. |
| Fused score below `0.6`, or no result | Proceed, then recheck the changed working set. |

The label modifies the response. “Same shape, different content” requires inspection even when structure is strong. “Same behavior, different code” asks you to reconcile intent before merging. The glossary gives the complete human-facing guidance.

The score is not a permission token. It supplies evidence to a repository decision.

## Reuse is broader than calling a helper

When Deslop finds an existing implementation, the new call site may be able to import and call it directly. If ownership is wrong for both locations, the correct move may be to extract a shared implementation into a neutral module. If the proposed behavior is genuinely different, make that difference explicit before authoring.

The prevention question is therefore:

> What is the smallest change that gives this caller the existing behavior without creating another owner?

## When the live path is unavailable

First diagnose the live connection: the editor server may not be running, or the MCP process may point at another root. Restore that path when possible.

The CLI is a fallback with a narrower promise. Take a baseline, inspect groups in the target file and its neighbours, write the bounded change, rerun analysis, and check whether the new source range joined a strong duplicate group. This catches immediately; it does not query unwritten code.

If neither live MCP nor the CLI is available, report that prevention evidence is unavailable. Do not replace repository analysis with memory.

## Workshop checkpoint

The agent proposes a payload decoder for the Workshop repository.

1. Keep the proposal outside the repository.
2. Query `find-similar` with the snippet and language.
3. Open the canonical occurrence returned by the strongest group.
4. Write a one-sentence difference statement.
5. Reuse the implementation if that statement describes only naming or location.
6. If the difference is real policy, name it in the interface or parameter rather than copying the implementation.
7. Recheck the affected file after the edit.

The finished edition will include the exact request, response, source ranges, and product capture from the pinned release.

## Agent handoff

```text
Before authoring a new code unit, call find-similar with the proposed snippet. Strong evidence blocks the copy; borderline evidence requires reading the canonical occurrence; weak or empty evidence permits authoring followed by a focused recheck.
```

## What changed

The agent did not “deduplicate” anything. It avoided creating a second owner. The repository ended the task with one behavior, one implementation, and a new caller.

## Source keys

- `deslop-for-ai`
- `deslop-agent-recipe`
- `deslop-ai-integration`
