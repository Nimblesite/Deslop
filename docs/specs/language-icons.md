# Language identity

## [VSIX-LANGUAGE-ICONS] Language icons

Every supported language has a distinct, legible icon in native language groups, file rows, occurrence rows and file metrics, and in the report, cluster and duplication webviews. One shared icon registry supplies the official Osprey extension image and the standard language glyphs from VS Code’s bundled Seti theme. Custom letter badges and crowned Osprey substitutes are prohibited. Source provenance and licenses live in `clients/vscode/licenses/`. Accessible labels and tooltips retain the human language name. Unknown extensions receive a neutral fallback.

Osprey leads the language registry and receives the official cyan Osprey icon. This presentation never changes duplication scores, thresholds or worst-first ranking. The `osprey`, `osprey-ml` and `osprey-python` identities display the same Osprey family artwork. Consumers prefer authoritative report language metadata over filename extensions, because opted-in Osprey Python shares `.py` with ordinary Python. Parser registration and verification status follow [LANG-CAND-OSPREY]; an icon does not establish syntax support.

Implemented by `clients/vscode/src/types/languageIcons.ts`, the native tree renderers and `LanguageBadge.tsx`; exercised by `clients/vscode/src/test/unit/languageIcons.unit.test.ts` and the webview smoke test.
