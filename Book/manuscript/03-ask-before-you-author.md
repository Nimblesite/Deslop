# Chapter 3 — Check for matching code before writing

Prepare the code you intend to add, then send it to `find-similar` with `snippet` and its registered `language`. Keep it outside the repository until you have read the result.

For code already in a file, pass `path`, `start_byte`, and `end_byte` instead. Byte ranges use an excluded end: `[start_byte, end_byte)`.

## Read both result lists

- **`clusters`:** related code already grouped by Deslop.
- **`existing`:** places where the proposed code occurs once, outside any duplicate group.

Empty `clusters` does not mean the code is new. A single existing implementation is exactly what you need to find before creating its second copy.

| Finding | Next step |
|---|---|
| Identical or nearly identical code | Read it and reuse the existing implementation where appropriate |
| Similar code or same behavior | Read both implementations and identify meaningful differences |
| Same shape, different content | Inspect it; shape alone does not establish a clone |
| Both lists empty | Proceed, then check the changed file |

Current clusters carry membership, kind, mass, and rank. Use `compare-pair` for evidence about two specific occurrences. Do not treat a score as permission to refactor. See the [agent guide](https://deslop.live/docs/for-ai/).

## Reuse the owner

Import or call the existing implementation when its interface fits. If both callers need a neutral owner, move the implementation there. Keep independent policy separate when a shared helper would blur the distinction.

After editing, request `duplicates` with the changed `path`. Check the actual source as well as the report.

**Try it:** check one proposed helper, inspect both result lists, and explain any real difference before writing it. A failed query is a setup problem, not evidence that the code is unique.
