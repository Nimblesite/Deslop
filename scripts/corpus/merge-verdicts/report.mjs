// [CORPUS-REGISTER-MERGE] Renders the disagreement report.
//
// Every string this file emits is either a column header, a label derived from
// the verdicts themselves, or a value read out of a judging folder. There is no
// commentary: a reader must be able to reproduce the document by re-running the
// script, and prose written here could not be reproduced or diffed.

import { MISCITED, VERDICTS } from "./pass.mjs";

/// Labels for the disagreements that are not simply two verdicts differing.
/// Each names the comparison that produced the row, never a diagnosis of it.
const MISCITED_KIND = MISCITED;
const REGISTER_KIND = "register_conflict";
const DUPLICATE_KIND = "duplicate_pair";
/// Separates the two verdicts of a split, and the parts of a multi-value cell.
/// Not a pipe: a pipe is the markdown column separator, and a label carrying
/// one silently splits the cell it is in.
const VERSUS = "/";
const WITHIN_CELL = "<br>";

/// Markdown cell text: a table breaks on an unescaped pipe or a line feed, and
/// judges write both.
const cell = (text) => String(text ?? "").replaceAll("|", "\\|").replaceAll(/\s*\n\s*/g, " ");

const ranges = (occurrences) => occurrences.map((range) => `\`${range}\``).join(WITHIN_CELL);
const pairs = (entries) => entries.join(WITHIN_CELL);

/// The kind of a split, read straight off the verdicts given: the distinct
/// verdicts, ordered, joined. Nothing is inferred.
const splitKindOf = (rulings) =>
  [...new Set(rulings.map((ruling) => ruling.verdict))].sort().join(VERSUS);

/// Every disagreement in one repository, as uniform rows.
const rowsFor = (repo) => [
  ...repo.result.disagreements.map((split) => ({
    repository: repo.name,
    candidate: String(split.candidate),
    kind: splitKindOf(split.rulings),
    occurrences: split.occurrences,
    verdicts: split.rulings.map((ruling) => `${ruling.judge}=${ruling.verdict}`),
    reasons: split.rulings.map((ruling) => `${ruling.judge}: ${cell(ruling.why)}`),
  })),
  ...repo.result.refused
    .filter((entry) => entry.kind === MISCITED)
    .map((entry) => ({
      repository: repo.name,
      candidate: String(entry.candidate),
      kind: MISCITED_KIND,
      occurrences: entry.shown,
      verdicts: [`${entry.judge} filed=${entry.filed.join(", ")}`],
      reasons: [],
    })),
  ...repo.result.contradicted.map((entry) => ({
    repository: repo.name,
    candidate: String(entry.candidate),
    kind: REGISTER_KIND,
    occurrences: entry.occurrences,
    verdicts: [
      `register=${entry.standing}`,
      ...entry.rulings.map((ruling) => `${ruling.judge}=${entry.proposed}`),
    ],
    reasons: entry.rulings.map((ruling) => `${ruling.judge}: ${cell(ruling.why)}`),
  })),
  ...repo.result.restated.map((entry) => ({
    repository: repo.name,
    candidate: `${entry.from}${VERSUS}${entry.candidate}`,
    kind: DUPLICATE_KIND,
    occurrences: entry.occurrences,
    verdicts: [`candidate ${entry.from}=${entry.standing}`, `candidate ${entry.candidate}=${entry.proposed}`],
    reasons: entry.rulings.map((ruling) => `${ruling.judge}: ${cell(ruling.why)}`),
  })),
];

/// Every disagreement across every repository, in a fixed order so two runs
/// over the same verdicts produce the same document.
const allRows = (repos) =>
  repos
    .flatMap(rowsFor)
    .sort(
      (left, right) =>
        left.kind.localeCompare(right.kind) ||
        left.repository.localeCompare(right.repository) ||
        left.candidate.localeCompare(right.candidate, undefined, { numeric: true }),
    );

/// A markdown table of rows already reduced to strings.
const table = (headers, rows) =>
  rows.length === 0
    ? "(none)\n"
    : [headers, headers.map(() => "---"), ...rows]
        .map((cells) => `| ${cells.join(" | ")} |`)
        .join("\n") + "\n";

/// How many disagreements a set of repositories holds in total.
export const disagreementCount = (repos) => allRows(repos).length;

/// How many pairs the merge imported, counted off the entries it wrote.
const importedCount = (repos) =>
  repos.reduce(
    (total, repo) =>
      total + VERDICTS.reduce((sum, verdict) => sum + repo.result.added[verdict].length, 0),
    0,
  );

/// Counts per kind, and per repository, straight off the rows.
const tally = (rows, field) => {
  const counted = new Map();
  for (const row of rows) counted.set(row[field], (counted.get(row[field]) ?? 0) + 1);
  return [...counted].sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]));
};

/// The whole report.
export const render = ({ sources, repos }) => {
  const rows = allRows(repos);
  const judges = sources.map((source) => source.split("/").filter(Boolean).pop());
  return `# Verdict disagreements

| field | value |
| --- | --- |
| generator | \`scripts/corpus/merge-verdicts.mjs\` |
| spec | \`docs/specs/corpus.md\` [CORPUS-REGISTER-MERGE] |
| judges | ${[...judges, "register"].join(", ")} |
| repositories | ${repos.length} |
| pairs_imported | ${importedCount(repos)} |
| pairs_left_out | ${rows.length} |

## by_kind

${table(["kind", "pairs"], tally(rows, "kind").map(([kind, count]) => [kind, String(count)]))}
## by_repository

${table(
  ["repository", "pairs"],
  tally(rows, "repository").map(([repository, count]) => [repository, String(count)]),
)}
## disagreements

${table(
  ["kind", "repository", "candidate", "candidate_occurrences", "by_source", "stated_reasons"],
  rows.map((row) => [
    row.kind,
    row.repository,
    row.candidate,
    ranges(row.occurrences),
    pairs(row.verdicts),
    pairs(row.reasons),
  ]),
)}`;
};
