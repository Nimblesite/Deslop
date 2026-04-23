# Taxonomy Content Cleanup Plan

## Scope

Finish the content side of [CLONE-BUCKETS](../specs/taxonomy.md) after the
code-side bucket work landed.

The Rust core, CLI summary, HTML report, MCP responses, and VSIX type helpers
already route through canonical bucket labels. Remaining work is public copy
and examples that still lead with `Type-N` language where humans are the
primary reader.

## Cleanup Rules

- Public human-facing copy should be bucket-first:
  `Identical code`, `Nearly identical code`, `Loosely similar code`, and
  `Same behavior, different code`.
- Academic `Type-N` terms are fine in specs, research docs, and AI-only schema
  context.
- Shared or educational docs may keep `Type-N` in parentheses after the bucket
  name when it helps readers connect to the literature.
- Do not edit generated `site/_site/**` directly; rebuild the site from
  `site/src/**`.

## Known Remaining Areas

- `site/src/docs/how-it-works.md`
- `site/src/docs/output-formats.md`
- `site/src/docs/ai-integration.md`
- `site/src/docs/index.md`
- `site/src/index.njk`
- `site/src/blog/ai-era-duplication.md`
- `site/src/blog/ranking-formula.md`
- `examples/README.md`
- Example source comments under `examples/**` where the comment is intended as
  user-facing sample guidance.

## TODO

- [ ] Update site docs to lead with canonical bucket names and use `Type-N`
      only as secondary context.
- [ ] Update the homepage sample cluster copy in `site/src/index.njk`.
- [ ] Update blog posts where `Type-N` appears in product-facing prose rather
      than research explanation.
- [ ] Update `examples/README.md` tables to use bucket-first descriptions.
- [ ] Review example source comments and keep only comments that are useful
      sample guidance.
- [ ] Rebuild the site so `site/_site/**` reflects source changes.
- [ ] Ripgrep `site/src` and `examples` for `Type-1`, `Type-2`, `Type-3`,
      `Type-4`, `near-miss`, `exact clone`, `semantic clone`, and `LSH-only`;
      verify every remaining hit is either research-facing or bucket-first.
