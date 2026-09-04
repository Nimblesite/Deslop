// [CORPUS-REGISTER-MERGE] The disagreement report.
//
// This report has one job: list the pairs the judges do not agree on. When two
// readers rule differently on the same lines of somebody else's source, at
// least one of them is wrong, and a register built by picking a side would
// carry that error as ground truth forever. So the merge stops, nothing is
// written, and every disagreement is printed here with each judge's own words.
//
// Nothing else goes in this file. Counts of what WOULD have merged, cost
// figures and queue movements are not disagreements and would bury them.

import { CONFIDENCE, CONTRADICTION, MISCITED } from "./pass.mjs";

/// Markdown cell text: a table breaks on an unescaped pipe or a line feed, and
/// judges write both.
const cell = (text) => String(text ?? "").replaceAll("|", "\\|").replaceAll(/\s*\n\s*/g, " ");

/// The ranges of a pair, as one cell.
const ranges = (occurrences) => occurrences.map((range) => `\`${range}\``).join("<br>");

/// Every judge's verdict on one pair, as one cell.
const verdicts = (rulings) => rulings.map((r) => `${r.judge}: **${r.verdict}**`).join("<br>");

/// A markdown table, or a line saying the section found nothing.
const table = (headers, rows, empty) => {
  if (rows.length === 0) return `${empty}\n`;
  const line = (cells) => `| ${cells.join(" | ")} |`;
  return [line(headers), line(headers.map(() => "---")), ...rows.map(line)].join("\n") + "\n";
};

/// Splits of one kind, across every repository.
const splits = (repos, kind) =>
  repos.flatMap((repo) =>
    repo.result.disagreements
      .filter((split) => split.kind === kind)
      .map((split) => ({ repo: repo.name, ...split })),
  );

/// One opposite-conclusion pair, in full. These are the ones worth reading:
/// two judges have made incompatible claims about the same lines, so the
/// report prints the ranges and what each of them actually said.
const opposed = (split) =>
  [
    `### ${split.repo} — candidate ${split.candidate}`,
    "",
    ...split.occurrences.map((range) => `- \`${range}\``),
    "",
    ...split.rulings.map((r) => `**${r.judge} — ${r.verdict}**\n\n> ${cell(r.why)}\n`),
  ].join("\n");

/// Rows where the judges contradict a verdict the register already holds.
const standingRows = (repos) =>
  repos.flatMap((repo) =>
    repo.result.contradicted.map((entry) => [
      repo.name,
      entry.candidate,
      ranges(entry.occurrences),
      `register: **${entry.standing}**<br>${entry.rulings.map((r) => `${r.judge}: **${r.proposed ?? entry.proposed}**`).join("<br>")}`,
    ]),
  );

/// Rows where one pair was drawn as two candidates and judged differently in
/// one pass — the judges contradicting themselves across two numbers.
const restatedRows = (repos) =>
  repos.flatMap((repo) =>
    repo.result.restated.map((entry) => [
      repo.name,
      `${entry.from} then ${entry.candidate}`,
      ranges(entry.occurrences),
      `**${entry.standing}** then **${entry.proposed}**`,
    ]),
  );

/// Rows where a judge filed ranges the candidate never showed them.
const miscitedRows = (repos) =>
  repos.flatMap((repo) =>
    repo.result.refused
      .filter((entry) => entry.kind === MISCITED)
      .map((entry) => [repo.name, entry.candidate, entry.judge, cell(entry.reason)]),
  );

/// How many disagreements a set of repositories holds in total.
export const disagreementCount = (repos) =>
  repos.reduce(
    (total, repo) =>
      total +
      repo.result.disagreements.length +
      repo.result.contradicted.length +
      repo.result.restated.length +
      repo.result.refused.filter((entry) => entry.kind === MISCITED).length,
    0,
  );

/// The whole report.
export const render = ({ sources, repos }) => {
  const opposite = splits(repos, CONTRADICTION);
  const unsure = splits(repos, CONFIDENCE);
  const total = disagreementCount(repos);
  return `# Verdicts that disagree

**${total} pairs. Nothing was merged.**

Judges read: ${sources.map((source) => `\`${source}\``).join(", ")}, plus this repository's existing registers. A pair enters a register only when every judge who ruled on it said the same thing, so every pair below was left out. Written by \`scripts/corpus/merge-verdicts.mjs\`; spec \`docs/specs/corpus.md\` [CORPUS-REGISTER-MERGE].

## Opposite conclusions — one judge CLEARLY IN, the other CLEARLY OUT

**${opposite.length}.** These cannot both be true of the same lines. Someone is wrong about the source.

${opposite.length === 0 ? "None.\n" : opposite.map(opposed).join("\n")}
## A judge's ranges are not the candidate's

**${miscitedRows(repos).length}.** The judge filed a verdict on lines the candidate never showed them, so it is a ruling on something else.

${table(["repository", "candidate", "judge", "what happened"], miscitedRows(repos), "None.")}
## Contradicts a verdict the register already holds

**${standingRows(repos).length}.** An earlier pass and this one read the same lines differently.

${table(["repository", "candidate", "ranges", "verdicts"], standingRows(repos), "None.")}
## The same pair judged twice, differently, in one pass

**${restatedRows(repos).length}.** The draw showed one pair of regions under two numbers, and the judges answered them differently.

${table(["repository", "candidates", "ranges", "verdicts"], restatedRows(repos), "None.")}
## One judge committed, the other would not

**${unsure.length}.** A firm verdict against NOT CLEAR. Weaker than an opposite conclusion, but still not agreement.

${table(
  ["repository", "candidate", "ranges", "verdicts"],
  unsure.map((split) => [split.repo, split.candidate, ranges(split.occurrences), verdicts(split.rulings)]),
  "None.",
)}`;
};
