import { languageDisplayName, languageForPath } from "../../../src/types/languages";
import { languageIconSvg } from "../../../src/types/languageIcons";

const BADGE_SIZE = "20px";

// [VSIX-LANGUAGE-ICONS] Inline trusted SVG works under the webview CSP.
export function LanguageBadge({ path, selected }: { path: string; selected?: string | null | undefined }) {
  const language = languageForPath(path, selected);
  const name = languageDisplayName(language);
  return <span class="language-icon" data-language={language} role="img"
    aria-label={name} title={name}
    style={{ display: "inline-flex", width: BADGE_SIZE, height: BADGE_SIZE,
      flexShrink: 0, verticalAlign: "middle", marginRight: "6px" }}
    dangerouslySetInnerHTML={{ __html: languageIconSvg(language) }} />;
}
