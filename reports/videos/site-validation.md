# Website and film browser validation

- Source commit: 7bb61014fd6b58bb32d717180969f570c172ffd7
- Result: 22 passed in 16.7s
- Documentation routes: 20
- Playback and seeking checked at phone and desktop widths.
- Command: `DESLOP_SITE_TEST_PORT=8097 npm test -- documentation-films.spec.js --workers=2`
- Server: Playwright-managed Eleventy development server on port 8097.

Regenerate after a successful run with `node scripts/videos/record-site-check.mjs <captured-test-log>`. This receipt is emitted from the completed Playwright output; it does not claim subjective audio review.

## Passed checks

```text
✓   1 tests/documentation-films.spec.js:63:1 › getting started fits the reading column without prose CSS (500ms)
✓   2 tests/documentation-films.spec.js:84:3 › en getting-started: responsive player and playable delivery (835ms)
✓   3 tests/documentation-films.spec.js:84:3 › /zh getting-started: responsive player and playable delivery (620ms)
✓   4 tests/documentation-films.spec.js:84:3 › en configuration: responsive player and playable delivery (620ms)
✓   5 tests/documentation-films.spec.js:84:3 › /zh configuration: responsive player and playable delivery (620ms)
✓   6 tests/documentation-films.spec.js:84:3 › en ai-integration: responsive player and playable delivery (592ms)
✓   7 tests/documentation-films.spec.js:84:3 › /zh ai-integration: responsive player and playable delivery (591ms)
✓   8 tests/documentation-films.spec.js:84:3 › en how-it-works: responsive player and playable delivery (594ms)
✓   9 tests/documentation-films.spec.js:84:3 › /zh how-it-works: responsive player and playable delivery (600ms)
✓  10 tests/documentation-films.spec.js:84:3 › en osprey: responsive player and playable delivery (564ms)
✓  11 tests/documentation-films.spec.js:84:3 › /zh osprey: responsive player and playable delivery (718ms)
✓  12 tests/documentation-films.spec.js:84:3 › en github-action: responsive player and playable delivery (779ms)
✓  13 tests/documentation-films.spec.js:84:3 › /zh github-action: responsive player and playable delivery (708ms)
✓  14 tests/documentation-films.spec.js:84:3 › en vscode-cluster-panel: responsive player and playable delivery (736ms)
✓  15 tests/documentation-films.spec.js:84:3 › /zh vscode-cluster-panel: responsive player and playable delivery (643ms)
✓  16 tests/documentation-films.spec.js:84:3 › en accuracy-transparency: responsive player and playable delivery (770ms)
✓  17 tests/documentation-films.spec.js:84:3 › /zh accuracy-transparency: responsive player and playable delivery (650ms)
✓  18 tests/documentation-films.spec.js:84:3 › en research-background: responsive player and playable delivery (667ms)
✓  19 tests/documentation-films.spec.js:84:3 › /zh research-background: responsive player and playable delivery (665ms)
✓  20 tests/documentation-films.spec.js:84:3 › en for-ai: responsive player and playable delivery (707ms)
✓  21 tests/documentation-films.spec.js:84:3 › /zh for-ai: responsive player and playable delivery (749ms)
✓  22 tests/documentation-films.spec.js:101:1 › retired Osprey docs open the localized launch film (223ms)
```
