# CLI stable identifier investigation

Actual release-profile CLI run over the rank-mass fixture, then with a third exact big-family copy added. Both scans exit0, with embeddings disabled and one worker.

```json
{
  "before": [
    {
      "id": "02bb5e80ee7e2d96",
      "rank": 1,
      "mass": 3792,
      "occurrence_count": 5
    },
    {
      "id": "2a6f2840074b7094",
      "rank": 2,
      "mass": 3513,
      "occurrence_count": 2
    }
  ],
  "after": [
    {
      "id": "018c88ec626d3c5e",
      "rank": 1,
      "mass": 7026,
      "occurrence_count": 3
    },
    {
      "id": "02bb5e80ee7e2d96",
      "rank": 2,
      "mass": 3792,
      "occurrence_count": 5
    }
  ]
}
```

Original terminal summary:

```text
deslop scanning /Users/christianfindlay/Documents/Code/Deslop/target/ci-prep/slug-ovmj3s5t/source for duplicated code...
  report → /Users/christianfindlay/Documents/Code/Deslop/target/ci-prep/slug-ovmj3s5t/report.{json,txt,html}
  log    → /Users/christianfindlay/Documents/Code/Deslop/target/ci-prep/slug-ovmj3s5t/logs/deslop-1794269790.log

Found 2 groups of duplicated code across 7 file(s)
  diagnostic severity: 2 × warning
  Highest mass: 3792 from 5 occurrences × 948 canonical nodes in mid_0.ts, mid_1.ts, mid_2.ts (+2 more)

Top 2 findings
  #1  Nearly identical code — mass 3792, 5 occurrences in mid_0.ts, mid_1.ts, mid_2.ts (+2 more)
  #2  Identical code — mass 3513, 2 occurrences in big_a.ts, big_b.ts

Next: open the .html report to inspect occurrences or explicitly compare two endpoints.

✔ done
    report /Users/christianfindlay/Documents/Code/Deslop/target/ci-prep/slug-ovmj3s5t/report.json
    report /Users/christianfindlay/Documents/Code/Deslop/target/ci-prep/slug-ovmj3s5t/report.txt
    report /Users/christianfindlay/Documents/Code/Deslop/target/ci-prep/slug-ovmj3s5t/report.html
    log    /Users/christianfindlay/Documents/Code/Deslop/target/ci-prep/slug-ovmj3s5t/logs/deslop-1794269790.log

```
