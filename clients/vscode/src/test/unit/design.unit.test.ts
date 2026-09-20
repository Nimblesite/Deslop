// Unit tests for design tokens — smoke-test that every export is well-formed,
// and pin the one paint table every cluster surface reads ([CLONE-KIND-COLOR]).

import * as assert from "node:assert/strict";
import {
  COLOR,
  KIND_COLOR,
  KIND_ICON,
  KIND_THEME_COLOR,
  SEVERITY_DOT,
  FONT,
  RADIUS,
  SPACING,
  TYPE,
  SHADOW,
} from "../../design";
import { CLUSTER_KINDS, SEVERITIES, type ClusterKind } from "../../types/report";

const HEX_COLOR = /^#[0-9a-fA-F]{3,8}$/;

suite("design tokens", () => {
  test("every color is a hex or rgba string", () => {
    for (const [k, v] of Object.entries(COLOR)) {
      assert.match(v, /^(#[0-9a-fA-F]{3,8}|rgba?\([^)]+\))$/, `${k}=${v}`);
    }
  });

  test("KIND_COLOR paints every clone kind with a distinct hex token", () => {
    // [CLONE-KIND-COLOR] The paint map. Every kind must exist and no two
    // may share a token, or two kinds become indistinguishable on screen.
    const tokens = CLUSTER_KINDS.map((kind) => KIND_COLOR[kind]);
    for (const [index, kind] of CLUSTER_KINDS.entries()) {
      assert.ok(tokens[index], `${kind} has no colour token`);
      assert.match(String(tokens[index]), HEX_COLOR, `${kind} is not a hex token`);
    }
    assert.equal(new Set(tokens).size, CLUSTER_KINDS.length, "kinds must not share a token");
  });

  test("every kind wears the colour the taxonomy names for it", () => {
    // [CLONE-KIND-COLOR] "Use crimson for Identical, amber for Nearly
    // identical, violet for Same behavior, blue for Similar, and muted
    // grey for shape-only." All five, so a later token rename cannot
    // quietly reassign one category's colour to another's.
    const EXPECTED_PAINT: readonly (readonly [ClusterKind, string, string])[] = [
      ["identical", COLOR.primaryContainer, "crimson — byte-identical code, the only kind proven character by character"],
      ["nearly_identical", COLOR.amber, "amber — a near-copy, renamed or lightly edited"],
      ["same_behavior", COLOR.violet, "violet — equivalent behaviour reached by different code"],
      ["loosely_similar", COLOR.tertiary, "blue — an established copy with larger edits"],
      ["structural_only", COLOR.onSurfaceMuted, "muted grey — shape only, and not a clone however high it ranks"],
    ];
    assert.equal(
      EXPECTED_PAINT.length,
      CLUSTER_KINDS.length,
      "the taxonomy names a colour for every category",
    );
    for (const [kind, token, rationale] of EXPECTED_PAINT) {
      assert.equal(KIND_COLOR[kind], token, rationale);
    }
    assert.deepEqual(
      EXPECTED_PAINT.map(([kind]) => kind),
      [...CLUSTER_KINDS],
      "the colour list follows the category display order ([CLONE-KIND-LABELS])",
    );
  });

  test("KIND_ICON and KIND_THEME_COLOR cover every kind, distinctly", () => {
    for (const kind of CLUSTER_KINDS) {
      assert.ok(KIND_ICON[kind], `${kind} has no icon`);
      assert.match(KIND_THEME_COLOR[kind], /^deslop\.kind\./, `${kind} must paint from a contributed deslop.kind.* colour`);
    }
    assert.equal(new Set(Object.values(KIND_ICON)).size, CLUSTER_KINDS.length, "icons must be distinct");
    assert.equal(new Set(Object.values(KIND_THEME_COLOR)).size, CLUSTER_KINDS.length, "theme ids must be distinct");
  });

  test("SEVERITY_DOT gives every diagnostic level its own distinct glyph", () => {
    // [SEVERITY-DIAGNOSTICS] The five levels a finding may carry. A glyph
    // per level, all different, so severity reads without colour — and
    // `none` is a level with a mark of its own, not a blank.
    const glyphs = SEVERITIES.map((severity) => SEVERITY_DOT[severity]);
    for (const [index, severity] of SEVERITIES.entries()) {
      assert.ok(glyphs[index], `${severity} has no glyph`);
    }
    assert.equal(new Set(glyphs).size, SEVERITIES.length, `levels must not share a glyph: ${glyphs.join()}`);
    assert.equal(
      Object.keys(SEVERITY_DOT).length,
      SEVERITIES.length,
      "one glyph per level, and no glyph for a level that does not exist",
    );
  });

  test("FONT has ui and mono stacks", () => {
    assert.match(FONT.ui, /Inter/);
    assert.match(FONT.mono, /JetBrains Mono/);
  });

  test("RADIUS has sm + none only", () => {
    assert.equal(RADIUS.none, "0");
    assert.equal(RADIUS.sm, "2px");
  });

  test("SPACING keys", () => {
    assert.ok(SPACING.xs && SPACING.sm && SPACING.md && SPACING.lg && SPACING.xl);
  });

  test("TYPE scales exist", () => {
    assert.ok(TYPE.displayLg && TYPE.displayMd && TYPE.headline && TYPE.bodyMd);
    assert.ok(TYPE.labelMd && TYPE.labelSm);
  });

  test("SHADOW.float uses tinted shadow, not pure black", () => {
    assert.match(SHADOW.float, /rgba\(14, 14, 14/);
  });
});
