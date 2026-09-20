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
  button:disabled { opacity: 0.45; cursor: default; }
  button:focus-visible, a:focus-visible { outline: 2px solid ${COLOR.primary}; outline-offset: 3px; }
  button.secondary { background: transparent; border: 1px solid ${COLOR.ghostBorder}; }
  .canonical-label { align-self: center; color: ${COLOR.onSurfaceMuted}; font-size: 12px; }
  .occurrence-list { margin-top: 24px; }
  .occurrence-heading { margin-bottom: 12px; }
  .occurrence-row { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 220px), 1fr)); gap: 16px; align-items: center; padding: 14px 20px; cursor: pointer; }
  .occurrence-row:nth-of-type(odd) { background: ${COLOR.surfaceContainerLow}; }
  .occurrence-location { min-width: 0; font-family: ${FONT.mono}; font-size: 12px; }
  .occurrence-location button { max-width: 100%; overflow-wrap: anywhere; }
  .occurrence-position { margin-top: 2px; font-size: 11px; color: ${COLOR.onSurfaceMuted}; }
  .occurrence-actions { display: flex; justify-content: flex-end; flex-wrap: wrap; gap: 8px; }
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
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 1px solid ${COLOR.ghostBorder};
    background: ${COLOR.surfaceContainerHigh};
    color: ${COLOR.onSurfaceMuted};
    font-family: ${FONT.mono};
    font-size: 13px;
    font-weight: 700;
    line-height: 1;
    text-decoration: none;
  }
  .help-bubble:hover {
    background: ${COLOR.surfaceContainerHighest};
    color: ${COLOR.primary};
    border-color: ${COLOR.primary};
  }
  .help-anchor { display: inline-flex; text-align: left; }
  .help-popover {
    position: fixed; z-index: 100; overflow-y: auto;
    padding: 14px 16px; border: 1px solid ${COLOR.ghostBorder}; border-radius: 8px;
    background: ${COLOR.surfaceContainerHigh}; color: ${COLOR.onSurface};
    box-shadow: 0 8px 28px #0008;
    font: 13px/1.5 ${FONT.ui}; letter-spacing: normal; text-transform: none;
    overflow-wrap: anywhere;
  }
  .help-popover p { margin: 0 0 10px; }
  .help-popover ul { padding-left: 20px; margin: 0 0 10px; }
  .help-popover li + li { margin-top: 6px; }
  .help-popover a { color: ${COLOR.primary}; }
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
  .cluster-panel { padding: 24px clamp(16px, 4vw, 32px); max-width: 1200px; margin: auto; }
  .cluster-eyebrow, .cluster-heading, .cluster-weight, .cluster-shortcuts { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .cluster-heading { margin-top: 16px; }
  .cluster-heading h1 { font-size: 26px; line-height: 1.2; letter-spacing: -0.02em; margin: 0; }
  .cluster-summary { margin: 10px 0; color: ${COLOR.onSurfaceMuted}; }
  .cluster-weight { margin: 14px 0; }
  .cluster-weight strong { margin-left: 6px; }
  .cluster-details { font-size: 12px; color: ${COLOR.onSurfaceMuted}; overflow-wrap: anywhere; }
  .cluster-details summary { cursor: pointer; width: fit-content; padding: 4px 0; }
  .cluster-details div { margin-top: 8px; }
  .cluster-navigation { display: flex; align-items: center; justify-content: flex-end; gap: 12px; margin-top: 24px; padding-top: 20px; border-top: 1px solid ${COLOR.ghostBorder}; }
  .cluster-position, .cluster-shortcuts { color: ${COLOR.onSurfaceMuted}; font-size: 12px; }
  .cluster-shortcuts { margin-top: 20px; }
  .cluster-shortcuts p { flex-basis: 100%; margin: 0; }
`;
