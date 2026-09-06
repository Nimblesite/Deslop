# Judging folder

Every directory beside this file is one repository to rule on. Each holds that
repository's source at a fixed commit, two lists of candidate duplicate pairs labelled
A and B, and the pairs drawn from those lists for you to judge.

Run the `judge-clone-pairs` skill, then take one directory at a time:
`<directory>/candidates/index.md` is the checklist, and your verdicts go in
`<directory>/verdicts.json`. Finish a repository before opening the next one.

**The letters are arbitrary and sealed.** Neither list is the newer one, and which list
a pair came from is not evidence about the pair.

Everything you need is inside this folder. Do not read anything outside it, and do not
try to find out what produced the two lists — if you learn it, the pass is void and
every verdict in it has to be thrown away.
