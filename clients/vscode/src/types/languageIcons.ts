// [VSIX-LANGUAGE-ICONS] Official Osprey artwork and VS Code's bundled Seti glyphs.
import artwork from "./languageArtwork.json";
import { languageFamily } from "./languages";

const ICON_SIZE = 24;

/** Trusted, vendored artwork only; source paths and user text never enter SVG. */
export function languageIconSvg(language: string): string {
  const family = languageFamily(language);
  const content = Object.hasOwn(artwork, family)
    ? artwork[family as keyof typeof artwork]
    : artwork.unknown;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${ICON_SIZE}" height="${ICON_SIZE}" viewBox="0 0 ${ICON_SIZE} ${ICON_SIZE}">${content}</svg>`;
}

/** Encoded SVG supported by VS Code tree icons and browser image elements. */
export function languageIconUri(language: string): string {
  return `data:image/svg+xml,${encodeURIComponent(languageIconSvg(language))}`;
}
