# VSIX coverage verification

Command: `env -u RUST_LOG npm run coverage:extension` (clients/vscode). Exit status: **0**.

Source: `9f7afeb41d605bacf428db1a296e7caf0a0043e2`.

Test totals read from runner output: 588, 5, 1; combined **594 passing**.

## extension

```json
{
  "lines": {
    "total": 3264,
    "covered": 2918,
    "skipped": 0,
    "pct": 89.39
  },
  "statements": {
    "total": 3715,
    "covered": 3225,
    "skipped": 0,
    "pct": 86.81
  },
  "functions": {
    "total": 896,
    "covered": 731,
    "skipped": 0,
    "pct": 81.58
  },
  "branches": {
    "total": 2286,
    "covered": 1734,
    "skipped": 0,
    "pct": 75.85
  },
  "branchesTrue": {
    "total": 0,
    "covered": 0,
    "skipped": 0,
    "pct": "Unknown"
  }
}
```
## webview

```json
{
  "lines": {
    "total": 1258,
    "covered": 1248,
    "skipped": 0,
    "pct": 99.2
  },
  "statements": {
    "total": 1258,
    "covered": 1248,
    "skipped": 0,
    "pct": 99.2
  },
  "functions": {
    "total": 78,
    "covered": 69,
    "skipped": 0,
    "pct": 88.46
  },
  "branches": {
    "total": 255,
    "covered": 225,
    "skipped": 0,
    "pct": 88.23
  },
  "branchesTrue": {
    "total": 0,
    "covered": 0,
    "skipped": 0,
    "pct": "Unknown"
  }
}
```

## Threshold gate

Command: `make _vsix-coverage-check`. Exit status: **0**.

```text
cd clients/vscode && npm run coverage:extension:check
npm notice run deslop-live@0.0.0-dev coverage:extension:check
npm notice run node ./scripts/check-coverage.mjs --extension
Extension-host line coverage: 89.4% (threshold: 88% + 1% slack)
OK: 89.4% + 1% slack >= 88%
cd clients/vscode && npm run coverage:webview:check
npm notice run deslop-live@0.0.0-dev coverage:webview:check
npm notice run node ./scripts/check-coverage.mjs --webview
Webview line coverage: 99.2% (threshold: 95% + 1% slack)
OK: 99.2% + 1% slack >= 95%

```
