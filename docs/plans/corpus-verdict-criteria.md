# Measured verdict criteria for the clone register

## [CORPUS-REGISTER-CRITERIA] The problem

The registers hold **350 CLEARLY IN, 100 CLEARLY OUT and 3,334 NOT CLEAR** pairs. The last Claude + Codex merge added 466 pairs. Only 14 of those were decisive; 452 were NOT CLEAR, and a further 138 pairs were left out because the judges disagreed.

The judging rules in `.claude/skills/judge-clone-pairs/SKILL.md` rest on feeling: "a shred of doubt", "if you hesitate", "scaffolding the framework invites". Two careful judges read the same diff and land on different sides of those lines. In 100 left-out pairs, one judge said CLEARLY IN and the other said NOT CLEAR. Most of those are byte-identical or one-token-different blocks that Codex filed as test scaffolding. One example is the 84 verbatim lines between axios `test/module/typings/esm/index.ts` and `cjs/index.ts`. A NOT CLEAR pair asserts nothing, so every such pair is a test the corpus does not run.

## Criteria

Each pair is decided by the first tier that applies. Tiers 1 and 2 are measured by a script, not judged. That script is independent of Deslop's engine, so the engine is never graded by its own code.

**Tier 1 — measured CLEARLY IN.** All four conditions must hold:

- The two ranges do not overlap.
- Each range has at least `MIN_CLONE_LINES` = 6 non-blank, non-comment lines.
- Compared as tree-sitter leaf tokens, with comments dropped, the ranges are either:
  - identical (**Type I**), or
  - identical apart from a consistent one-to-one renaming of identifiers, literals and type names (**Type II**).
- At least one statement is not an import or a `use`.

Test files and scaffolding get **no exemption**. An identical 84-line block is a copy whether it sits in a test or not. Whether that copy is worth refactoring is a ranking question, not a verdict question.

**Tier 2 — measured CLEARLY OUT.** A short fragment set against a much longer region (length ratio ≥ 3) is CLEARLY OUT when either:

- the two share no identifier and no literal beyond language keywords, or
- their token sequences align for less than `MAX_UNRELATED_ALIGNMENT` = 30 %.

These are the "shared syntactic skeleton" pairs the skill already calls CLEARLY OUT.

**Tier 3 — ragged boundaries are settled, not shelved.** The script finds the longest aligned token run inside both ranges. If that core passes Tier 1, the pair is CLEARLY IN with its candidate ranges unchanged (the scorer matches by overlap), and `verified` names the core's lines.

**Tier 4 — Type III and Type IV stay with judges, with numbers attached.** The workspace shows each judge:

- the aligned-statement ratio, and
- the number of edited statements.

A pair with ≥ 70 % of statements aligned and edits confined to ≤ 2 contiguous blocks is CLEARLY IN unless the judge names a concrete reason it is not a copy. Below 40 % aligned, it is NOT CLEAR at best. A judge disagreement is settled by one blinded third judge reading the same numbers, not by being left out.

**Unchanged.** The register still never grows by softening a verdict, and no entry is deleted to make a run pass ([CORPUS-REGISTER]). Tier 1 and Tier 2 do not relax certainty. They replace the judge's sense of certainty with a measurement anyone can rerun.

## Steps

- [x] Let re-judging promote: the merge read a standing NOT CLEAR as a vote against any later firm verdict, so no pair could ever leave NOT CLEAR. It is now promoted when every new judge agrees (`scripts/corpus/merge-verdicts/pass.mjs`, counted as `pairs_promoted`).
- [x] Queue 18 more repositories so each supported language has two to four of different sizes (`corpus/judging-queue.json`, 28 queued).
- [x] Judging folders `~/clone-judging-claude` and `~/clone-judging-codex` (identical; 28 repositories, 5,576 candidates, including all 1,000 requeued NOT CLEAR pairs drawn first), built from the last comparison's reports under the corrected skill.
- [ ] Two independent judge sessions, then `make _merge-verdicts`.
- [x] Second folder pair for the 18 newly queued repositories: `~/clone-judging-queued-claude` and `~/clone-judging-queued-codex` (identical, 3,422 candidates), from `compare-2026-11-08T04-10-47Z`.
- [x] Remove the contradictions from the judging skill (`.agents/skills/judge-clone-pairs/SKILL.md`), the workspace candidate text (`scripts/corpus/register-workspace.mjs`) and `docs/specs/corpus.md` [CORPUS-REGISTER]: a fixed six-real-line bar, no test or scaffolding exemption for `Type I`/`Type II`, ragged boundaries filed as CLEARLY IN around a qualifying core.
- [ ] Write the measurement as a test-support binary in Rust: `crates/deslop-test-support/src/corpus_score/criteria.rs`. It tokenises with the workspace's tree-sitter grammars but uses none of the pipeline's normalisation, fingerprinting or clustering. Each threshold is a named, config-overridable constant.
- [ ] Add fixtures for every tier boundary: exactly 5 and exactly 6 lines; one inconsistent rename; overlapping ranges; import-only blocks; a 29 % and a 31 % alignment.
- [ ] Dry run over the 3,334 NOT CLEAR entries and the 138 left-out pairs. Write `docs/reports/verdict-criteria-dry-run.md` from the script, with counts moved per tier and per repository and every promoted pair listed.
- [ ] Hand-check a seeded random sample of 30 promotions per tier against the source. Any wrong promotion stops the tier until its rule is fixed.
- [ ] Amend `docs/specs/corpus.md` [CORPUS-REGISTER] and the judging skill to match, then apply. Promoted entries carry `"verified"` with the measured token diff and `"decided_by": "criteria tier N"`.
- [ ] Run the score gate. New CLEARLY IN pairs that Deslop misses are real false negatives, filed as such, not relaxed.

## [CORPUS-REGISTER-EXTERNAL] Open-source judgments: research, 2026-11-08

Deslop is MIT-licensed and published commercially by Nimblesite Pty Ltd. A source can enter `corpus/` only if its licence allows redistribution inside an MIT repository for commercial use. It must also point at real repository code, so that it fits the pinned-repo register format. Every imported pair still passes our own Tier 1–4 check against the pinned source before it is filed.

| Source | Licence | Fit | Verdict |
| --- | --- | --- | --- |
| [BigCloneBench](https://github.com/clonebench/BigCloneBench) | CC BY-NC 4.0, "applies to the benchmark database and its derivatives" | Java only. [Krinke & Ragkhitwetsagul 2025](https://arxiv.org/abs/2505.04311) found 93 % of a 406-pair sample of its weak Type-3/Type-4 pairs mislabelled. | ❌ Non-commercial clause. |
| CodeXGLUE BigCloneBench copy ([Hugging Face](https://huggingface.co/datasets/google/code_x_glue_cc_clone_detection_big_clone_bench)) | C-UDA, derived from BigCloneBench | Inherits BigCloneBench's terms and errors. | ❌ |
| [GPTCloneBench](https://github.com/srlabUsask/GPTCloneBench) | README: CC BY-NC-ND; the sidebar also shows MIT, which is contradictory | Code is GPT-generated, with no repository paths. | ❌ Non-commercial, no derivatives, synthetic. |
| SemanticCloneBench ([paper](https://clones.usask.ca/pubfiles/articles/Omari_SemanticClonesBenchIWSC2020.pdf)) | None published. The code comes from Stack Overflow (CC BY-SA). | Snippets of semantic (Type IV) clones in Java, C, C# and Python. No repository ranges. | ⚠️ Licence unverified. At best a separately licensed fixture set, never register entries. |
| Bellon's benchmark ([usask list](https://clones.usask.ca/clones/benchmarks/)) | None found | File and line ranges over 8 C and Java systems; only about 2 % of candidates were hand-validated, by one person, in 2002. | ⚠️ Usable only with written permission from the authors, then re-verified by Tiers 1–4. |

**Result:** none of these can be imported under licence today. The fastest path to more decisive pairs is the measured criteria above, applied to the 3,472 pairs already judged (3,334 NOT CLEAR plus 138 left out). Bellon's data is the one external candidate worth asking about. It needs a permission request to its authors, which has not been sent.
