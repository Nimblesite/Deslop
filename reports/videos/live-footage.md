# Documentation film footage [DOC-FILMS-LIVE]

11 films use 30 distinct recorded application takes across their complete timelines. Only captions, step labels and playback-speed labels overlay those recordings. Finished-frame content verification is in workflow-verification.md.

| Film | Recorded application timeline (seconds) | Recorded takes |
| --- | ---: | --- |
| getting-started | 56.33 | start-extension, review-finding, review-sources, review-compare |
| configuration | 57.51 | config-create, config-exclude, config-override, config-limit |
| ai-integration | 63.06 | agent-ask, agent-search, agent-results, review-compare |
| how-it-works | 59.89 | review-finding, review-compare, review-sources, review-refresh |
| osprey | 60.34 | osprey-finding, osprey-compare, osprey-python, osprey-config |
| github-action | 55.76 | action-create, action-version, action-limit, action-changes |
| vscode-cluster-panel | 59.63 | review-finding, review-compare, review-pair, review-counts |
| accuracy-transparency | 60.31 | review-counts, review-sources, review-compare, config-exclude |
| research-background | 59.01 | review-sources, review-edit, review-edited-compare, review-pair |
| for-ai | 61.74 | agent-duplicates, agent-details, agent-search, review-pair |
| python-click | 72.67 | python-open, python-compare, python-identical, python-dedupe |

The timeline accounts for the complete encoded duration; this inventory supplements finished-frame review and playback checks. Narration is local Kokoro-82M (af_heart).

Regenerate with `node scripts/videos/record-footage-inventory.mjs`.
