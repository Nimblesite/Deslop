# Interactive TUI Plan

## Scope

Deferred terminal UI for operators who want to inspect results without opening
the HTML report or editor extension.

This is not on the critical path. The CLI summary and HTML report are already
usable, and the right TUI shape should come from real operator feedback.

## Product Shape

- `deslop --interactive` opens a paginated worst-offenders view.
- Navigation moves between clusters and occurrences.
- Inline previews show line/column locations and snippets derived from the
  canonical report byte ranges.
- The TUI reads from the same canonical `Report` as JSON, text, and HTML.
- Refactor actions stay out of scope until there is a real refactoring engine.

## Implementation Notes

- Use a proven terminal UI crate such as `ratatui` if this becomes active work.
- Do not fork report rendering logic; create small adapters over `Report`.
- Keep batch CLI behavior unchanged when `--interactive` is not passed.
- E2E should exercise terminal output deterministically, not through sleeps or
  timing assumptions.

## TODO

- [ ] Collect feedback from real CLI/HTML users about what they inspect first.
- [ ] Decide whether `--interactive` should analyse first or open an existing
      report via `--from-report`.
- [ ] Pick the TUI crate and terminal event model.
- [ ] Design keyboard navigation for clusters, occurrences, search, and quit.
- [ ] Render neutral `Duplicate code` cluster rows with occurrence count, mass, and engine-stamped rank; never render pair classification or evidence on the row.
- [ ] Render occurrence previews from source bytes with line numbers.
- [ ] Add deterministic E2E coverage for opening, navigation, and empty reports.
