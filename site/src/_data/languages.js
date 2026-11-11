// Supported languages for the homepage language panel, read from the VS Code
// extension's registry and icon artwork so the site never lists a language,
// extension or glyph the product does not ship ([VSIX-LANGUAGE-ICONS]).
import registry from "../../../clients/vscode/src/types/languageRegistry.json" with { type: "json" };
import artwork from "../../../clients/vscode/src/types/languageArtwork.json" with { type: "json" };

const ICON_SIZE = 24;

const extensionsOf = (id) =>
  Object.entries(registry.extensions)
    .filter(([, language]) => language === id)
    .map(([extension]) => `.${extension}`);

const iconOf = (id) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${ICON_SIZE} ${ICON_SIZE}" aria-hidden="true" focusable="false">${artwork[id]}</svg>`;

export default Object.entries(registry.display).map(([id, name]) => ({
  id,
  name,
  extensions: extensionsOf(id),
  icon: iconOf(id),
}));
