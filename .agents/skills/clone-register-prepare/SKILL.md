---
name: clone-register-prepare
description: Build the blinded folder a clone judge works through — the repositories, the reports and the judging skill — then file and score the verdicts that come back, into the CLEARLY IN / CLEARLY OUT register that decides whether a Deslop change improved or degraded accuracy. Never judges. Use when comparing two Deslop versions, or when the user says "register", "prepare a judging pass", "did accuracy slip", "add a corpus assertion".
argument-hint: "[prepare [repo ...] | file <folder> | score]"
---

# The Clone Register — Preparer

Deslop's own reports cannot tell you whether Deslop got better. Cluster counts move,
percentages move, ranks move — none of it is evidence. The register is the outside
opinion that turns those movements into a verdict.

A register is a **per-repository, commit-pinned list of code pairs, each carrying a
verdict reached by a judge who never had Deslop in front of them**.

## What you produce — three things, in one folder

You are the **preparer**. You may read everything. Your entire output is one folder,
built outside this repository, holding exactly:

1. **The repositories** — each checked out at its pinned commit, source only.
2. **The reports** — two pair lists per repository, stripped to `groups`/`regions` and
   labelled A and B by a sealed coin flip.
3. **The judge's skill** — installed at the root of that folder as
   `.agents/skills/judge-clone-pairs/SKILL.md`, with `.claude/skills/judge-clone-pairs`
   symlinked to it, exactly as this repository lays out its own skills. An agent opening
   the folder runs the protocol by name instead of reaching back in here for it, and
   there is one file behind both paths rather than two copies that drift. The guides
   beside it are copied from `.agents/skills/judge-clone-pairs/handover/` byte for byte.

**Nothing in that folder is written by you.** Not the guides, not the protocol, not a
note explaining what you did. Every word the judge reads is a file in this repository,
copied across — otherwise two passes hand two different folders to two judges and the
process stops being repeatable. If a guide needs changing, change the file under
`handover/` and rebuild.

Then you hand the folder over and stop.

**You do not judge.** Not one candidate, not "just the obvious ones". Judging in the
session that prepared the folder voids the result, exactly as loudly as reading the
engine's source would. You also do not answer a judge's questions about the engine.

Your second and only other job comes later, after somebody else's verdicts come back:
file them into the register verbatim (Step 3) and score them (Step 4).

| Role | Skill | Runs in | May see |
|---|---|---|---|
| **Preparer** | this one | the Deslop repo | everything |
| **Judge** | `judge-clone-pairs` | the handed-over folder only | target source and candidates |

The judging skill and the folder both live outside this repository. Each repository
directory also carries `JUDGING.md`, a link to the one installed copy of the protocol, so
a judge handed a single directory still has it.

## Why the blind exists

"Stop reading Deslop and now judge fairly" does not work. Context does not unload. Once
the judge has read `sibling.rs`, their reasoning is contaminated: they will think *"the
window is eight siblings wide, so of course the extent stops there"* — which is **the
algorithm's opinion of itself**, filed as ground truth. Assertions built that way agree
with the engine by construction and can never catch it being wrong.

So the separation is physical, not a matter of discipline.

## Step 1 — Build the folder

```bash
make compare            # two engines over every register and everything queued for judging
make judging-folder     # clone the repos, blind the reports, install the judge's skill
```

`make judging-folder` builds `~/clone-judging` (override with `JUDGING_DIR`). One
workspace per repository the last comparison scanned, and nothing in it names this
project:

```
~/clone-judging/                      ← hand over exactly this
  .agents/skills/judge-clone-pairs/SKILL.md   the judge's skill
  .claude/skills/judge-clone-pairs   → symlink to it, so it loads by name
  AGENTS.md  CLAUDE.md                what the folder is; read on open
  click/                              one directory per repository
    JUDGING.md    → the protocol above
    PINNED.txt    source url + commit sha, so the register can cite it
    source/       the repository at the pinned commit, and nothing else
    report-a.json one pair list — { groups: [ { id, regions: [ {path,start_line,end_line} ] } ] }
    report-b.json the other pair list, same shape
    candidates/
      index.md    the checklist — one line per candidate
      0001.md …   one file per candidate: two regions, rendered as source
      pairs.json  the same ranges the merge step reads back
    verdicts.json empty; the judge writes here
  cobra/  axios/  Polly/ …
```

One directory is created **beside** it and must never move inside:
`~/clone-judging.preparer/`, holding `keys/<repo>.key.json` — the seed and which engine got
which letter — and `checkouts/`, the pinned clones. The key is the answer sheet; a judge who
reads one is judging the comparison rather than the code. Neither is needed to judge, and
`checkouts/` is only a cache.

For one repository, or a different location:

```bash
scripts/corpus/prepare-judging.sh ~/clone-judging 1 click
```

### Adding a repository that has never been judged

A repository with no register is not scanned by the comparison, and a repository the
comparison never scanned has no reports to draw candidates from — so a new one would
never get in. `corpus/judging-queue.json` is the way through: add it there with its url,
a full commit id and its language, and the next `make compare` scans it alongside the
registers. Prefer a language with no register at all over a second repository in one that
already has one. When the verdicts come back, the repository moves into
`corpus/register/` and out of the queue. See §[CORPUS-REGISTER-QUEUE].

### What the folder guarantees, and you must not undo

- **Up to 200 candidates**, stratified across provenance (A-only / B-only / shared) and
  size (one-liners, small, medium, large blocks), then shuffled. The sample is wide
  because most candidates land in NOT CLEAR, so it has to be wide enough that the few
  firm verdicts are not all the same shape. Two hundred is a **ceiling, not a quota** —
  past it a single repository stops paying for the time. How many repositories, in which
  languages, and how far each should be taken is
  `docs/specs/corpus.md` §[CORPUS-REGISTER-COVERAGE]; read it before choosing a target.
- **The A/B letters come from a seeded coin flip** recorded only in the key file. Nothing
  in the workspace says which engine is newer, and no candidate says which list it came
  from.
- **Every engine-reasoning field is stripped** — rank, band, mass, node counts,
  occurrence counts, thresholds, tool version, cache stats, embedding provenance,
  timings, binary hashes, metrics.
- **Field names are neutralised** to `groups`/`regions`, so nothing in the workspace uses
  this codebase's vocabulary.
- **The word "deslop" appears nowhere in the workspace.** The script scans its own output
  and refuses to finish if it does. Do not weaken that check — any mention is
  contamination, and a workspace that leaks the producer's identity is worthless.
- Reports are handed over as **raw JSON**, never the rendered HTML report, which carries
  this project's branding throughout.

Then stop. Hand the folder over and say nothing about which report is which.

## Step 2 — The judge works

In a **fresh session**, outside this repository, opened on the folder. The skill is
already installed there, so the judge runs `judge-clone-pairs` and works through one
repository directory at a time, filling in each `verdicts.json`. You have no part in it.

## Step 3 — File it

Verdicts land in `corpus/register/<name>.json`, pinned to the same `sha`:

```json
{
  "name": "click",
  "url": "…", "tag": "…", "sha": "…",
  "clearly_in":  [ { "why": "…", "verified": "…", "occurrences": ["a.py:340-345", "a.py:394-399"] } ],
  "clearly_out": [ { "why": "…", "verified": "…", "occurrences": ["a.cs:15-21", "b.cs:13-18"] } ],
  "not_clear":   [ { "why": "…", "occurrences": [...] } ]
}
```

- Copy `why` and `verified` across **as the judge wrote them**. Do not tidy, sharpen or
  re-argue. If a verdict reads oddly, that is data about the judging, not a typo to fix.
- An empty `clearly_out` needs a `clearly_out_status` saying why it is empty. An empty
  list must never be read as evidence that precision is good.
- Never widen a range so it matches what the engine reported.

Enforced by `crates/deslop/tests/corpus_register_contract.rs` in `make test`.

## Step 4 — Score

Both verdicts use **one predicate, read in opposite directions**:

> An entry is *matched* when some published cluster shows visible occurrences that
> **overlap every listed range**.

- **CLEARLY IN matched** → correct. Unmatched → **false negative**.
- **CLEARLY OUT matched** → **false positive**. Unmatched → correct.

Overlap, not exact line equality. That is what makes it non-fragile: it survives extent
drift, rank movement, band movement, mass changes and occurrence-count changes, and
breaks only when the engine genuinely stops pairing two regions, or starts pairing two it
must not.

```bash
make score-gate            # score the current build and fail if it slipped
```

One register, one scorer, no second copy. The calculation, the gate and the scorecard
markdown all live in `deslop-test-support`'s `corpus_score` module — nothing outside Rust
computes a count, a percentage or a delta. `scripts/compare-versions.sh` scores both
engines on every run, the `corpus-score` CI job gates every push, and the thresholds sit
in `corpus/register/score-thresholds.json`. See `docs/specs/corpus.md` §[CORPUS-SCORE].

This gives degradation a hard definition:

> **Version B degraded against version A** if B misses a CLEARLY IN that A found, or B
> reports a CLEARLY OUT that A stayed silent on.
>
> **Nothing else counts.** Cluster totals, duplication percentages, rank movement and
> band movement are descriptions, not verdicts.

**A new false positive or a new false negative is a bug. Full stop.**

A CLEARLY IN neither version finds is a **standing false negative** — real, not a
regression. A CLEARLY OUT both versions report is a **standing false positive** — same.
Both are reported as standing defects, never as slippage.

## The report

`corpus-score` writes `SCORE.md` and `score.json` beside the reports. **That is the report.**

- It is **mechanical**: every count, percentage, delta and cost figure is emitted by the code that took the measurement.
- It is **in the repository**, at a path you can hand to anyone.
- It is **never hosted, uploaded or published** — not as an artifact, not as a page, not anywhere.

Do not transcribe it into prose, do not re-type its numbers into a summary, and do not build a prettier copy of it. A hand-written report cannot be re-run, cannot be diffed against the last run, and is free to be wrong in ways nothing catches. When someone asks for the results, give them the path.

## Growing the register

- Every confirmed false positive or false negative earns an entry that would have caught it.
- **An entry is never deleted or softened to make a run pass.** If the engine now
  contradicts a CLEARLY IN, either the engine is wrong or the entry was never CLEARLY IN
  — and re-judging needs a fresh isolated workspace and a fresh reading of the source.
- Promoting NOT CLEAR to a verdict later is allowed. Demoting a verdict to NOT CLEAR
  requires stating what the original judge got wrong **about the source**.

## Common ways this goes wrong

- **Preparing and judging in one session.** The whole point, quietly defeated.
- **Leaving the keys or the checkouts inside the folder.** The key names which engine
  produced which list; a judge who reads one is judging the comparison, not the code.
- **Handing over a folder without the skill in it.** The judge then has to be told where
  the protocol lives, and the obvious place to look is back in here.
- **Handing over the HTML report** instead of the stripped JSON. It names this project on
  every page.
- **Answering the judge's question** about why the engine paired something.
- **Editing a verdict on the way into the register.** You are a scribe at this step.
- **Reading a cluster-id change as a regression.** An id is not a finding. Two engines can
  pair the same regions under different ids; only the score settles it.
- **Chasing volume in the register.** Ten unarguable entries beat two hundred arguable
  ones. The candidate sample is large so the register can stay small.
- **Grinding one repository past its ceiling.** Once a repository has been judged to
  roughly two hundred candidates, the next pass belongs on a different repository — and
  preferably a different language. Coverage across languages catches more than depth in
  one of them.
