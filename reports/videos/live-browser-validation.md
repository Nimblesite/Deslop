# Website and film browser validation

- Source commit: 679ed2f3b1f1ffeabfd80e16b6ffde7450e499f4
- Result: 22 passed in 34.9s
- Documentation routes: 20
- Playback and seeking checked at phone and desktop widths.
- Command: `DESLOP_SITE_TEST_URL=https://deslop.live npm test -- documentation-films.spec.js --workers=2`
- Server: https://deslop.live; no local web server.

Regenerate after a successful run with `node scripts/videos/record-site-check.mjs <captured-test-log>`. This receipt is emitted from the completed Playwright output; it does not claim subjective audio review.

## Passed checks

```text
✓   1 tests/documentation-films.spec.js:65:1 › getting started fits the reading column without prose CSS (701ms)
✓   2 tests/documentation-films.spec.js:103:3 › en getting-started: responsive player and playable delivery (1.9s)
✓   3 tests/documentation-films.spec.js:103:3 › /zh getting-started: responsive player and playable delivery (1.6s)
✓   4 tests/documentation-films.spec.js:103:3 › en configuration: responsive player and playable delivery (1.6s)
✓   5 tests/documentation-films.spec.js:103:3 › /zh configuration: responsive player and playable delivery (1.3s)
✓   6 tests/documentation-films.spec.js:103:3 › en ai-integration: responsive player and playable delivery (2.2s)
✓   7 tests/documentation-films.spec.js:103:3 › /zh ai-integration: responsive player and playable delivery (2.1s)
✓   8 tests/documentation-films.spec.js:103:3 › en how-it-works: responsive player and playable delivery (1.5s)
✓   9 tests/documentation-films.spec.js:103:3 › /zh how-it-works: responsive player and playable delivery (1.5s)
✓  10 tests/documentation-films.spec.js:103:3 › en osprey: responsive player and playable delivery (2.4s)
✓  11 tests/documentation-films.spec.js:103:3 › /zh osprey: responsive player and playable delivery (1.4s)
✓  12 tests/documentation-films.spec.js:103:3 › en github-action: responsive player and playable delivery (1.4s)
✓  13 tests/documentation-films.spec.js:103:3 › /zh github-action: responsive player and playable delivery (1.6s)
✓  14 tests/documentation-films.spec.js:103:3 › en vscode-cluster-panel: responsive player and playable delivery (2.1s)
✓  15 tests/documentation-films.spec.js:103:3 › /zh vscode-cluster-panel: responsive player and playable delivery (1.2s)
✓  16 tests/documentation-films.spec.js:103:3 › en accuracy-transparency: responsive player and playable delivery (1.5s)
✓  17 tests/documentation-films.spec.js:103:3 › /zh accuracy-transparency: responsive player and playable delivery (1.5s)
✓  18 tests/documentation-films.spec.js:103:3 › en research-background: responsive player and playable delivery (1.5s)
✓  19 tests/documentation-films.spec.js:103:3 › /zh research-background: responsive player and playable delivery (1.2s)
✓  20 tests/documentation-films.spec.js:103:3 › en for-ai: responsive player and playable delivery (1.9s)
✓  21 tests/documentation-films.spec.js:103:3 › /zh for-ai: responsive player and playable delivery (1.3s)
✓  22 tests/documentation-films.spec.js:121:1 › retired Osprey docs open the localized launch film (999ms)
```
