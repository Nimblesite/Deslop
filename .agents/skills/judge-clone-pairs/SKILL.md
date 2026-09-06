---
name: judge-clone-pairs
description: Sort candidate code pairs into CLEARLY IN, CLEARLY OUT or NOT CLEAR by reading the source and nothing else. Use when handed a judging workspace holding a source tree, two pair lists labelled A and B, and a candidates folder. Triggers - "judge these pairs", "clearly in", "clearly out", "is this a real clone", "sort these candidates".
argument-hint: "<workspace-directory>"
---

# Judging Clone Pairs

You have been handed a folder. Inside it is one directory per repository, and inside
each of those sits somebody else's source code at a fixed commit, two lists of candidate
pairs labelled **A** and **B**, and a folder of candidates drawn from those lists.

Take one repository at a time, start to finish, and write that repository's verdicts
before opening the next.

Your whole job is to answer one question about each candidate, and no other question:

> **Is this the same code written twice?**

You are not evaluating a tool. You do not know what produced these lists. You must not
find out.

## Before the first verdict — confirm these out loud

1. The working directory is the folder you were handed, and it is **not** inside the
   checkout of any analysis tool.
2. Each repository directory holds only `source/`, `report-a.json`, `report-b.json`,
   `candidates/`, `PINNED.txt`, `verdicts.json` and this document; the folder around
   them holds only those directories, this protocol and a `README.md`.
3. Nothing about whatever produced these reports has been read this session — no source,
   specs, docs, tests, issues, changelog or even its name.

If any of the three cannot be confirmed, **abort before starting rather than after.**

## 🚨 Contamination aborts the pass — immediately

Once you know how the producing tool works, you cannot un-know it. You will catch
yourself reasoning *"of course the region stops there, the window is eight siblings
wide"* — which is **the tool's opinion of itself**, filed as ground truth. Verdicts
reached that way agree with the tool by construction and can never catch it being wrong.
That is not a judgement. It is a mirror.

**Contamination is any of these:**

- Reading source, specs, plans, tests, manifests or agent instructions belonging to the
  tool that produced these reports.
- A tool result that quotes that code or spec text, however incidentally.
- Being told how it works, what any threshold is, or why it emitted a pair.
- Seeing any field that expresses the producer's own confidence — a score, rank, band,
  weight, node count, threshold or version stamp. These are stripped from the workspace;
  if one appears, something went wrong upstream.
- Learning which of A and B is newer, or which one a candidate came from.

**On contamination, in this order and nothing else:**

1. **STOP.** Do not finish the candidate in hand.
2. **Discard every verdict from this session** — including ones written before the
   contamination. You cannot prove which reasoning was already drifting.
3. **Report** what was loaded, how, and how many verdicts were thrown away.
4. **Restart** in a fresh session with a clean workspace.

Never carry a verdict across a contamination boundary. Never "keep the good ones".

## The three verdicts

| Verdict | Meaning | Admission price |
|---|---|---|
| **CLEARLY IN** | An obvious clone. No reasonable engineer would say otherwise. | Certainty. |
| **CLEARLY OUT** | Putting these two regions together is plainly wrong. | Certainty. |
| **NOT CLEAR** | Anything carrying a shred of doubt. | None — it asserts nothing. |

**Most candidates are NOT CLEAR. That is the expected, healthy outcome.** Out of two
hundred and fifty candidates, a dozen firm verdicts is a good pass. The value of this
work comes from being small and unarguable, never from volume.

> **If you hesitate, it is NOT CLEAR.** If you find yourself building an argument for a
> verdict, you have already answered: the argument is the doubt.

## How to judge one candidate

Open `candidates/NNNN.md`. It names two regions of `source/`. Read both in the file
itself — not only the rendered excerpt — so you see what surrounds them.

**CLEARLY IN** — you would say, without hesitating, *"someone copied this."*

- Byte-identical spans of real length in two places.
- The same code with one systematic rename applied throughout (`Foo`→`FooAsync`,
  `Required`→`Dirname`, `--debug`→`--no-debug`) and nothing else changed.
- A sync/async, or generic/non-generic, twin of the same routine.
- The same named function defined in two files that do not share it.

**CLEARLY OUT** — you would say, without hesitating, *"these have nothing to do with
each other."*

- Two regions sharing only a syntactic skeleton: unrelated parameter lists, unrelated
  `switch` blocks, unrelated import runs.
- Pairs whose only commonality is a language or framework requirement.
- A short fragment set against a long block that merely contains something of the same
  shape — where *"this region is a copy of that region"* is simply false.

**NOT CLEAR** — everything else, and specifically:

- Scaffolding that genuinely repeats but is the idiom the framework invites
  (`invoke`, `assert`, `assert`).
- Two- and three-line idioms.
- Pairs whose *content* is duplicated but whose *boundaries* are ragged.

### Two traps

- **Repetition is not the question.** Test scaffolding is often genuinely repeated *and*
  genuinely NOT CLEAR. Both are true at once.
- **A ragged boundary is not a wrong pair.** If two regions really do share copied code
  but the start or end line is off, that is a real clone with a bad boundary — it is not
  CLEARLY OUT. Record it as NOT CLEAR and say so in the note.

## Verify before you write

Never file a verdict from the rendered excerpt alone. Extract both ranges out of
`source/` and diff them. Record what the diff **actually returned** — "byte-identical
over 26 lines", or the exact list of what differs.

A `verified` line that cannot be reproduced by re-running that diff is not a verdict.

## The two reports

`report-a.json` and `report-b.json` are the raw pair lists the candidates were drawn
from, in one neutral shape:

```json
{ "groups": [ { "id": "…", "regions": [ { "path": "…", "start_line": 1, "end_line": 9 } ] } ] }
```

They are there so you can look up a candidate's wider family before judging it — a pair
drawn from a group of sixty regions deserves that context.

**A and B are sealed arbitrary labels.** Neither is "the new one"; which is which was
decided by a coin flip recorded outside this folder. So:

- Do not compare the totals. "A has more groups than B" is a fact about two lists, not a
  judgement about any pair.
- Do not let a candidate's presence in one list and absence from the other move you a
  millimetre. If you notice it, note that you noticed and judge the code anyway.

## Writing `verdicts.json`

```json
{
  "clearly_in": [
    {
      "candidate": 41,
      "why": "plain English, as an engineer would say it out loud",
      "verified": "the diff you ran, and what it returned",
      "occurrences": ["path/one.py:340-345", "path/one.py:394-399"]
    }
  ],
  "clearly_out": [ { "candidate": 7,  "why": "…", "verified": "…", "occurrences": [...] } ],
  "not_clear":   [ { "candidate": 12, "why": "…", "occurrences": [...] } ]
}
```

- `why` — the judgement in plain English. Not a rule, not a metric. What you would tell
  a colleague leaning over your shoulder.
- `verified` — the diff, and its result.
- `occurrences` — two or more ranges. **Never write a range you did not read**, and never
  widen one so it matches a report.
- `not_clear` is recorded so nobody re-litigates it, and asserts nothing.

Every candidate gets a verdict. Skipping one hides a decision you did make.

The result may come out **asymmetric** — a repository can yield several CLEARLY IN and no
CLEARLY OUT, or the reverse. **Never manufacture an entry to balance the table.**

## Your output is a file, not a message

`verdicts.json` in each repository directory is the deliverable — one per repository. Do
not host them, upload them, publish them, or paste a prettied-up version anywhere. Write
the files and say where they are.

## Common ways this goes wrong

- **Judging from the excerpt.** The excerpt is an index, not the evidence.
- **Reasoning from where a candidate appeared.** That is voting for a list, not reading code.
- **Filing a boundary complaint as CLEARLY OUT.** See the traps above.
- **Filing idiomatic scaffolding as CLEARLY IN** because it really is repeated.
- **Chasing volume.** Ten unarguable verdicts beat two hundred arguable ones.
- **Softening a verdict later to make something pass.** If a verdict is ever contradicted,
  either the contradiction is right or the verdict was never certain — and re-deciding
  needs a fresh workspace and a fresh reading of the source, not an edit.
