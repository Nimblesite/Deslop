// [CORPUS-REGISTER-MERGE] The rules for folding judging passes into ONE
// repository's clone register.
//
// Split out of `merge-verdicts.mjs` so the driver can run them over every
// repository in a judging folder without the rules being written twice.
//
// What they enforce, and will not be talked out of:
//
//   * Agreement. Every supplied judge must have ruled on a candidate exactly
//     once with the SAME verdict. A split or incomplete pass records nothing.
//   * Clone type. A CLEARLY IN keeps a type only when every source gave the
//     same valid one, blank included; otherwise it is filed without a type.
//   * The ranges come from the workspace's own pair list, not from what a judge
//     retyped. A judge whose ranges disagree with the candidate read something
//     other than the candidate, so that verdict is refused and named.
//   * Prose. `why` and `verified` must actually say something; an entry that
//     states no reason asserts nothing while looking like an assertion.
//   * Accumulation, never replacement. Passes add to a register across seeds
//     and sessions; a pair already judged keeps its first verdict, and a later
//     pass that contradicts it is reported rather than silently applied.

/// Judges who must have ruled before a verdict is recorded at all. One reader
/// having a firm opinion is an opinion; two arriving at it separately is
/// evidence.
export const MINIMUM_JUDGES = 2;
/// The verdicts a register scores, and the one it records as asserting nothing.
export const SCORED = ["clearly_in", "clearly_out"];
export const CLEARLY_IN = SCORED[0];
export const CLEARLY_OUT = SCORED[1];
export const NOT_CLEAR = "not_clear";
export const VERDICTS = [...SCORED, NOT_CLEAR];
/// [CORPUS-REGISTER-TYPE] The clone types a CLEARLY IN may carry, as the
/// scorer's `CloneType` (`corpus_score/clone_type.rs`) spells them. A blank
/// type pins nothing: any clone kind may report the pair.
const CLONE_TYPES = ["Type I", "Type II", "Type III", "Type IV"];
const UNTYPED = "";
/// A judgement stated in fewer characters than this is not a judgement. Matches
/// the floor `corpus_register_contract` holds every SCORED entry to.
export const MINIMUM_PROSE = 40;
/// NOT CLEAR is held to a note, not to that floor. It asserts nothing, so
/// there is no assertion to state at length, and `corpus_register_contract`
/// deliberately exempts it too. Holding it to the assertion floor would throw
/// away hundreds of correctly recorded "we read this, it is not clear" results
/// — which is the whole reason NOT CLEAR is recorded at all: so that the next
/// pass does not re-litigate a pair somebody has already read.
export const MINIMUM_NOTE = 1;
/// Field order a register is written in, so a diff shows verdicts and nothing else.
export const REGISTER_FIELDS = ["name", "language", "url", "sha", "protocol"];
/// The two ways judges can differ. One of them means somebody is wrong about
/// the code; the other means one of them would not commit.
export const CONTRADICTION = "contradiction";
export const CONFIDENCE = "confidence";
/// Why a ruling was thrown out. MISCITED is a set comparison between two
/// files that must agree — the ranges `candidates/pairs.json` associates with
/// a candidate number, and the ranges the judge filed against that same
/// number. Nothing is inferred about why they differ; the report prints both.
export const MISCITED = "occurrences_mismatch";
export const INCOMPLETE = "incomplete_independent_passes";
export const THIN = "thin";
/// Where a standing verdict came from. A clash with the register is one pass
/// contradicting an earlier one; a clash with this same run is the same two
/// regions drawn as two candidates and judged differently inside one pass.
/// Reporting the second as the first would accuse a register that is empty.
const FROM_REGISTER = "register";

/// A pair's identity: its ranges, ordered, so two judges who listed the same
/// two regions in opposite orders are ruling on one pair rather than two.
export const key = (occurrences) => [...occurrences].sort().join(" + ");

/// Every entry of one pass, flattened to `{verdict, candidate, why, verified}`.
const entriesOf = (pass) =>
  VERDICTS.flatMap((verdict) =>
    (pass[verdict] ?? []).map((entry) => ({ verdict, ...entry })),
  );

/// The clone type a ruling or register entry names, blank when it names none.
const typeOf = (entry) => String(entry.type ?? UNTYPED).trim();

/// The type every source gave, or blank when any two differ — a blank and a
/// label outside the four included. Never a majority: a type two readers see
/// differently is not specified at all.
const agreedType = (votes) => {
  const [first] = votes;
  const unanimous = votes.every((vote) => vote.type === first.type);
  return unanimous && CLONE_TYPES.includes(first.type) ? first.type : UNTYPED;
};

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

/// Whether judges split over what the code IS, rather than over how sure to be.
/// One reader calling a pair an obvious clone while another calls pairing the
/// two regions plainly wrong is a claim about the source that cannot be half
/// true — a different thing from one reader declining to commit.
const splitKind = (rulings) =>
  rulings.some((ruling) => ruling.verdict === CLEARLY_IN) &&
  rulings.some((ruling) => ruling.verdict === CLEARLY_OUT)
    ? CONTRADICTION
    : CONFIDENCE;

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
  Array.isArray(ruling.occurrences) && key(ruling.occurrences) === key(occurrences);

/// What every judge said about one candidate, for the report.
const stated = (rulings) =>
  rulings.map(({ judge, verdict, why }) => ({ judge, verdict, why: why ?? "" }));

/// The prose an entry carries, and which required fields state too little.
const prosaic = (verdict, rulings) => {
  const best = bestStated(rulings);
  const prose = { why: best.why ?? "", verified: best.verified ?? "" };
  const required = verdict === NOT_CLEAR ? ["why"] : ["why", "verified"];
  const floor = verdict === NOT_CLEAR ? MINIMUM_NOTE : MINIMUM_PROSE;
  return { prose, thin: required.filter((field) => prose[field].trim().length < floor) };
};

/// The register entry for an agreed verdict. NOT CLEAR carries no `verified`
/// because it asserts nothing to verify.
const entryFor = (verdict, prose, occurrences) =>
  verdict === NOT_CLEAR
    ? { why: prose.why, occurrences }
    : { why: prose.why, verified: prose.verified, occurrences };

/// A CLEARLY IN with `type` set, or removed when blank, kept beside its prose
/// and before its ranges so a register diff shows the type where it is read.
const withType = ({ type: _dropped, occurrences, ...prose }, type) =>
  type === UNTYPED ? { ...prose, occurrences } : { ...prose, type, occurrences };

/// A candidate absent from the list shown to judges cannot name any pair.
const refuseUnknown = (candidate, rulings, out) => {
  out.refused.push({
    candidate, judge: "", kind: MISCITED, shown: [],
    filed: rulings.flatMap((ruling) => ruling.occurrences ?? []),
  });
};

/// A ruling without the displayed ranges is not evidence for this candidate.
const refuseMisread = (candidate, rulings, occurrences, out) => {
  const misread = rulings.filter((ruling) => !namesTheCandidate(ruling, occurrences));
  for (const ruling of misread) {
    out.refused.push({
      candidate,
      judge: ruling.judge,
      kind: MISCITED,
      shown: occurrences,
      filed: ruling.occurrences ?? [],
    });
  }
  return misread.length > 0;
};

/// One folder cannot stand in for another by repeating its own ruling.
const refuseIncomplete = (candidate, rulings, occurrences, expectedJudges, out) => {
  const unique = new Set(rulings.map((ruling) => ruling.judge)).size;
  if (rulings.length === expectedJudges && unique === expectedJudges) return false;
  out.refused.push({
    candidate, judge: "", kind: INCOMPLETE, shown: occurrences,
    filed: rulings.map((ruling) => ruling.judge),
  });
  return true;
};

/// Rules on one candidate, pushing to whichever of the outcome lists applies.
const rule = (candidate, rulings, pairs, standing, out, expectedJudges) => {
  const occurrences = pairs.get(candidate);
  if (!occurrences) return refuseUnknown(candidate, rulings, out);
  if (refuseMisread(candidate, rulings, occurrences, out)) return;
  if (refuseIncomplete(candidate, rulings, occurrences, expectedJudges, out)) return;
  const verdict = agreed(rulings);
  if (!verdict) {
    out.disagreements.push({ candidate, kind: splitKind(rulings), occurrences, rulings: stated(rulings) });
    return;
  }
  record(candidate, rulings, verdict, occurrences, standing, out);
};

/// Collects every source's type for an agreed CLEARLY IN, keyed by pair, so the
/// type is settled once over all of them — the standing register included.
const voteTypes = (verdict, candidate, occurrences, votes, out) => {
  if (verdict !== CLEARLY_IN) return;
  const pair = key(occurrences);
  const found = out.typeVotes.get(pair) ?? { candidate, occurrences, votes: [] };
  found.votes.push(...votes);
  out.typeVotes.set(pair, found);
};

/// Applies an agreed verdict, or reports why it cannot be applied.
const record = (candidate, honest, verdict, occurrences, standing, out) => {
  const already = standing.get(key(occurrences));
  const votes = honest.map((ruling) => ({ source: ruling.judge, type: typeOf(ruling) }));
  if (already) {
    if (already.verdict === verdict) {
      const prior = already.from === FROM_REGISTER ? [{ source: FROM_REGISTER, type: already.type }] : [];
      voteTypes(verdict, candidate, occurrences, [...prior, ...votes], out);
    } else {
      const clash = {
        candidate,
        occurrences,
        standing: already.verdict,
        from: already.from,
        proposed: verdict,
        rulings: stated(honest),
      };
      if (already.from === FROM_REGISTER) out.contradicted.push(clash);
      else out.restated.push(clash);
    }
    return;
  }
  const { prose, thin } = prosaic(verdict, honest);
  if (thin.length > 0) {
    out.refused.push({
      candidate,
      judge: "",
      kind: THIN,
      reason: `${thin.join(" and ")} states too little`,
    });
    return;
  }
  out.added[verdict].push(entryFor(verdict, prose, occurrences));
  standing.set(key(occurrences), { verdict, from: candidate });
  voteTypes(verdict, candidate, occurrences, votes, out);
};

/// Every verdict the register already holds, keyed by pair, with its type. The
/// register is read as one more judge: a standing entry the new passes
/// contradict is a disagreement, not an update, and its type is one more vote.
const standingVerdicts = (register) =>
  new Map(
    VERDICTS.flatMap((verdict) =>
      (register[verdict] ?? []).map((entry) => [
        key(entry.occurrences),
        { verdict, type: typeOf(entry), from: FROM_REGISTER },
      ]),
    ),
  );

/// The type each voted pair settles on, and the pairs some source typed but
/// whose sources did not all agree — those are filed without a type.
const settleTypes = (out) => {
  const settled = new Map();
  for (const [pair, { candidate, occurrences, votes }] of out.typeVotes) {
    const type = agreedType(votes);
    settled.set(pair, type);
    if (type === UNTYPED && votes.some((vote) => vote.type !== UNTYPED)) {
      out.untyped.push({ candidate, occurrences, votes });
    }
  }
  return settled;
};

/// Every CLEARLY IN with the type its sources settled on; pairs nobody voted
/// on this run keep the type they already had.
const typed = (entries, settled) =>
  entries.map((entry) => {
    const type = settled.get(key(entry.occurrences));
    return type === undefined ? entry : withType(entry, type);
  });

/// The register as it will be written: original fields first, then each verdict
/// list with this pass's agreed entries appended.
const rewrite = (register, added, settled) => {
  const merged = {};
  for (const field of REGISTER_FIELDS) {
    if (register[field] !== undefined) merged[field] = register[field];
  }
  for (const verdict of SCORED) merged[verdict] = [...(register[verdict] ?? []), ...added[verdict]];
  merged[CLEARLY_IN] = typed(merged[CLEARLY_IN], settled);
  if (register.clearly_out_status && merged.clearly_out.length === 0) {
    merged.clearly_out_status = register.clearly_out_status;
  }
  merged[NOT_CLEAR] = [...(register[NOT_CLEAR] ?? []), ...added[NOT_CLEAR]];
  return merged;
};

/// Folds `passes` into `register`, returning the rewritten register and
/// everything that could not be merged. Nothing is written here: the caller
/// decides whether this run applies its result.
export const mergePass = ({ register, pairs, passes, expectedJudges = passes.length }) => {
  const out = {
    added: Object.fromEntries(VERDICTS.map((verdict) => [verdict, []])),
    refused: [],
    disagreements: [],
    contradicted: [],
    restated: [],
    typeVotes: new Map(),
    untyped: [],
  };
  const standing = standingVerdicts(register);
  const ordered = [...byCandidate(passes)].sort((left, right) => left[0] - right[0]);
  for (const [candidate, rulings] of ordered) rule(candidate, rulings, pairs, standing, out, expectedJudges);
  const disputed = new Set(out.restated.map((entry) => key(entry.occurrences)));
  for (const verdict of VERDICTS) {
    out.added[verdict] = out.added[verdict].filter((entry) => !disputed.has(key(entry.occurrences)));
  }
  const settled = settleTypes(out);
  return { ...out, merged: rewrite(register, out.added, settled), judges: passes.length };
};
