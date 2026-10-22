# Chapter 11 — Merge one duplicate group safely

Start from the recorded test result and the decision to merge one group. If important behavior is untested, add an assertion before changing its implementation.

## Choose the owner

Place shared code in the module responsible for the behavior. Both callers must be allowed to depend on it. A general `utils` file is not automatically the right home.

Move the implementation and update its callers. Remove the old copies rather than adding another path beside them.

## Preserve meaningful differences

Use named inputs when they describe variations of one operation. Keep independent policies separate. Avoid a helper whose flags merely conceal two unrelated implementations.

Preserve required public interfaces, error behavior, side effects, and performance constraints. A smaller diff or lower duplication percentage cannot establish those properties.

`merge-plan` can provide a read-only plan for a supported transformation. Read its verdict and proposed edit. An unsupported or `ai_or_human` verdict is not an executable merge. See the [agent guide](https://deslop.live/docs/for-ai/).

## Keep the change focused

Change the shared implementation, its callers, and the tests needed to protect the behavior. Leave unrelated formatting and design changes for another task.

**Try it:** consolidate one accepted group, preserving assertions for every meaningful difference. Run the same checks recorded before the edit.
