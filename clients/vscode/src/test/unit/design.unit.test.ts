// Unit tests for design tokens — smoke-test that every export is well-formed.

import * as assert from "node:assert/strict";
import {
  COLOR,
  DESLOP_SEVERITY_COLOR,
  SEVERITY_COLOR,
  SEVERITY_DOT,
  FONT,
  RADIUS,
  SPACING,
  TYPE,
  SHADOW,
} from "../../design";
import { DESLOP_SEVERITIES } from "../../types/report";

suite("design tokens", () => {
  test("every color is a hex or rgba string", () => {
    for (const [k, v] of Object.entries(COLOR)) {
      assert.match(v, /^(#[0-9a-fA-F]{3,8}|rgba?\([^)]+\))$/, `${k}=${v}`);
    }
  });

  test("SEVERITY_COLOR covers every bucket", () => {
    assert.ok(SEVERITY_COLOR.worst);
    assert.ok(SEVERITY_COLOR.top10);
    assert.ok(SEVERITY_COLOR.mid);
    assert.ok(SEVERITY_COLOR.faint);
  });

  test("DESLOP_SEVERITY_COLOR covers every level and is a distinct token each", () => {
    // [SEVERITY-COLOR] The paint map. Every level must exist and no two may
    // share a token, or two buckets become indistinguishable on screen.
    const tokens = DESLOP_SEVERITIES.map((level) => DESLOP_SEVERITY_COLOR[level]);
    for (const [index, level] of DESLOP_SEVERITIES.entries()) {
      assert.ok(tokens[index], `${level} has no colour token`);
      assert.match(String(tokens[index]), /^#[0-9a-fA-F]{3,8}$/, `${level} is not a hex token`);
    }
    assert.equal(new Set(tokens).size, DESLOP_SEVERITIES.length, "levels must not share a token");
    assert.equal(
      DESLOP_SEVERITY_COLOR.error,
      COLOR.primaryContainer,
      "crimson is reserved for the byte-proven bucket",
    );
  });

  test("SEVERITY_DOT covers every bucket", () => {
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
