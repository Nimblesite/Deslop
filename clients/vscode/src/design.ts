// Kinetic Manuscript design tokens — single source of truth for every VSIX surface.
// Mirrors docs/designs/designsystem.md. Change tokens here, never inline colors.

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

  onSurface: "#ece0dd",
  onSurfaceMuted: "#a9a2a0",
  ghostBorder: "rgba(90, 64, 61, 0.2)",
} as const;

// Percentile ramp per [LSP-SEVERITY-PERCENTILE]. Drives the *filter facet*
// and the webview badge, which both sort by impact. It is NOT the paint:
// [SEVERITY-COLOR] gives colour to the bucket, and using this ramp to paint
// an editor surface is what made rank-1 shape-only scaffolding crimson.
export const SEVERITY_COLOR = {
  worst: COLOR.primaryContainer,
  top10: COLOR.primary,
  mid: COLOR.tertiary,
  faint: COLOR.onSurfaceMuted,
} as const;

// [SEVERITY-COLOR] The paint. Keyed by the Deslop severity level, which is a
// function of the bucket alone ([SEVERITY-DESLOP-MAP]), so an occurrence is
// coloured by what kind of duplicate it is and never by where it happens to
// sort. Same four tokens as the percentile ramp — crimson is still the
// surgical tool — but earned by evidence rather than by position.
export const DESLOP_SEVERITY_COLOR = {
  error: COLOR.primaryContainer,
  warning: COLOR.primary,
  information: COLOR.tertiary,
  hint: COLOR.onSurfaceMuted,
} as const;

// Glyph density per [SEVERITY-COLOR]: the weight-percentile channel.
export const SEVERITY_DOT = {
  worst: "●●",
  top10: "●",
  mid: "◐",
  faint: "○",
} as const;

export const FONT = {
  ui: "Inter, ui-sans-serif, system-ui, -apple-system, Segoe UI, sans-serif",
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
