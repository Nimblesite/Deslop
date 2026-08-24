# Repository scripts

Scripts are grouped by the surface they support. Keep executable logic out of this directory root; new tools belong in the narrowest matching folder.

| Folder | Purpose |
|---|---|
| `actions/` | Composite GitHub Action runtime helpers and their contract tests |
| `benchmarks/` | Manually invoked real-repository benchmarks |
| `corpus/` | Corpus acquisition and verification |
| `deployment/` | Package, binary, manifest, installer, and documentation verification |
| `issues/` | Typed GitHub issue-report generation and tests |
| `release/` | Version stamping, change classification, and release-workflow contracts |
| `repository/` | Repository-wide content and duplication gates |
| `typediagram/` | TypeDiagram generation and post-processing |

The Makefile is the public entry point for CI-facing scripts. Scripts called directly by workflows use their full domain path so path changes fail visibly.
