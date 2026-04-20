# Clone Type Taxonomy

### [CLONE-TYPE-TAXONOMY] Ground rules

- **Type-1** — identical code, ignoring whitespace/comments.
- **Type-2** — identical up to renaming of identifiers/literals/types.
- **Type-3** — Type-2 + added/removed/modified statements ("near-miss" clones).
- **Type-4** — semantically equivalent, syntactically different (same behavior, different structure/algorithm).

Recent work reframes Type-4 specifically as *"code segments deliver identical functionality through syntactically distinct implementations, such as differing algorithmic approaches or data structure choices that yield substantially varied program structures."* ([PMC — Semantic code clone detection via hybrid IR + BiLSTM, 2025](https://pmc.ncbi.nlm.nih.gov/articles/PMC12818651/))

**Implication for CodeDedup:** Types 1–3 are the sweet spot for a deterministic static tool. Type-4 is expensive, noisy, and only reliably solved today with LLMs + execution-based validation — treat it as out-of-scope for v1.
