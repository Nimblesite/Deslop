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
import { CLUSTER_KINDS } from "../../types/report";

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
    assert.equal(
      KIND_COLOR.identical,
      COLOR.primaryContainer,
      "crimson is reserved for byte-identical code — the only kind whose evidence is character-by-character proof",
    );
    assert.equal(
      KIND_COLOR.structural_only,
      COLOR.onSurfaceMuted,
      "a cluster that only shares shape is muted, however high it ranks",
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

  test("SEVERITY_DOT covers every rank band", () => {
    assert.ok(SEVERITY_DOT.worst);
    assert.ok(SEVERITY_DOT.top10);
    assert.ok(SEVERITY_DOT.mid);
    assert.ok(SEVERITY_DOT.faint);
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
