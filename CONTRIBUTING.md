# Contributing to Deslop

## We are in accuracy-auditing mode

Deslop is proven useful across nine languages. Accuracy is now the highest-value
aim — ahead of new features, new languages, UI work, and performance. Every
reported cluster must be a real duplicate, and every real duplicate must be
reported.

The top priority is fixing code that can or does cause inaccuracies: false
positives, false negatives, wrong buckets, unstable scores, stale generations.
That work outranks everything else on the roadmap.

## Issues

Please do log issues — especially if you can reproduce the bug or add a lot of
detail. Those are genuinely useful. A false positive or false negative with a
minimal snippet is the single most valuable report you can file.

## Pull requests

Code contributions are discouraged at the moment. We will only consider a pull
request that:

1. directly addresses an existing, confirmed bug, and
2. comes with several end-to-end tests proving the bug is thoroughly fixed.

Anything outside that is likely to be closed unmerged, so please open an issue
first rather than writing the patch.

## Building

Requires Rust 1.80+ and GNU Make.

```bash
make build
make test
make ci
```

Read [CLAUDE.md](CLAUDE.md) before contributing.
