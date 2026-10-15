# Clone registers

Independent ground truth for accuracy: one file per target repository, pinned to the
commit it was judged at.

Each file lists code pairs a judge classified **CLEARLY IN** (an obvious clone — not
reporting it is a false negative), **CLEARLY OUT** (a pairing that would be plainly
wrong — reporting it is a false positive), or **NOT CLEAR** (recorded, asserts nothing).

The judge works in an isolated workspace and never loads this codebase. Read
`docs/specs/corpus.md` §[CORPUS-REGISTER] for the contract and
§[CORPUS-REGISTER-COVERAGE] for how far this is meant to go, then
`.agents/skills/clone-register-prepare` before building or filing anything here.

- Build the folder a judge is handed — repositories, reports and the judging skill:
  `make judging-folder`
- Judge it (fresh session, outside this repo): `.agents/skills/judge-clone-pairs`
- Score the current build and gate on it: `make score-gate`
- Compare two engines against the registers: `scripts/compare-versions.sh`
- Enforced by `crates/deslop/tests/corpus_register_contract.rs` in `make test`, by the
  `corpus-score` CI job on every push, and by `scripts/compare-versions.sh` on every
  version comparison.

The score itself and its markdown are `docs/specs/corpus.md` §[CORPUS-SCORE]. `outcomes.json` beside these files is not a register: it records every judged pair's last outcome, pass or fail, and since when. A pair on record as failing is a known defect; one that fails after passing, or with no record, fails the run. See `docs/specs/corpus.md` §[CORPUS-SCORE-OUTCOMES].

## Where it stands

The target is two to three repositories per language at visibly different sizes, each
carried to roughly 100 CLEARLY IN and 100 CLEARLY OUT.

**Per-repository counts are not written here.** They change on every merge, and a
hand-kept table of them goes stale silently. `docs/reports/verdict-merge.md` carries them,
written by the run that merged them. Read that for the numbers; read this for what to
judge next.

| Language | Judged | Next |
|---|---|---|
| python | click | a large python repo; `django` or `tornado` are pinned already |
| go | cobra, hugo | more passes on hugo |
| javascript | axios, react | more passes on react |
| csharp | Polly, serilog | more passes on serilog |
| rust | ripgrep, deslop | a second rust repo of a different size; `deslop` is the self-register, filed from tracked issues rather than a blinded pass (see its `provenance`) |
| typescript | zod | a second typescript repo of a different size |
| dart | bloc, dio | more passes on dio |
| php | guzzle, composer | more passes on composer |
| fsharp | FSharp.Data, giraffe (no agreed CLEARLY IN yet) | another pass on giraffe, which is still queued |

Every one of the nine languages now has a register. Python and TypeScript still rest on a single repository, and the second repositories above were judged on a short pass, so depth is what the next passes buy, not breadth. `corpus/judging-queue.json` still holds giraffe, because its register has no agreed CLEARLY IN. `corpus_commit_pins.rs` fails whenever the queue is empty, by design.

A CLEARLY IN may pin a clone type (`Type I` to `Type IV`); blank accepts any type. See
`docs/specs/corpus.md` §[CORPUS-REGISTER-TYPE].

Counts are not written here. `docs/reports/verdict-merge.md` carries them, written by the
run that produced them.

"Queued" means `corpus/judging-queue.json`: the comparison scans those repositories so a
first pass has reports to draw candidates from. Each gets a register here when its verdicts come back, and leaves the queue once that register holds a CLEARLY IN. See `docs/specs/corpus.md` §[CORPUS-REGISTER-QUEUE].

Verdicts reach these files only through `make merge-verdicts JUDGED_DIRS="<folder> <folder>"`.
It imports **only** the pairs every source agrees on — these registers included. Anything a
source disagrees on, or is not confident about, is left out and listed in
`docs/reports/verdict-merge.md`. See §[CORPUS-REGISTER-MERGE].

The counts are low on purpose. They rise by running more passes at fresh seeds, never by
admitting an arguable pair.
