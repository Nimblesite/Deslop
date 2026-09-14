// The webview reads the extension's design tokens straight from
// `clients/vscode/src/design.ts` — one table for every surface, so the
// badge, the tree icon and the editor underline can never disagree on a
// kind's colour ([CLONE-KIND-COLOR]). Only the global stylesheet the
// webview documents share lives here.

import { COLOR, FONT } from "../../src/design";

export { COLOR, FONT, KIND_COLOR, SEVERITY_DOT } from "../../src/design";

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
  .doc-link {
    color: inherit;
    text-decoration: underline;
    text-decoration-color: ${COLOR.onSurfaceMuted};
    text-decoration-style: dotted;
    text-underline-offset: 3px;
  }
  .doc-link:hover { color: ${COLOR.primary}; text-decoration-color: ${COLOR.primary}; }
  .with-help {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .help-bubble {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1px solid ${COLOR.ghostBorder};
    background: ${COLOR.surfaceContainerHigh};
    color: ${COLOR.onSurfaceMuted};
    font-family: ${FONT.mono};
    font-size: 10px;
    font-weight: 700;
    line-height: 1;
    text-decoration: none;
  }
  .help-bubble:hover {
    background: ${COLOR.surfaceContainerHighest};
    color: ${COLOR.primary};
    border-color: ${COLOR.primary};
  }
  button.text-action {
    background: transparent;
    color: inherit;
    padding: 0;
    font-family: ${FONT.mono};
    text-align: left;
    text-decoration: underline;
    text-decoration-color: ${COLOR.onSurfaceMuted};
    text-decoration-style: dotted;
    text-underline-offset: 3px;
  }
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
