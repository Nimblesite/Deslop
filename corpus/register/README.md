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
carried to roughly 100 CLEARLY IN and 100 CLEARLY OUT. This table is the standing, kept
current so the next pass can be chosen without re-deriving it.

| Language | Judged | Size | IN | OUT | Next |
|---|---|---|---|---|---|
| python | click | 79 files / 28.6k loc | 2 | 0 | a large python repo; `django` or `tornado` are pinned already |
| go | cobra | 36 files / 16.5k loc | 3 | 0 | a large go repo; `hugo` is pinned already |
| javascript | axios | 164 files / 17.1k loc | 3 | 0 | a large js repo; `react` is pinned already |
| csharp | Polly | 243 files / 54.7k loc | 4 | 2 | a small csharp repo to sit under Polly |
| rust | — | — | 0 | 0 | **nothing judged**; `ripgrep` is queued |
| typescript | — | — | 0 | 0 | **nothing judged**; `zod` is queued |
| dart | — | — | 0 | 0 | **nothing judged**; `bloc` is queued |
| php | — | — | 0 | 0 | **nothing judged**; `guzzle` is queued |
| fsharp | — | — | 0 | 0 | **nothing judged**; `FSharp.Data` is queued |

Every judged repository so far is on the small side, and five of the nine languages in
the corpus have no register at all. **A language with nothing judged is the strongest
candidate for the next pass** — breadth beats depth, because a false positive that only
shows up in one language stays invisible until that language is judged.

"Queued" means `corpus/judging-queue.json`: the comparison scans those repositories so a
first pass has reports to draw candidates from, and each moves into this directory when
its verdicts come back. See `docs/specs/corpus.md` §[CORPUS-REGISTER-QUEUE].

The counts are low on purpose. They rise by running more passes at fresh seeds, never by
admitting an arguable pair.
