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

The score itself, its thresholds and its markdown are `docs/specs/corpus.md`
§[CORPUS-SCORE]. Gate exceptions live in `score-thresholds.json` beside these files:
one entry per tracked defect, with the reason, tightened only by fixing the engine.

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
| go | cobra | a large go repo; `hugo` is pinned already |
| javascript | axios | a large js repo; `react` is pinned already |
| csharp | Polly | a small csharp repo to sit under Polly |
| rust | ripgrep | a second rust repo of a different size |
| typescript | zod | a second typescript repo of a different size |
| dart | bloc | a second dart repo of a different size |
| php | guzzle | a second php repo of a different size |
| fsharp | FSharp.Data | a second fsharp repo of a different size |

Every one of the nine languages now has a register, and every judged repository is on the
small side — so depth and size are what the next passes buy, not breadth. **The judging
queue is empty**, which means nothing new can enter the register until a repository is
added to `corpus/judging-queue.json`; `corpus_commit_pins.rs` fails while that is true,
by design.

Counts are not written here. `docs/reports/verdict-merge.md` carries them, written by the
run that produced them.

"Queued" means `corpus/judging-queue.json`: the comparison scans those repositories so a
first pass has reports to draw candidates from, and each moves into this directory when
its verdicts come back. See `docs/specs/corpus.md` §[CORPUS-REGISTER-QUEUE].

Verdicts reach these files only through `make merge-verdicts JUDGED_DIRS="<folder> <folder>"`.
It imports **only** the pairs every source agrees on — these registers included. Anything a
source disagrees on, or is not confident about, is left out and listed in
`docs/reports/verdict-merge.md`. See §[CORPUS-REGISTER-MERGE].

The counts are low on purpose. They rise by running more passes at fresh seeds, never by
admitting an arguable pair.
