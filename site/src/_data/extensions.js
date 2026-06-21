// Published Deslop editor extensions — the single source of truth for the
// homepage install/review band (partials/editors.njk) and any other surface
// that links to a marketplace listing. Add a new registry or a new released
// extension here and every consumer picks it up; do not hard-code these URLs
// in templates. Brand fields (registry, editors) read the same in every
// language; only `blurb` is localised, keyed by page language.
export default [
  {
    id: "vscode-marketplace",
    registry: "Visual Studio Marketplace",
    editors: "VS Code · GitHub Codespaces",
    icon: "code",
    installUrl: "https://marketplace.visualstudio.com/items?itemName=Nimblesite.deslop-live",
    reviewUrl: "https://marketplace.visualstudio.com/items?itemName=Nimblesite.deslop-live&ssr=false#review-details",
    blurb: {
      en: "The full bundle — live bubble, LSP server, MCP server, and the deslop CLI — for VS Code and GitHub Codespaces.",
      zh: "完整捆绑包——实时气泡、LSP 服务器、MCP 服务器与 deslop CLI——适用于 VS Code 与 GitHub Codespaces。",
    },
  },
  {
    id: "open-vsx",
    registry: "Open VSX Registry",
    editors: "Cursor · Windsurf · VSCodium",
    icon: "extension",
    installUrl: "https://open-vsx.org/extension/Nimblesite/deslop-live",
    reviewUrl: "https://open-vsx.org/extension/Nimblesite/deslop-live/reviews",
    blurb: {
      en: "The same extension for Cursor, Windsurf, VSCodium, Gitpod, and every Open VSX editor.",
      zh: "同一扩展，适用于 Cursor、Windsurf、VSCodium、Gitpod 以及所有 Open VSX 编辑器。",
    },
  },
];
