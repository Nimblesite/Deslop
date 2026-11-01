// [VSIX-LANGUAGE-ICONS] One icon source for native trees and webviews.

import { languageFamily } from "./languages";

interface LanguageMark {
  label: string;
  color: string;
}

const LANGUAGE_MARKS: Readonly<Record<string, LanguageMark>> = {
  osprey: { label: "Osprey", color: "#f5c451" },
  csharp: { label: "C#", color: "#b693ed" },
  rust: { label: "Rs", color: "#e9a17f" },
  python: { label: "Py", color: "#84b7eb" },
  dart: { label: "D", color: "#60d3ef" },
  javascript: { label: "JS", color: "#f1dc62" },
  typescript: { label: "TS", color: "#86b9f2" },
  tsx: { label: "TSX", color: "#86d9ef" },
  fsharp: { label: "F#", color: "#9bb5ec" },
  php: { label: "php", color: "#bcb5e9" },
  go: { label: "Go", color: "#72d4e4" },
};
const UNKNOWN_MARK: LanguageMark = { label: "?", color: "#b8bec8" };
const ICON_SIZE = 24;
const OSPREY_LANGUAGE = "osprey";
const OSPREY_BIRD = '<path d="M3 12 9 13 12 9 15 13 21 12 16 17 13 16 12 21 11 16 8 17Z"/>';
const OSPREY_CROWN = '<path d="m7 3 3 2 2-3 2 3 3-2-1 5H8Z"/>';

/** Trusted registry artwork only; no source paths or user text enter SVG. */
export function languageIconSvg(language: string): string {
  const family = languageFamily(language);
  const registered = Object.hasOwn(LANGUAGE_MARKS, family) ? LANGUAGE_MARKS[family] : undefined;
  const mark = registered ?? UNKNOWN_MARK;
  const artwork = family === OSPREY_LANGUAGE
    ? OSPREY_CROWN + OSPREY_BIRD
    : `<text x="12" y="16" text-anchor="middle" font-family="sans-serif" font-size="11" font-weight="700">${mark.label}</text>`;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${ICON_SIZE}" height="${ICON_SIZE}" viewBox="0 0 24 24"><rect x="1" y="1" width="22" height="22" rx="5" fill="#18202b"/><g fill="${mark.color}">${artwork}</g></svg>`;
}

/** Encoded SVG supported by VS Code tree icons and browser image elements. */
export function languageIconUri(language: string): string {
  return `data:image/svg+xml,${encodeURIComponent(languageIconSvg(language))}`;
}
