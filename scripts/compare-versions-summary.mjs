#!/usr/bin/env node
// [COMPARE-VERSIONS-SUMMARY] Renders the comparison reports for
// scripts/compare-versions.sh. Every figure here is lifted verbatim from the
// engine's own report JSON, or from the scorecard `corpus-score` already
// computed ([CORPUS-SCORE]); this renderer derives nothing except which
// published cluster ids the two engines share.
//
// Usage: node scripts/compare-versions-summary.mjs <meta.json>

import { readFileSync, writeFileSync } from "node:fs";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const TOP_CLUSTER_ROWS = 5;
const SHORT_SHA_LENGTH = 12;
const BINARY_SHA_PREVIEW = 16;
const ABSENT = "n/a";
// [CORPUS-SCORE] The scorecard `corpus-score` writes beside the reports. Every
// register figure in this document is read from it; none is recomputed here,
// so the scorecard and these summaries can never disagree.
const SCORECARD = "score.json";
const NO_REGISTER = "no register for this target";

const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));

// Provenance is the point of this document: a figure whose producing binary is
// unidentified is not comparable, so a missing fingerprint fails the render
// rather than printing a hole.
const readTiming = (path) => {
  const timing = readJson(path);
  if (!timing.binary_sha256) {
    throw new Error(`${path} records no binary_sha256 — cannot stamp provenance`);
  }
  return timing;
};

// [COMPARE-VERSIONS-SUMMARY] Every stat is a field the engine published, or a
// fingerprint of the binary that published it. Nothing is recomputed here.
const statsOf = (report, timing) => [
  ["binary_sha256", timing.binary_sha256],
  ["tool_version", report.tool_version],
  ["files_analysed", report.files_analysed],
  ["analysed_loc", report.metrics.analysed_loc],
  ["duplicated_loc", report.metrics.duplicated_loc],
  ["duplication_percent (engine)", report.metrics.duplication_percent],
  ["clusters_total", report.metrics.clusters_total],
  ["clusters_hidden", report.clusters_hidden ?? ABSENT],
  ["analysis_wall_ms (cold, no cache)", timing.elapsed_ms],
];

// Only fields present in BOTH reports are comparable across versions; anything
// one schema dropped or gained is listed separately rather than compared.
const comparableRows = (statsA, statsB) =>
  statsA.map(([label, a], i) => [label, a, statsB[i][1]]);
const bothPresent = ([, a, b]) => a !== undefined && b !== undefined;
const eitherAbsent = (row) => !bothPresent(row);

const CLUSTER_DISPLAY = [
  ["rank", (c) => c.rank],
  ["id", (c) => c.id],
  ["band", (c) => c.rank_band],
  ["mass", (c) => c.mass],
  ["weight", (c) => c.weight],
  ["size", (c) => c.size],
  ["bucket", (c) => c.bucket],
  ["category", (c) => c.category],
  ["nodes", (c) => c.canonical_node_count],
  ["occurrences", (c) => c.occurrence_count],
  ["occurrences_total", (c) => c.occurrences_total],
  ["fused", (c) => (c.signals ? c.signals.fused : undefined)],
];

const clustersOf = (report) => report.clusters ?? [];
// A column is shown only when this report's own schema populates it, so no
// cell is ever an approximation of some other schema's field.
const columnsFor = (report) =>
  CLUSTER_DISPLAY.filter(([, get]) => clustersOf(report).some((c) => get(c) !== undefined));

const row = (cells) => `| ${cells.join(" | ")} |`;
const divider = (count) => `|${Array.from({ length: count }, () => "---").join("|")}|`;

const clusterTable = (report) => {
  const columns = columnsFor(report);
  return [
    row(columns.map(([label]) => label)),
    divider(columns.length),
    ...clustersOf(report)
      .slice(0, TOP_CLUSTER_ROWS)
      .map((cluster) => row(columns.map(([, get]) => get(cluster)))),
  ];
};

const idsOf = (report) => new Set(clustersOf(report).map((cluster) => cluster.id));

const overlapOf = (reportA, reportB) => {
  const idsA = idsOf(reportA);
  const idsB = idsOf(reportB);
  const shared = [...idsA].filter((id) => idsB.has(id)).length;
  return { shared, onlyA: idsA.size - shared, onlyB: idsB.size - shared };
};

const topLevelOnlyIn = (report, other) => Object.keys(report).filter((key) => !(key in other));
const clusterKeys = (report) => [...new Set(clustersOf(report).flatMap((c) => Object.keys(c)))];
const clusterOnlyIn = (report, other) => {
  const otherKeys = clusterKeys(other);
  return clusterKeys(report).filter((key) => !otherKeys.includes(key));
};
const listOrNone = (values) => (values.length ? values.join(", ") : "none");
const schemaLine = (label, report, other) =>
  `- ${label} — top level: ${listOrNone(topLevelOnlyIn(report, other))}; per cluster: ${listOrNone(clusterOnlyIn(report, other))}`;

// [COMPARE-VERSIONS-SUMMARY] The provenance stamp. Both deslop commit ids in
// full, the target repository's exact commit in full, and the sha256 of each
// binary that produced the numbers below it. Without all four a figure in this
// document cannot be traced back to what produced it.
const provenance = (meta, target, sides) => [
  "## Provenance",
  "",
  row(["", `deslop A`, `deslop B`]),
  divider(3),
  row(["deslop commit", `\`${meta.deslop.a.sha}\``, `\`${meta.deslop.b.sha}\``]),
  row(["commit subject", meta.deslop.a.subject, meta.deslop.b.subject]),
  row(["tool_version", sides.a.report.tool_version, sides.b.report.tool_version]),
  row([
    "binary sha256",
    `\`${sides.a.timing.binary_sha256}\``,
    `\`${sides.b.timing.binary_sha256}\``,
  ]),
  "",
  row(["target repository", ""]),
  divider(2),
  row(["url", target.url]),
  row(["commit", `\`${target.sha}\``]),
  row(["language", target.language]),
  "",
  `- Analysis config: \`${meta.flags}\``,
  `- Each cycle: clean build artifacts → delete deslop cache → fresh \`git archive\` extract → clean release rebuild → scan`,
  `- Both cycles scanned this identical checkout; only the engine differed.`,
];

// [CORPUS-SCORE] One target's scored standing, or undefined when no register
// is judged for it yet.
const targetScore = (scorecard, target) =>
  scorecard.targets.find((scored) => scored.name === target.slug);

// [CORPUS-SCORE] One scored line per judged entry, for one engine.
const registerLines = (scored, engineId) =>
  scored.scores[engineId].entries.map((entry) =>
    row([
      entry.verdict === "clearly_in" ? "CLEARLY IN" : "CLEARLY OUT",
      `\`${entry.occurrences.join("\` + \`")}\``,
      entry.correct
        ? "correct"
        : entry.verdict === "clearly_in"
          ? "**FALSE NEGATIVE**"
          : "**FALSE POSITIVE**",
    ]),
  );

// [CORPUS-SCORE] The headline for one engine, from the scorer's own counts.
const registerHeadline = (score, shortSha) =>
  `deslop@${shortSha}: ${score.clearly_in_found}/${score.clearly_in_total} CLEARLY IN found · ` +
  `${score.clearly_out_absent}/${score.clearly_out_total} CLEARLY OUT correctly absent · ` +
  `score ${score.score_percent === null ? "not judged" : `${score.score_percent.toFixed(1)}%`}`;

// [CORPUS-SCORE] The only evidence of an accuracy change between two engines,
// as the scorer decided it. Cluster totals and percentages are description,
// not verdict, and are deliberately absent from this judgement.
const degradationVerdict = ({ degraded, new_false_negatives, new_false_positives }) => {
  if (!degraded) {
    return "- Register verdict: **no degradation** — B introduces no false negative and no false positive against the judged pairs.";
  }
  return [
    `- Register verdict: **DEGRADED** — ${new_false_negatives.length} new false negative(s), ${new_false_positives.length} new false positive(s).`,
    ...new_false_negatives.map((entry) => `  - new false negative: ${entry}`),
    ...new_false_positives.map((entry) => `  - new false positive: ${entry}`),
  ].join("\n");
};

// [CORPUS-SCORE] The register section of one target summary.
const registerSection = (scorecard, target, shortA, shortB) => {
  const scored = targetScore(scorecard, target);
  if (!scored) {
    return ["## Clone register", "", `Not scored: ${NO_REGISTER}.`, ""];
  }
  const { standing_false_negatives, standing_false_positives } = scored.degradation;
  return [
    "## Clone register",
    "",
    `Judged independently of this codebase at \`${scored.sha}\`; protocol in \`.agents/skills/judge-clone-pairs\`.`,
    "",
    `- ${registerHeadline(scored.scores[shortA], shortA)}`,
    `- ${registerHeadline(scored.scores[shortB], shortB)}`,
    degradationVerdict(scored.degradation),
    ...(standing_false_negatives + standing_false_positives
      ? [
          `- Standing defects both engines share: ${standing_false_negatives} false negative(s), ${standing_false_positives} false positive(s) — real, but not slippage.`,
        ]
      : []),
    "",
    row([`entry — deslop@${shortB}`, "occurrences", "outcome"]),
    divider(3),
    ...registerLines(scored, shortB),
    "",
  ];
};

const targetSummary = (meta, scorecard, target) => {
  const sides = {
    a: { report: readJson(target.reports.a), timing: readTiming(target.timings.a) },
    b: { report: readJson(target.reports.b), timing: readTiming(target.timings.b) },
  };
  const rows = comparableRows(
    statsOf(sides.a.report, sides.a.timing),
    statsOf(sides.b.report, sides.b.timing),
  );
  const struck = rows.filter(eitherAbsent).map(([label]) => label);
  const overlap = overlapOf(sides.a.report, sides.b.report);
  const identical =
    readFileSync(target.reports.a, "utf8") === readFileSync(target.reports.b, "utf8");
  const shortA = meta.deslop.a.sha.slice(0, SHORT_SHA_LENGTH);
  const shortB = meta.deslop.b.sha.slice(0, SHORT_SHA_LENGTH);
  return [
    `# ${target.slug} — deslop \`${shortA}\` vs \`${shortB}\``,
    "",
    ...provenance(meta, target, sides),
    "",
    "## Metrics from both versions",
    "",
    row(["metric", `deslop@${shortA}`, `deslop@${shortB}`]),
    divider(3),
    ...rows.filter(bothPresent).map((cells) => row(cells)),
    ...(struck.length ? ["", `Not comparable (absent in one schema): ${struck.join(", ")}`] : []),
    "",
    `- Published clusters shared by id: **${overlap.shared}** · only in \`${shortA}\`: **${overlap.onlyA}** · only in \`${shortB}\`: **${overlap.onlyB}**`,
    `- Canonical JSON reports byte-identical: **${identical ? "yes" : "no"}**`,
    "",
    ...registerSection(scorecard, target, shortA, shortB),
    `## Top ${TOP_CLUSTER_ROWS} clusters — deslop@${shortA}`,
    "",
    ...clusterTable(sides.a.report),
    "",
    `## Top ${TOP_CLUSTER_ROWS} clusters — deslop@${shortB}`,
    "",
    ...clusterTable(sides.b.report),
    "",
    "## Report schema changes",
    "",
    schemaLine(`Only in ${shortA}`, sides.a.report, sides.b.report),
    schemaLine(`Only in ${shortB}`, sides.b.report, sides.a.report),
    "",
    `Reports: \`${target.reports.a}\` and \`${target.reports.b}\` (each with .txt/.html siblings and logs).`,
    "",
    "Timing caveat: the cycles run sequentially, so a later scan benefits from warm OS caches — treat small wall-time deltas as noise.",
    "",
  ].join("\n");
};

// [CORPUS-SCORE] The index cell: found/total for each verdict on engine B, its
// score, and whether B degraded. Absent when the target has no register.
const indexRegisterCell = (scorecard, target, shortB) => {
  const scored = targetScore(scorecard, target);
  if (!scored) return ABSENT;
  const score = scored.scores[shortB];
  return (
    `IN ${score.clearly_in_found}/${score.clearly_in_total} · ` +
    `OUT ${score.clearly_out_absent}/${score.clearly_out_total} · ` +
    `${score.score_percent === null ? ABSENT : `${score.score_percent.toFixed(1)}%`} · ` +
    (scored.degradation.degraded ? "**DEGRADED**" : "no degradation")
  );
};

const indexRow = (scorecard, shortB, target) => {
  const reportA = readJson(target.reports.a);
  const reportB = readJson(target.reports.b);
  const overlap = overlapOf(reportA, reportB);
  return row([
    target.slug,
    target.language,
    `\`${target.sha.slice(0, SHORT_SHA_LENGTH)}\``,
    reportA.files_analysed,
    reportA.metrics.analysed_loc,
    `${reportA.metrics.clusters_total} → ${reportB.metrics.clusters_total}`,
    `${reportA.metrics.duplication_percent} → ${reportB.metrics.duplication_percent}`,
    `${overlap.shared} / +${overlap.onlyB} / −${overlap.onlyA}`,
    indexRegisterCell(scorecard, target, shortB),
  ]);
};

const indexDocument = (meta, scorecard) => {
  const shortA = meta.deslop.a.sha.slice(0, SHORT_SHA_LENGTH);
  const shortB = meta.deslop.b.sha.slice(0, SHORT_SHA_LENGTH);
  return [
    "# Deslop version comparison — all targets",
    "",
    `- deslop A: \`${meta.deslop.a.sha}\` — ${meta.deslop.a.subject}`,
    `- deslop B: \`${meta.deslop.b.sha}\` — ${meta.deslop.b.subject}`,
    `- Analysis config: \`${meta.flags}\``,
    `- Generated: ${meta.generated_at}`,
    "",
    "Cluster and percentage columns are description. The register column is the only",
    "evidence of an accuracy change: a new false negative or a new false positive is a bug,",
    "and nothing else here is. Protocol: `.agents/skills/judge-clone-pairs`.",
    "",
    row([
      "target",
      "language",
      "target commit",
      "files",
      "analysed LOC",
      `clusters ${shortA}→${shortB}`,
      `dup% ${shortA}→${shortB}`,
      "clusters shared / gained / lost",
    "register (B)",
    ]),
    divider(9),
    ...meta.targets.map((target) => indexRow(scorecard, shortB, target)),
    "",
    ...meta.targets.map((t) => `- [${t.slug}](${t.slug}/SUMMARY.md) — \`${t.url}\` @ \`${t.sha}\``),
    "",
  ].join("\n");
};

const [metaPath] = process.argv.slice(2);
if (!metaPath) {
  process.stderr.write("usage: compare-versions-summary.mjs <meta.json>\n");
  process.exit(2);
}
const meta = readJson(metaPath);
// [CORPUS-SCORE] `corpus-score` runs first and writes this beside the reports.
// Without it there is no accuracy verdict to render, so a missing scorecard
// fails the render rather than quietly dropping the only column that matters.
const scorecardPath = join(dirname(resolve(metaPath)), SCORECARD);
if (!existsSync(scorecardPath)) {
  throw new Error(`${scorecardPath} is missing — run \`corpus-score score\` before rendering`);
}
const scorecard = readJson(scorecardPath);

for (const target of meta.targets) {
  writeFileSync(target.summary_path, targetSummary(meta, scorecard, target));
}

// [CORPUS-SCORE] Echo the only verdict that is evidence, so a comparison run
// states it in the terminal rather than only in a file nobody opens.
const shortB = meta.deslop.b.sha.slice(0, SHORT_SHA_LENGTH);
for (const target of meta.targets) {
  const scored = targetScore(scorecard, target);
  if (!scored) {
    console.log(`${target.slug}: no clone register — accuracy change not judged`);
    continue;
  }
  const score = scored.scores[shortB];
  const { degraded, standing_false_negatives, standing_false_positives } = scored.degradation;
  console.log(
    `${target.slug}: register ${degraded ? "DEGRADED" : "no degradation"} — ` +
      `CLEARLY IN ${score.clearly_in_found}/${score.clearly_in_total}, ` +
      `CLEARLY OUT ${score.clearly_out_absent}/${score.clearly_out_total} on B` +
      (standing_false_negatives + standing_false_positives
        ? ` (standing defects in both: ${standing_false_negatives} FN, ${standing_false_positives} FP)`
        : ""),
  );
}

const index = indexDocument(meta, scorecard);
writeFileSync(meta.index_path, index);
process.stdout.write(index);
