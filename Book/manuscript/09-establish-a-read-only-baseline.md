# Chapter 9 — Measure existing duplication without changing code

Record the current Deslop report and test result before cleanup. Keep the first scan separate from a refactor.

## Save a baseline

Record the repository revision, Deslop executable and version, configuration, and embedding policy. Run the repository's normal static checks and tests. Save their results, including failures that already exist.

For an audit that must leave the checkout untouched, write reports outside it and disable cache access. Here `$DESLOP` names the absolute path to the installed CLI; replace both example paths:

```sh
"$DESLOP" /path/to/repository \
  --output /path/to/audit/report \
  --no-incremental --no-fail-over \
  --log-to-console --log-level warn --no-color
```

`--output` is a prefix: the CLI appends `.json`, `.txt`, and `.html`. `--no-fail-over` clears the duplication ceiling for this discovery run; it does not disable other errors. Keep the normal ceiling for CI.

For routine repeat scans where cache writes are allowed, retain the default cache instead. See the [CLI reference](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/cli.md).

## Choose the work from evidence

Read the JSON with a JSON parser. Use the HTML report for human inspection. Record the included kinds, analysed lines, duplicated lines, percentage, and selected group IDs. Retain the generated reports rather than retyping their numbers into a new report.

Choose one group to inspect next. A large group is a starting point, not approval to merge all its members.

This measure-first sequence follows Kevin Moore's [Deslop Duplication Audit Protocol](https://github.com/kevmoo/kevmoo_skills/blob/main/skills/deslop-duplication-audit/SKILL.md). Use your repository's own test and toolchain rules.

**Try it:** save a baseline and identify one candidate group without editing source or configuration.
