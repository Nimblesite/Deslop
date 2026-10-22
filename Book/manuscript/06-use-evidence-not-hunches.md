# Chapter 6 — Use evidence to decide what to do

Use a group's kind to understand the relationship and its rank to choose what to inspect. Neither proves that a merge preserves behavior.

## Compare a specific pair

`compare-pair` takes `left` and `right` endpoints. Each names `path`, `start_byte`, and `end_byte`. It returns evidence for those two ranges, including text identity, content and rename evidence, admission decision, and any classification.

Clusters do not carry a selected pair's similarity measurements. A fused admission score belongs to the pair comparison; it is not a universal refactoring threshold. See the [MCP specification](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/mcp.md).

## Treat locations as a snapshot

Byte offsets identify exact ranges in the analysed source. An edit before a range can shift its offsets as well as its line numbers. Refresh after editing before reusing either.

A group ID stays stable while the cloned code is unchanged. Editing that code can change the ID. Track the affected files and ranges too; a missing old ID alone does not prove cleanup.

## Read the percentage's scope

`metrics.duplication_percent` is duplicated source lines divided by analysed source lines, using the clone kinds listed in `metrics.total_includes`. By default, the total includes Identical and Nearly identical code. Shape-only findings never count.

Filtering a `duplicates` page changes the returned cluster list, not the whole-repository metrics. A lower percentage can also come from a larger analysed denominator. It is not a measure of detector accuracy.

**Try it:** record one group's ID, kind, mass, source ranges, and the total's included kinds. Compare two occurrences before deciding what to share.
