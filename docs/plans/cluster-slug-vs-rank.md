# Cluster Slug vs Rank — Stable Identity, Mutable Order

## Summary

Today the UI conflates two different things behind the symbol `#N`:

- **Rank** — the cluster's 1-based position in the worst-offenders list. Mutable across runs.
- **Cluster id** — a hash-derived string. Stable across runs.

Reports render rank in the slot a stable id should occupy, so user and AI references like "fix cluster #5" silently rebind to a different cluster on the next run. This plan separates the two everywhere they surface, adopting two named, never-conflated fields:

- **`rank`** — always rendered as the literal word `Rank` followed by the number. Never bare `#N`.
- **`slug`** — a 7-char hex prefix of the canonical cluster id, always rendered as the literal word `Slug` followed by the value. Universal id shown to humans and AI.

The full canonical id (the existing 16-char hex on `Cluster.id`) stays as the underlying primary key in the wire shape, deeplinks, and storage. The slug is a derived display form with collision-bump logic; the rank is computed at render time from sorted position. Neither concept changes the underlying ranking algorithm or hashing — this is a labeling and surface change.

## Definitions

- **Canonical id (`cluster.id`)** — Existing field. 16-char lowercase hex (first 8 bytes of the cluster hash via `encode_short_id`). Stable across runs for the same code. Stays the primary key in IPC, MCP responses, deeplink URIs, and storage.
- **Slug (`cluster.slug`)** — New field. Default 7-char lowercase hex prefix of `cluster.id`. If two clusters in the same report would collide on 7 chars, bump *both* colliding slugs to 8 chars; if still colliding, 9; and so on, up to the full 16 chars. Mirrors `git rev-parse --short`. Slug length within one report can vary per cluster.
- **Rank (`rank`)** — 1-based position in the rendered worst-offenders ordering. Computed at render time. Never persisted. Changes between runs.
- **Display labels** — Surfaces show `Rank 7` and `Slug ab3f9c2`. Never `#7`, never bare `ab3f9c2`, never `Cluster #7`, never `cluster_id:`-style raw key in human-readable surfaces.

## Key Changes

### Data shape

- Add a `slug: String` field to `ReportCluster` and to the slim `ClusterSummary` in `docs/models/live-ipc.td`. Regenerate `crates/deslop-core/src/wire_generated.rs` and `clients/vscode/src/types/wire-generated.ts`.
- Compute slug in the cluster aggregation stage, after the final cluster set is known, so collision-bump has a global view. Centralize in one helper in `crates/deslop-core/src/cluster.rs` (e.g. `assign_slugs(&mut [Cluster])`). One helper, called once.
- Do **not** add a `rank` field to the wire shape. Rank is render-time, derived from array position in the sorted output. Surfaces that need it compute it locally from the index. (This keeps the wire shape immutable under re-sorts and prevents anyone treating rank as identity in storage.)

### MCP id resolution

- MCP tools that take a cluster id parameter (`cluster-by-id`, `report-get` per-cluster lookups, etc., per `ClusterIdParams` in `live-ipc.td`) accept **either** the canonical id or any unambiguous prefix (the slug is just the most common case of a prefix). Resolution rule:
  - Exact match on `cluster.id` → resolved.
  - Otherwise, prefix match against all cluster ids in the active report. Exactly one match → resolved. Zero matches → not-found error. Multiple matches → ambiguous-slug error listing the candidates.
- Tool descriptions explicitly state: "accepts canonical cluster id or its slug prefix; if you got the slug from a Deslop surface it is always unambiguous within that report".

### Surfaces — Rust

- **`crates/deslop-core/src/cluster.rs`** — Add `slug` to `Cluster`, plus the `assign_slugs` helper with collision-bump. Unit-tested via existing E2E fixtures (no new unit tests; per CLAUDE.md, coarse E2E only).
- **`crates/deslop-core/src/report_render.rs`** — Carry `slug` into `ReportCluster` when copying.
- **`crates/deslop-lsp/src/presentation.rs`** — Replace `rank_prefix` returning `#{value} ` with a new `rank_label` returning `Rank {value} · `, and prepend `Slug {slug} · ` when a slug is available. Hover headline becomes e.g. `Rank 1 · Slug ab3f9c2 · {title} — {action}`.
- **`crates/deslop-lsp/src/presentation.rs::diagnostic_data`** — Keep `cluster_id` as the canonical id field name in the structured payload (this is machine-readable, not user-facing). Add a `slug` sibling field. Do not rename `cluster_id` — it is a stable contract for downstream consumers.
- **`crates/deslop/src/summary/body.rs`** — CLI table:
  - Rename the `id` column header to `slug` and emit the (possibly collision-bumped) slug, not a raw 8-char truncation of `cluster.id`. The full canonical id is no longer shown in the standard summary; add it under `--verbose` (or equivalent existing flag if present) only.
  - Rename the `rank` column to literally `Rank` (capitalized header) and render values as `Rank N` — the column doubles as both header and label so users copying a row get an unambiguous string.
  - Update the column legend line ("columns: rank, signal, id, copies, …") accordingly.

### Surfaces — VS Code extension

- **`clients/vscode/src/tree/nodes.ts::clusterRowLabel`** — Replace `#${rank} ${dot} ${title}` with `Rank ${rank} · Slug ${slug} · ${dot} ${title}`. Severity dot stays.
- **`clients/vscode/src/tree/nodes.ts::ClusterNode`** — Tooltip currently says ``cluster id: `${cluster.id}` ``. Change to two lines: ``Slug: `${slug}` `` and ``Cluster id: `${cluster.id}` `` (full id retained for copy/debug). Accessibility label uses `Rank ${rank}, Slug ${slug}, ${title}`.
- **`clients/vscode/src/bubble/live.ts`** — Inline bubble, ghost text, and hover all swap `#${rank} ` prefix for `Rank ${rank} · Slug ${slug} · `. Bubble hover Markdown gains an explicit "Slug" / "Rank" line at the top so the meaning is unambiguous on hover.
- **`clients/vscode/src/clusterHover.ts`** — Same prefix change. The "View cluster" deeplink keeps using the canonical `cluster.id` in the URI argument (stable, no collision risk).
- **`clients/vscode/src/clusterDocument.ts`** — Continue resolving `deslop://cluster/{id}` against the canonical id. Optionally accept a slug as well (using the same prefix-resolution rule as MCP) so users pasting a slug from elsewhere get a useful result; ambiguous → error page listing candidates.
- **`clients/vscode/webview-ui/src/store.ts`** — `severityByClusterId` keeps its current rank-position-based color mapping; rename internal local variable `rank` to keep the meaning honest, no UX change. The visible rank in the webview header gets the `Rank N · Slug abcdef0` treatment.
- **`clients/vscode/src/webview/panels.ts`** and any header rendering in `webview-ui` — Cluster detail panel headline becomes `Rank N · Slug abcdef0 · {title}`.

### Copy-for-AI payloads

- **`clients/vscode/src/commands/treeMenus.ts::aiPayloadForCluster`** — Header keys become:
  - `slug: ab3f9c2`
  - `cluster_id: ab3f9c2def012345` (canonical, full)
  - `rank: 7`
  Order: slug first (the universal id), canonical id second (for tools that round-trip), rank last (volatile, informational).
- **`clients/vscode/src/commands/treeMenus.ts::parentClusterLines`** — Same three-line header.
- **`clients/vscode/src/commands/treeMenus.ts::clusterLocationsText`** — Header changes from `cluster ${cluster.id} · ${bucket} · …` to `Slug ${slug} · Rank ${rank} · ${bucket} · …`.

### Site / docs

- **`site/src/docs/output-formats.md`** — Document `slug` (definition, length, collision rule), `cluster_id` (canonical, stable across runs), and `rank` (volatile, render-time position). Add a short "Why three?" paragraph: rank for human triage order, slug for casual reference, canonical id for unambiguous tooling.
- **`docs/snippets/agents-md-recipe.md`** — Update any example MCP responses or copy-for-AI snippets to show the new header.

## Test Plan

All test changes are E2E and assertion updates per CLAUDE.md (no unit tests, coarse only).

- **VSIX tree-view tests** (`clients/vscode/src/test/unit/tree.topOffenders.unit.test.ts`) — Replace `/#1\b/` / `/#2\b/` patterns with `/\bRank 1\b/` and add a parallel `/\bSlug [0-9a-f]{7,}\b/` assertion on the same row to prove both labels render.
- **Bubble tests** (`clients/vscode/src/test/unit/bubble.unit.test.ts`) — Replace `/#42/` and `/^\*\*#42 /` patterns with `/\bRank 42\b/` and `/\bSlug [0-9a-f]{7,}\b/` assertions; assert both labels appear in the bubble hover Markdown.
- **Copy-for-AI tests** (`clients/vscode/src/test/unit/command-impls.unit.test.ts`) — Update `/rank: 7/` style assertions to also assert `/^slug: [0-9a-f]{7,}$/m` and `/^cluster_id: [0-9a-f]{16}$/m` lines are present in that order.
- **Context-menu E2E** (`clients/vscode/src/test/suite/context-menus.e2e.test.ts`) — Same payload-shape assertion update.
- **New E2E coverage**:
  - Slug collision-bump: a fixture (synthetic or crafted) that produces two cluster ids sharing a 7-char prefix; assert both render with an 8-char slug, neither with 7.
  - MCP slug resolution: `cluster-by-id` accepts a slug, accepts the canonical id, errors on a non-matching prefix, errors with a candidate list on an ambiguous prefix.
  - Stable-across-runs: parse the report twice from the same fixture and assert canonical ids and slugs match between runs while ranks may differ when the fixture is mutated to reorder severity.
- **Forbid the old format**: add a project-wide assertion in one E2E test that scans rendered CLI output and the VSIX tree labels for the bare patterns `/Cluster #\d/` and `/^#\d/` — none must match anywhere in human-facing surfaces. (Keeps regressions out.)
- Coverage threshold in `coverage-thresholds.json` ratchets only upward — do not lower it.

## Out of Scope

- No change to the hashing algorithm or the canonical id length on the wire (`cluster.id` stays 16 hex chars).
- No change to the ranking score or sort order. Only the label changes.
- No persistence of rank. Rank stays computed at render time.
- No migration of existing user references to old `#N` labels — those references were already broken by definition; no compatibility shim.
- No rename of the `cluster_id` field in machine-readable structured payloads (`diagnostic_data`, MCP responses). The contract name stays; only human-rendered surfaces change.

## Assumptions

- 7 hex chars (28 bits) is sufficient for typical reports (a few hundred to low thousands of clusters); collision bump handles the rest. Same assumption git relies on at much larger scale.
- The full canonical id remains acceptable as the deeplink key; no need to make slugs round-trippable as URIs.
- Webview rank-percentile severity coloring is independent of label format and unaffected.
- No breaking change to the typeDiagram schema beyond adding fields. Adding fields is a non-breaking schema evolution per the existing `live-ipc.td` patterns.

## TODO

- [ ] Add `slug: String` to `ReportCluster` and `ClusterSummary` in [docs/models/live-ipc.td](docs/models/live-ipc.td); regenerate Rust and TS wire types.
- [ ] Implement `assign_slugs` (7-char default, collision-bump up to 16) in [crates/deslop-core/src/cluster.rs](crates/deslop-core/src/cluster.rs) and call it from the cluster aggregation stage so every emitted cluster carries a slug.
- [ ] Carry `slug` through `cluster_to_report` in [crates/deslop-core/src/report_render.rs](crates/deslop-core/src/report_render.rs) and the slim summary builder in [crates/deslop-mcp/src/page.rs](crates/deslop-mcp/src/page.rs).
- [ ] Replace `rank_prefix` with explicit `Rank N · Slug abcdef0 · ` rendering in [crates/deslop-lsp/src/presentation.rs](crates/deslop-lsp/src/presentation.rs); add `slug` alongside `cluster_id` in `diagnostic_data` without renaming `cluster_id`.
- [ ] Update CLI summary header and per-row format in [crates/deslop/src/summary/body.rs](crates/deslop/src/summary/body.rs): emit `Rank N` and `Slug abcdef0`; drop the raw 8-char id truncation from the standard view.
- [ ] Update tree row label, tooltip, and accessibility label in [clients/vscode/src/tree/nodes.ts](clients/vscode/src/tree/nodes.ts) to surface both `Rank` and `Slug` and keep the canonical id in the tooltip only.
- [ ] Update inline bubble, ghost text, and hover Markdown in [clients/vscode/src/bubble/live.ts](clients/vscode/src/bubble/live.ts) to the `Rank N · Slug abcdef0 · …` format.
- [ ] Update cluster hover headline in [clients/vscode/src/clusterHover.ts](clients/vscode/src/clusterHover.ts) and webview cluster detail header in [clients/vscode/src/webview/panels.ts](clients/vscode/src/webview/panels.ts) and [clients/vscode/webview-ui/src/store.ts](clients/vscode/webview-ui/src/store.ts) to the same format.
- [ ] Update copy-for-AI clipboard headers in [clients/vscode/src/commands/treeMenus.ts](clients/vscode/src/commands/treeMenus.ts) (`aiPayloadForCluster`, `parentClusterLines`, `clusterLocationsText`) so the first lines are `slug:`, `cluster_id:`, `rank:` in that order.
- [ ] Make MCP tools that take a cluster id (`cluster-by-id`, etc.) in [crates/deslop-mcp/src/tools/handlers.rs](crates/deslop-mcp/src/tools/handlers.rs) accept canonical id or slug prefix; return ambiguous-slug error with candidate list on multiple prefix matches; update tool descriptions.
- [ ] Make `deslop://cluster/{id}` document provider in [clients/vscode/src/clusterDocument.ts](clients/vscode/src/clusterDocument.ts) accept either the canonical id or a slug prefix using the same resolution rule.
- [ ] Document `slug`, `cluster_id`, and `rank` semantics in [site/src/docs/output-formats.md](site/src/docs/output-formats.md) and any agent recipe in [docs/snippets/agents-md-recipe.md](docs/snippets/agents-md-recipe.md).
- [ ] Update VSIX tree-view tests in [clients/vscode/src/test/unit/tree.topOffenders.unit.test.ts](clients/vscode/src/test/unit/tree.topOffenders.unit.test.ts) to assert `Rank N` and `Slug abcdef0` instead of `#N`.
- [ ] Update bubble tests in [clients/vscode/src/test/unit/bubble.unit.test.ts](clients/vscode/src/test/unit/bubble.unit.test.ts) to assert both labels render and the old `#N` pattern is absent.
- [ ] Update copy-for-AI assertions in [clients/vscode/src/test/unit/command-impls.unit.test.ts](clients/vscode/src/test/unit/command-impls.unit.test.ts) and [clients/vscode/src/test/suite/context-menus.e2e.test.ts](clients/vscode/src/test/suite/context-menus.e2e.test.ts) to require `slug:`, `cluster_id:`, `rank:` lines in order.
- [ ] Add an E2E test covering slug collision-bump (two clusters sharing a 7-char prefix render at 8+ chars).
- [ ] Add an E2E test covering MCP id resolution for canonical id, slug prefix, non-match, and ambiguous match.
- [ ] Add an E2E test asserting that `/Cluster #\d/` and bare `/^#\d/` patterns appear nowhere in CLI output or VSIX tree labels.
- [ ] Run `make test` and `make lint`; ratchet `coverage-thresholds.json` upward only.
