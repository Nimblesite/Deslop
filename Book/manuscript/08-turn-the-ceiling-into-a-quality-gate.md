# Chapter 8 — Set a duplication limit in CI

Measure the repository before choosing a limit. Commit the limit in `.deslop.toml` so local scans and CI share it.

```toml
[threshold]
max_duplication_percent = 5.5
```

This is an example ceiling, not a recommended target. Choose a value supported by your baseline. The CLI writes enabled reports and exits `3` when the measured percentage exceeds the ceiling. See the [configuration reference](https://deslop.live/docs/configuration/).

![The actual Deslop Duplication view shows the repository percentage and folder breakdown.](assets/screenshots/08-report-overview.png)

*The Duplication view summarises this captured repository state. Folder percentages use their own folder totals.*

## Keep the measurement comparable

Use the same build, root, analysis settings, and embedding policy for the baseline and check. Commit `[total] include` when changing which clone kinds count; otherwise its default is Identical and Nearly identical code.

Exclusions remove files from analysis. `report_hide` retains analysis evidence but removes hidden occurrences from duplication metrics. Shape-only findings never count. Do not change any of these to conceal a failed cleanup.

## Lower the ceiling after verified cleanup

When tests pass and Deslop confirms the intended copies were removed, lower the ceiling to the accepted measured result. Do not round a displayed percentage down into a ceiling the underlying value already exceeds.

**Try it:** in a disposable repository, set a ceiling the baseline meets. Add a known copy, observe the failing exit and saved report, then remove it and check again.
