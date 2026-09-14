# One repository

`source/` is this repository at a fixed commit, and nothing else. `PINNED.txt` names the
url and the commit, so a verdict can cite exactly what was read.

`report-a.json` and `report-b.json` are two lists of candidate duplicate pairs found in
that source by two runs of the same kind of analysis. **The letters are arbitrary and
sealed** — neither is the newer run, and knowing which list a pair came from is not
evidence about the pair.

`candidates/` holds the pairs drawn from those two lists, shuffled. Start at
`candidates/index.md` and write your verdicts into `verdicts.json`.

**Read `JUDGING.md` first.** Do not try to find out what produced these reports — if you
learn it, the pass is void and every verdict in it must be discarded.
