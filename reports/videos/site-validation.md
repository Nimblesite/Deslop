# Website and film browser validation

- Source commit: 4124a406b450e18e2bf82a1710e1112ca0376d40
- Result: 21 passed in 15.7s
- Documentation routes: 20
- Playback and seeking checked at phone and desktop widths.
- Command: `DESLOP_SITE_TEST_PORT=8097 npm test -- documentation-films.spec.js --workers=2`
- Server: installed `@11ty/eleventy-dev-server`, serving an immutable completed website build on port 8097.

Regenerate after a successful run with `node scripts/videos/record-site-check.mjs <captured-test-log>`. This receipt is emitted from the completed Playwright output; it does not claim subjective audio review.

## Passed checks

```text
✓   1 tests/documentation-films.spec.js:63:1 › getting started fits the reading column without prose CSS (345ms)
✓   2 tests/documentation-films.spec.js:84:3 › en getting-started: responsive player and playable delivery (782ms)
✓   3 tests/documentation-films.spec.js:84:3 › /zh getting-started: responsive player and playable delivery (619ms)
✓   4 tests/documentation-films.spec.js:84:3 › en configuration: responsive player and playable delivery (651ms)
✓   5 tests/documentation-films.spec.js:84:3 › /zh configuration: responsive player and playable delivery (663ms)
✓   6 tests/documentation-films.spec.js:84:3 › en ai-integration: responsive player and playable delivery (626ms)
✓   7 tests/documentation-films.spec.js:84:3 › /zh ai-integration: responsive player and playable delivery (639ms)
✓   8 tests/documentation-films.spec.js:84:3 › en how-it-works: responsive player and playable delivery (650ms)
✓   9 tests/documentation-films.spec.js:84:3 › /zh how-it-works: responsive player and playable delivery (689ms)
✓  10 tests/documentation-films.spec.js:84:3 › en osprey: responsive player and playable delivery (598ms)
✓  11 tests/documentation-films.spec.js:84:3 › /zh osprey: responsive player and playable delivery (683ms)
✓  12 tests/documentation-films.spec.js:84:3 › en github-action: responsive player and playable delivery (635ms)
✓  13 tests/documentation-films.spec.js:84:3 › /zh github-action: responsive player and playable delivery (614ms)
✓  14 tests/documentation-films.spec.js:84:3 › en vscode-cluster-panel: responsive player and playable delivery (559ms)
✓  15 tests/documentation-films.spec.js:84:3 › /zh vscode-cluster-panel: responsive player and playable delivery (573ms)
✓  16 tests/documentation-films.spec.js:84:3 › en accuracy-transparency: responsive player and playable delivery (570ms)
✓  17 tests/documentation-films.spec.js:84:3 › /zh accuracy-transparency: responsive player and playable delivery (590ms)
✓  18 tests/documentation-films.spec.js:84:3 › en research-background: responsive player and playable delivery (591ms)
✓  19 tests/documentation-films.spec.js:84:3 › /zh research-background: responsive player and playable delivery (590ms)
✓  20 tests/documentation-films.spec.js:84:3 › en for-ai: responsive player and playable delivery (617ms)
✓  21 tests/documentation-films.spec.js:84:3 › /zh for-ai: responsive player and playable delivery (624ms)
```
