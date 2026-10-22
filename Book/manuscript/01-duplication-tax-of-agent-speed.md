# Chapter 1 — Why coding agents duplicate code

An agent adds a validator, connects its callers, and writes passing tests. Another package already has the same validator under a different name.

The tests can prove that the new code works. They do not prove that the repository needed another implementation. The agent may have read the task's files without seeing the existing owner.

Give it a check at that decision point: **does this code already exist?**

## Check before the copy gains callers

Use `find-similar` before adding a function, helper, fixture, or other code unit. Read the returned source. Reuse an existing implementation when it fits.

A check after saving still helps, but removing a copy becomes harder once callers, tests, and policy differences accumulate around it.

| Check | Work still needed |
|---|---|
| Before writing | Inspect the match and reuse it |
| After writing | Replace the new copy and retest callers |
| After the copies diverge | Reconcile their differences before merging |

## A match starts a design decision

DRY asks where knowledge should have one authoritative representation. Deslop finds repeated source. A source match alone cannot choose the right module or dependency direction.

![DRY asks where knowledge belongs. Deslop finds repeated source for the developer to inspect.](assets/diagrams/01-dry-vs-deslop.png)

*The detection result supplies evidence. The developer chooses the design.*

For each substantial copied block, choose an outcome:

1. **Reuse:** call the implementation that already owns the behavior.
2. **Move:** put the implementation where both callers can depend on it.
3. **Delete:** remove a redundant or unused path.
4. **Generate:** maintain one source for repetition required by an external format.
5. **Retain:** keep separate copies for a specific reason, such as independent policies or a fixture that tests duplication itself.

Record why copies remain. Similar validators can represent rules that happen to agree today but must evolve independently.

![Repeated source leads to a choice to reuse, move, delete, generate, or retain it.](assets/diagrams/01-identical-code-needs-a-verdict.png)

*Even identical text needs a decision about ownership, dependencies, and behavior.*

## Try it

Choose a helper your agent is about to add. Check it with `find-similar` before saving it. Open the closest existing occurrence and state the difference in one sentence. If the difference is only its name or location, look for a way to reuse it.

**Agent rule:** check proposed code before writing it; inspect the source before deciding to merge it. See the [agent guide](https://deslop.live/docs/for-ai/) for the current workflow.
