# bug-empty-class

Fixture-per-bug seed for the [BUG-FIXTURE] workflow in `CLAUDE.md`.

A file containing a single empty C# class — no methods, no fields, just
a namespace and a class declaration. Must be analysed without panic and
reported as `files_analysed: 1`. Sibling-window fingerprinting used to
trip over the empty child list before the fix in P3; this fixture keeps
the regression pinned.

Copy this directory to seed a new bug fixture. Naming rule: `bug-<kebab-
case-summary>/`. Pair every new fixture with a failing-then-passing test
in `tests/cli.rs` that asserts the *specific* behaviour the bug broke.
