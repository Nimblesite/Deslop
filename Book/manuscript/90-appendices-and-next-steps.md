# Quick reference

| Job | Tool or view |
|---|---|
| Check proposed code | `find-similar` with `snippet` and `language`; inspect `clusters` and `existing` |
| List existing groups | `duplicates`, starting with a small summary |
| Inspect one group | `cluster-by-id`; page occurrences with `offset` |
| Check a file or range | `duplicates` with `path`, optionally both byte offsets |
| Compare two exact ranges | `compare-pair` with `left` and `right` endpoints |
| Recover watcher lag | `rescan`, optionally with changed `paths` |
| Verify the live root | `session {}` |
| Read field definitions | `schema-doc` once per session |
| Preview a supported consolidation | `merge-plan`; inspect the verdict and edit |
| Scan saved source in CI | Installed Deslop CLI |
| Inspect ranked findings visually | Top Offenders in the editor |

Use the rule in Chapter 5 and the cleanup record in Chapter 12. Ranges refer to the analysed source; refresh them after editing.

## Evidence for this edition

Screenshots were captured from the running Deslop VS Code extension on macOS, analysing the Deslop repository. Untouched captures are under `assets/screenshots/masters/`; publication crops, environment details, dimensions, and SHA-256 hashes are listed in `figures.json`. The installed binaries report version `0.38.0`; hashes identify the captured build more precisely than the version alone. This is a working checkout, not a frozen release fixture or a demonstrated cleanup.

The examples are exercises to perform on your own code. No before-and-after cleanup results or separate Workshop repository are claimed.

## Current documentation

- [Agent setup](https://deslop.live/docs/ai-integration/)
- [Agent workflow and recovery](https://deslop.live/docs/for-ai/)
- [Configuration](https://deslop.live/docs/configuration/)
- [Clone definitions](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/taxonomy.md)
