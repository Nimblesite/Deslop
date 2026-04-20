// Mirror of clients/vscode/src/design.ts — the webview cannot import Node
// code, so we duplicate the token values and keep both sides updated together.
// If you change one, change both; a lint rule enforces parity in CI.

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
  tertiary: "#00619e",
  tertiaryContainer: "#003c6b",
  errorContainer: "#93000a",
  onSurface: "#ece0dd",
  onSurfaceMuted: "#a9a2a0",
  ghostBorder: "rgba(90, 64, 61, 0.2)",
} as const;

export const SEVERITY_COLOR = {
  worst: COLOR.primaryContainer,
  top10: COLOR.primary,
  mid: COLOR.tertiary,
  faint: COLOR.onSurfaceMuted,
} as const;

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

export const GLOBAL_CSS = `
  :root { color-scheme: dark; }
  * { box-sizing: border-box; }
  html, body {
    margin: 0; padding: 0;
    background: ${COLOR.surface};
    color: ${COLOR.onSurface};
    font-family: ${FONT.ui};
    font-size: 14px;
    line-height: 1.4;
  }
  button {
    font-family: inherit;
    border: 0;
    background: ${COLOR.surfaceContainerHighest};
    color: ${COLOR.onSurface};
    padding: 6px 14px;
    border-radius: 2px;
    cursor: pointer;
    letter-spacing: 0.02em;
  }
  button.primary {
    background: linear-gradient(180deg, ${COLOR.primary} 0%, ${COLOR.primaryContainer} 100%);
    color: ${COLOR.onPrimaryContainer};
    font-weight: 600;
  }
  button:hover { filter: brightness(1.1); }
  input, select {
    background: ${COLOR.surfaceContainerLowest};
    color: ${COLOR.onSurface};
    border: 0;
    padding: 6px 8px;
    font-family: ${FONT.mono};
    font-size: 12px;
    border-radius: 2px;
    outline: 1px solid ${COLOR.ghostBorder};
  }
  code, .mono { font-family: ${FONT.mono}; font-size: 12px; letter-spacing: 0.02em; }
  .label { font-family: ${FONT.mono}; font-size: 11px; letter-spacing: 0.04em; color: ${COLOR.onSurfaceMuted}; text-transform: uppercase; }
  .stale { opacity: 0.4; }
`;
