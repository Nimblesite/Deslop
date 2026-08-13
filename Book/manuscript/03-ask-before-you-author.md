# Chapter 3 — Check for matching code before writing

Run the duplicate check before the agent adds the proposed code to a file. At that point, reusing existing code requires less work because there are no new callers or tests to move.

## What you will be able to do

Call `find-similar` before writing a function, method, class, helper, fixture, test setup, parser branch, error type, route handler, view model, or other code unit larger than a few lines. Use the response to reuse, inspect, or proceed.

## Describe behavior before names

Names are weak search keys. Two agents can call the same behavior `parseOrder`, `decodePurchase`, and `orderFromPayload`. Deslop compares the proposed source structure and content rather than depending on the name the current agent happened to choose.

Prepare one of two inputs:

- a proposed snippet plus its language, when the code does not exist yet; or
- a file path and byte range, when the draft already lives in an editor buffer or scratch file.

The first form checks code before it is written. The second can still catch a copy immediately after the edit, while the agent is still working on the same files.

## Use the score to choose the next step

The current agent guidance uses three score bands:

| Evidence | Action |
|---|---|
| Fused score at or above `0.85` | Do not write the copy. Read and reuse the canonical occurrence, or extract a shared implementation. |
| Fused score from `0.6` up to `0.85` | Read the nearest occurrence before deciding. Bias toward reuse. |
| Fused score below `0.6`, or no result | Proceed, then recheck the files that changed. |

The label modifies the response. “Same shape, different content” requires inspection even when structure is strong. “Same behavior, different code” asks you to reconcile intent before merging. The glossary gives the complete human-facing guidance.

The score tells the agent what to inspect next. It does not decide where shared code should live or whether two business rules should use one implementation.

## Reuse is broader than calling a helper

When Deslop finds an existing implementation, the new call site may be able to import and call it directly. If ownership is wrong for both locations, the correct move may be to extract a shared implementation into a neutral module. If the proposed behavior is genuinely different, make that difference explicit before authoring.

When Deslop finds a match, ask how the new caller can use the existing behavior without adding another implementation. That may mean importing the existing function, changing its interface, or moving it to a module both callers can use.

## When the live path is unavailable

First diagnose the live connection. The editor server may not be running, or the MCP process may point at another repository root. Restore the connection when possible.

The CLI can only check saved code. Record the report before editing, inspect groups in the target file and nearby files, make the small change, run Deslop again, and check whether the new source joined a strong duplicate group. This catches a copy after it is written; it cannot query unwritten code.

If neither the live agent tool nor the CLI is available, report that Deslop could not run. Do not assume the code is new because you did not happen to see a match.

## Workshop exercise

The agent proposes a payload decoder for the Workshop repository.

1. Keep the proposal outside the repository.
2. Query `find-similar` with the snippet and language.
3. Open the canonical occurrence returned by the strongest group.
4. Write a one-sentence difference statement.
5. Reuse the implementation if that statement describes only naming or location.
6. If the difference is real policy, name it in the interface or parameter rather than copying the implementation.
7. Recheck the affected file after the edit.

The finished edition will include the exact request, response, source ranges, and product capture from the pinned release.

## Instruction for coding agents

```text
Before authoring a new code unit, call find-similar with the proposed snippet. Strong evidence blocks the copy; borderline evidence requires reading the canonical occurrence; weak or empty evidence permits authoring followed by a focused recheck.
```

## Main points

The agent found an existing implementation and used it. No second copy was added.

## Source keys

- `deslop-for-ai`
- `deslop-agent-recipe`
- `deslop-ai-integration`
