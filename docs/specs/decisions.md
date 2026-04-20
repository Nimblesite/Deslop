# Decisions with fallback rules

### [DECISION-MIN-NODES] Minimum subtree size

Default `--min-nodes` = **30**. Subtrees below this threshold are excluded from fingerprinting, clustering, and embedding. Rationale: smaller subtrees (`return x;`, single-statement blocks) are noise and dominate the report. If the top-50 clusters on a real C# corpus are dominated by trivial fragments, raise the default to 40 before the next release. If large real duplicates are being missed, lower to 20. The flag is always user-overridable. Never ship a default below 15 or above 60.

### [DECISION-CROSS-LANGUAGE] Cross-language clones

Out of scope for v1. The normalization format is identical across languages so that a future cross-language pass can compare fingerprints directly without rework. Do not add heuristics, mappings, or type-system bridges until cross-language is an explicit feature goal.

### [DECISION-TYPE3-TWO-PASS] Type-3 recall via AST sibling-extension + token LSH

Ship both passes. Sibling-extension runs first because it is cheaper and produces byte-range-accurate matches. Token LSH runs second and surfaces Type-3 candidates whose structure diverged too far for sibling-extension. Fallback rule: if the LSH pass contributes fewer than 5% additional clusters on three consecutive representative corpora (measured in a calibration run), mark it as a future removal candidate and raise the issue — do not silently disable it.
