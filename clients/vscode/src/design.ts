// Kinetic Manuscript design tokens — single source of truth for every VSIX surface.
// Mirrors docs/designs/designsystem.md. Change tokens here, never inline colors.

import type { ClusterKind } from "./types/wire-generated";

export const COLOR = {
  surface: "#131313",
  surfaceContainerLowest: "#0e0e0e",
  surfaceContainerLow: "#1a1a1a",
  surfaceContainer: "#1f1f1f",
  surfaceContainerHigh: "#2a2a2a",
  surfaceContainerHighest: "#353534",

  primary: "#ffb4aa",
  primaryContainer: "#b3261e",
  onPrimaryContainer: "#ffdad4",

  secondaryContainer: "#474746",

  tertiary: "#00619e",
  tertiaryContainer: "#003c6b",

  errorContainer: "#93000a",

  amber: "#e8912d",
  violet: "#a98cff",

  onSurface: "#ece0dd",
  onSurfaceMuted: "#a9a2a0",
  ghostBorder: "rgba(90, 64, 61, 0.2)",
} as const;

// [CLONE-KIND-COLOR] The one paint table. Colour follows the cluster's
// clone kind ([CLONE-KIND-FOLD]) and nothing else: crimson is the
// byte-proven copy, and a cluster that only shares shape is muted however
// high it ranks. Every surface — tree icon, bubble, editor underline,
// webview badge, HTML report — reads this table; the tree paints through
// the contributed `KIND_THEME_COLOR` ids whose package.json defaults carry
// the same values, and the HTML report declares the same values as its
// `--kind-*` variables. `kind.unit.test.ts` holds the copies together.
export const KIND_COLOR: Record<ClusterKind, string> = {
  identical: COLOR.primaryContainer,
  nearly_identical: COLOR.amber,
  same_behavior: COLOR.violet,
  structural_only: COLOR.onSurfaceMuted,
  loosely_similar: COLOR.tertiary,
} as const;

// [CLONE-KIND-COLOR] The theme colour ids the tree paints kind icons with.
// package.json contributes each id with the matching KIND_COLOR value as
// its default, so a themed icon and a hex-painted surface show one colour.
export const KIND_THEME_COLOR: Record<ClusterKind, string> = {
  identical: "deslop.kind.identical",
  nearly_identical: "deslop.kind.nearlyIdentical",
  same_behavior: "deslop.kind.sameBehavior",
  structural_only: "deslop.kind.structuralOnly",
  loosely_similar: "deslop.kind.looselySimilar",
} as const;

// [CLONE-KIND-COLOR] One codicon per kind, so kinds stay distinct even
// where colour is unavailable.
export const KIND_ICON: Record<ClusterKind, string> = {
  identical: "circle-filled",
  nearly_identical: "circle-large-filled",
  same_behavior: "sparkle",
  structural_only: "circle-slash",
  loosely_similar: "circle-outline",
} as const;

// [SEVERITY-BAND] Glyph density is the mass rank band's channel — how
// much of the repository's duplication this cluster is — orthogonal to the
// kind colour.
export const SEVERITY_DOT = {
  worst: "●●",
  top10: "●",
  mid: "◐",
  faint: "○",
} as const;

export const FONT = {
  ui: "Inter, ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
  mono: "'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace",
} as const;

export const RADIUS = {
  none: "0",
  sm: "2px",
} as const;

export const SPACING = {
  xs: "4px",
  sm: "8px",
  md: "16px",
  lg: "24px",
  xl: "32px",
} as const;

// Typographic scale per §3 of the design system.
export const TYPE = {
  displayLg: "font: 700 3.5rem/1.05 " + FONT.ui + "; letter-spacing: -0.03em;",
  displayMd: "font: 700 2.25rem/1.1 " + FONT.ui + "; letter-spacing: -0.02em;",
  headline: "font: 600 1.25rem/1.2 " + FONT.ui + ";",
  bodyMd: "font: 400 0.875rem/1.4 " + FONT.ui + ";",
  labelMd: "font: 500 0.75rem/1.2 " + FONT.mono + "; letter-spacing: 0.04em;",
  labelSm: "font: 500 0.6875rem/1.2 " + FONT.mono + "; letter-spacing: 0.04em;",
} as const;

// Ambient shadow per §4: tinted, never pure black.
export const SHADOW = {
  float: "0 20px 40px rgba(14, 14, 14, 0.6)",
} as const;
