# bug-empty-class

Fixture-per-bug seed for the [fix-bug skill](../../../../../.claude/skills/fix-bug/SKILL.md).

A file containing a single empty C# class — no methods, no fields, just
a namespace and a class declaration. Must be analysed without panic and
reported as `files_analysed: 1`. Sibling-window fingerprinting used to
trip over the empty child list; this fixture keeps the regression pinned.
The test lives in `crates/deslop/tests/cli/cache_and_debug.rs`.

Copy this directory to seed a new bug fixture. Naming rule: `bug-<kebab-
case-summary>/`. Pair every new fixture with a failing-then-passing test
under `crates/deslop/tests/cli/` that asserts the *specific* behaviour
the bug broke.
