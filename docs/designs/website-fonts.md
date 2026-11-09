# Material Symbols

Material Symbols Outlined is supplied by Google under Apache-2.0.
The accompanying `material-symbols-LICENSE.txt` retains the upstream license.

Source: https://github.com/google/material-design-icons
Google Fonts delivery version: v383, downloaded 2026-11-09.
The shipped WOFF2 is subset to the icons found in the completed website build,
with the original font's default regular, unfilled appearance. Regenerate with
`scripts/site/subset-icons.py <original.woff2> site/_site <output.woff2>` using
FontTools with WOFF support; alias ligatures are resolved from the upstream font.
The release-card icon is preserved even when live release metadata is unavailable.
The browser test checks every published icon still shapes to a single glyph.
Retain the upstream license with this derived font.

Serving this font locally prevents navigation icons from disappearing or
holding page load open when the external font service is unavailable.

Inter and JetBrains Mono variable TrueType fonts are unmodified downloads from
Google Fonts on 2026-11-09, under their accompanying SIL Open Font Licenses:

- https://github.com/google/fonts/tree/main/ofl/inter
- https://github.com/google/fonts/tree/main/ofl/jetbrainsmono

The complete upstream fonts preserve language coverage and variable weights.
Reading and code fonts are local too, so the site keeps its typography when
external font services cannot be reached.
