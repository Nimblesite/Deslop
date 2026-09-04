#!/usr/bin/env node
// [CORPUS-REGISTER] Folds one judging pass into a repository's clone register.
//
// This is the only way a verdict reaches a register. It is mechanical on
// purpose: a register is the independent evidence that this detector is
// accurate, and a human retyping verdicts into it is a place where a judgement
// can quietly change on the way in.
//
// What it enforces, and will not be talked out of:
//
//   * Agreement. Every judge who ruled on a candidate must have given the SAME
//     verdict, and at least MINIMUM_JUDGES must have ruled. A split is recorded
//     as NOT CLEAR, which asserts nothing — never as the majority view.
//   * The ranges come from the workspace's own pair list, not from what a judge
//     retyped. A judge whose ranges disagree with the candidate read something
//     other than the candidate, so that verdict is refused and named.
//   * Prose. `why` and `verified` must actually say something; an entry that
//     states no reason asserts nothing while looking like an assertion.
//   * Accumulation, never replacement. Passes add to a register across seeds
//     and sessions; a pair already judged keeps its first verdict, and a later
//     pass that contradicts it is reported rather than silently applied.
//
// Usage: merge-verdicts.mjs <register.json> <workspace> <verdicts.json ...>

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";

/// Judges who must have ruled before a verdict is recorded at all. One reader
/// having a firm opinion is an opinion; two arriving at it separately is
/// evidence.
const MINIMUM_JUDGES = 2;
/// The verdicts a register records, and the one it records as asserting nothing.
const SCORED = ["clearly_in", "clearly_out"];
const NOT_CLEAR = "not_clear";
const VERDICTS = [...SCORED, NOT_CLEAR];
/// A judgement stated in fewer characters than this is not a judgement. Matches
/// the floor `corpus_register_contract` holds every entry to.
const MINIMUM_PROSE = 40;
/// Field order a register is written in, so a diff shows verdicts and nothing else.
const REGISTER_FIELDS = ["name", "language", "url", "sha", "protocol"];

const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));
const key = (occurrences) => [...occurrences].sort().join(" + ");

/// Every entry of one pass, flattened to `{verdict, candidate, why, verified}`.
const entriesOf = (pass) =>
  VERDICTS.flatMap((verdict) =>
    (pass[verdict] ?? []).map((entry) => ({ verdict, ...entry })),
  );

/// Groups every judge's ruling by the candidate it ruled on.
const byCandidate = (passes) => {
  const rulings = new Map();
  for (const [judge, pass] of passes) {
    for (const entry of entriesOf(pass)) {
      const found = rulings.get(entry.candidate) ?? [];
      found.push({ judge, ...entry });
      rulings.set(entry.candidate, found);
    }
  }
  return rulings;
};

/// The one verdict every judge gave, or null when they disagreed or too few
/// ruled. Deliberately not a majority: a pair two readers see differently is
/// exactly the pair a register must not assert anything about.
const agreed = (rulings) => {
  if (rulings.length < MINIMUM_JUDGES) return null;
  const [first] = rulings;
  return rulings.every((ruling) => ruling.verdict === first.verdict) ? first.verdict : null;
};

/// The best-stated ruling among agreeing judges: the one that recorded the most
/// of what it read. Ties break on judge name, so a re-run writes the same file.
const bestStated = (rulings) =>
  [...rulings].sort(
    (left, right) =>
      (right.verified ?? "").length - (left.verified ?? "").length ||
      left.judge.localeCompare(right.judge),
  )[0];

/// Whether a ruling names the candidate it claims to. A judge who wrote ranges
/// the candidate never showed read something else, and the verdict is void.
const namesTheCandidate = (ruling, occurrences) =>
  !ruling.occurrences || key(ruling.occurrences) === key(occurrences);

const [registerPath, workspace, ...verdictPaths] = process.argv.slice(2);
if (!registerPath || !workspace || verdictPaths.length === 0) {
  throw new Error("usage: merge-verdicts.mjs <register.json> <workspace> <verdicts.json ...>");
}
if (verdictPaths.length < MINIMUM_JUDGES) {
  throw new Error(
    `${verdictPaths.length} judging pass(es) given; ${MINIMUM_JUDGES} independent passes are ` +
      `required before any verdict is recorded`,
  );
}

const pairs = new Map(
  readJson(join(workspace, "candidates", "pairs.json")).pairs.map((pair) => [
    pair.number,
    pair.occurrences,
  ]),
);
const register = existsSync(registerPath) ? readJson(registerPath) : {};
const existing = new Map(
  VERDICTS.flatMap((verdict) =>
    (register[verdict] ?? []).map((entry) => [key(entry.occurrences), verdict]),
  ),
);
const added = Object.fromEntries(VERDICTS.map((verdict) => [verdict, []]));
const refused = [];
const split = [];
const contradicted = [];

for (const [candidate, rulings] of [...byCandidate(verdictPaths.map((path) => [path, readJson(path)]))].sort(
  (left, right) => left[0] - right[0],
)) {
  const occurrences = pairs.get(candidate);
  if (!occurrences) {
    refused.push(`candidate ${candidate} is not in this workspace's pair list`);
    continue;
  }
  const misread = rulings.filter((ruling) => !namesTheCandidate(ruling, occurrences));
  for (const ruling of misread) {
    refused.push(`${ruling.judge} candidate ${candidate}: ranges are not the candidate's`);
  }
  const honest = rulings.filter((ruling) => !misread.includes(ruling));
  const verdict = agreed(honest);
  if (!verdict) {
    if (honest.length >= MINIMUM_JUDGES) {
      split.push(`candidate ${candidate}: ${honest.map((r) => r.verdict).join(" vs ")}`);
    }
    continue;
  }
  const already = existing.get(key(occurrences));
  if (already) {
    if (already !== verdict) contradicted.push(`candidate ${candidate}: ${already} -> ${verdict}`);
    continue;
  }
  const stated = bestStated(honest);
  const prose = { why: stated.why ?? "", verified: stated.verified ?? "" };
  const required = verdict === NOT_CLEAR ? ["why"] : ["why", "verified"];
  const thin = required.filter((field) => prose[field].trim().length < MINIMUM_PROSE);
  if (thin.length > 0) {
    refused.push(`candidate ${candidate}: ${thin.join(" and ")} states too little to assert`);
    continue;
  }
  added[verdict].push(
    verdict === NOT_CLEAR
      ? { why: prose.why, occurrences }
      : { why: prose.why, verified: prose.verified, occurrences },
  );
  existing.set(key(occurrences), verdict);
}

const merged = {};
for (const field of REGISTER_FIELDS) if (register[field] !== undefined) merged[field] = register[field];
for (const verdict of SCORED) merged[verdict] = [...(register[verdict] ?? []), ...added[verdict]];
if (register.clearly_out_status && merged.clearly_out.length === 0) {
  merged.clearly_out_status = register.clearly_out_status;
}
merged[NOT_CLEAR] = [...(register[NOT_CLEAR] ?? []), ...added[NOT_CLEAR]];
writeFileSync(registerPath, `${JSON.stringify(merged, null, 2)}\n`);

const report = (label, lines) => {
  if (lines.length > 0) console.log(`  ${label}:\n${lines.map((line) => `    - ${line}`).join("\n")}`);
};
console.log(`merged ${verdictPaths.length} judging pass(es) into ${registerPath}`);
for (const verdict of VERDICTS) {
  console.log(`  +${added[verdict].length} ${verdict} (register now holds ${merged[verdict].length})`);
}
report("split, recorded as nothing", split);
report("refused", refused);
report("contradicts a standing verdict, NOT applied", contradicted);
