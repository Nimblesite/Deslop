# Pair admission and mass-only clusters — wholesale cutover plan

This plan replaces the shipped cluster-evidence design in one cutover so code, tests, generated wire models, CLI, LSP, MCP, reports, and VSIX agree with [fused.md](../specs/fused.md). There is no compatibility stage, adapter, deprecated field, dual rendering path, fallback, or period in which the old model is preserved.

## Governing contract

- Structural similarity, token Jaccard, embedding similarity, content agreement, rename consistency, literal fraction, admission result, and pair classification belong to one exact pair.
- Candidate pairs are admitted under [FUSED-STRATEGY-BOUNDED-MAX](../specs/fused.md#fused-strategy-bounded-max). Clusters form from the transitive closure of admitted pairs.
- [CLONE-NOISE-VERBATIM-SUBGROUP](../specs/noise.md#clone-noise-verbatim-subgroup) is the only post-closure partition: a convicted component becomes its qualifying byte-identical families; an unconvicted component remains untouched.
- A clone cluster owns identity, canonical extent, occurrence membership, mass, rank, and mass-derived rank band. It owns no similarity evidence, pair classification, finding category, content verdict, or source-edge field.
- `mass = canonical_node_count × max(visible_occurrences - 1, 0)`. Sort by mass descending, then cluster id ascending. No multiplier or evidence tie-break exists.
- Pair evidence renders only after a caller explicitly identifies two distinct occurrences. The VSIX pair view uses a compact `PAIR EVIDENCE` surface; content evidence is a muted secondary line, never a cluster card.

## One destructive replacement

- [x] Cleanse the governing specifications so pair evidence, closure, convicted-noise handling, mass, wire ownership, presentation, metrics, and severity do not contradict one another.
- [ ] Replace the canonical wire model in `docs/models/live-ipc.td`: delete every cluster `signals`, `bucket`, `category`, `interpretation`, evidence-verdict, pair-source, and fused-gate field; add one explicit pair-comparison request/response keyed by two occurrence endpoints; regenerate Rust and TypeScript models once.
- [ ] Delete the engine path that stamps a cluster from any pair or aggregate. Remove all component means, per-axis maxima, edge selection, cluster content measurement, cluster classification, cluster confidence, evidence-weighted ranking, category multipliers, and structural-only multipliers.
- [ ] Make pair measurement the single owner of `S`, `J`, `E`, `A`, `R`, literal fraction, admission, and optional pair classification. Store or recompute the exact endpoint-keyed record without copying it into a component.
- [ ] Enforce closure directly from admitted edges. Retain only the exhaustive convicted-noise behavior from [CLONE-NOISE-VERBATIM-SUBGROUP]; delete every generic component repair, family fallback, silent drop, or control-flow panic.
- [ ] Replace report weighting wholesale with [RANK-MASS-SUM]. Delete every multiplier, boost, confidence factor, spanned-byte factor, and evidence tie-break. Equal mass sorts by cluster id.
- [ ] Replace text, Markdown, HTML, JSON, LSP, MCP cluster responses, AI context, site examples, and CLI summaries so cluster output contains membership and mass only. Delete neutralized helpers rather than leaving no-op shims.
- [ ] Replace VSIX cluster surfaces wholesale: bubble, hover, code lens, Top Offenders, cluster webview, report webview, tooltips, accessibility labels, copy-for-AI, history, and fixtures contain neutral cluster identity, membership, and mass only.
- [ ] Implement explicit VSIX pair selection and `compare-pair`. The response names both endpoints and returns only that pair's evidence. Render the three admission axes compactly and the content fields as a subtle secondary line; closing the pair view removes them from the surface.
- [ ] Delete cluster bucket/category facets and per-bucket severity configuration. Cluster severity derives only from the engine-stamped mass rank band. Cluster filters use language, path, and mass severity.
- [ ] Separate literal-family findings from clone closure components so literal kind cannot masquerade as pair classification or cluster evidence. Kept literal findings use unmodified mass.
- [ ] Delete evidence-weighted repository metrics, weight tables, weighted gate flags, weighted wire fields, configuration, renderers, and tests. The one repository duplication percentage remains unweighted line density.

## Assertions that must fail before the replacement and pass after it

- [ ] Black-box JSON, text, Markdown, and HTML tests assert exact cluster ids, occurrence paths and ranges, canonical node count, visible count, exact mass, and global order. They assert cluster records contain no pair-evidence or classification fields.
- [ ] Pair-comparison tests select two concrete endpoints and assert exact `S`, `J`, `E`, `A`, `R`, literal fraction, admission result, and pair classification. Reversing endpoint order preserves symmetric evidence while replacing either endpoint asks a different relation; a cluster id alone cannot request evidence.
- [ ] Closure tests assert the admitted edge set and exact connected components. Convicted-noise tests assert qualifying byte-identical families survive and outsiders drop; unconvicted components remain byte-for-byte unchanged.
- [ ] Ranking tests assert mass exactly, id-only tie-breaking, and invariance under every pair-evidence value, classification, language, path, and visibility configuration that does not change visible membership.
- [ ] LSP and MCP tests assert cluster payloads and messages contain membership plus mass only, while explicit pair responses identify both endpoints and contain pair evidence only.
- [ ] VSIX unit and Playwright tests assert cluster pages never render `PAIR EVIDENCE`, content agreement, rename consistency, literal fraction, structural, Jaccard, embedding, or pair labels. After an explicit two-occurrence Compare action, the separate compact pair view renders the exact endpoint labels and exact evidence values.
- [ ] Generated-model tests assert removed cluster fields do not exist in Rust or TypeScript and that hand-written mirror types cannot drift from the generated contract.
- [ ] Regression fixtures assert exact clusters, occurrences, paths, ranges, mass, and order across every affected language; no assertion is weakened to a cluster count.

## Whole-system proof

- [ ] Run formatting, lint, generated-model verification, Rust build, Rust tests with coverage, TypeScript typecheck, VSIX unit tests, Playwright webview smoke, packaging verification, and the full repository CI target with zero failures.
- [ ] Build and install the current VSIX artifact without killing VS Code. From the real extension UI, verify a cluster opens with membership and mass only, select two distinct occurrences, open Compare, verify compact pair evidence appears only there, edit a watched file, and verify the tree, bubble, cluster view, pair view, diagnostics, and mass refresh coherently.
- [ ] Re-run repository-wide searches for every removed cluster field, old command, old label, compatibility shim, multiplier, and weighted-metric surface. Only historical issue prose outside executable/spec contracts may remain.
- [ ] Run [spec-check](../../.agents/skills/spec-check/SKILL.md) and [ci-prep](../../.agents/skills/ci-prep/SKILL.md). Submit through [submit-pr](../../.agents/skills/submit-pr/SKILL.md) only after all gates pass.

## Completion

The plan is complete only when the source specs, generated schema, Rust model, pair admission, closure and suppression behavior, mass ranking, every renderer, every client, every assertion, and the installed VSIX UI all enforce the same boundary with no old path left in the repository.
