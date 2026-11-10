"""[SITE-NAV-FONTS] Keep only icons used by the completed website build.

Run with fonttools[woff]; pass the original font, built site, and output font.
"""
from html.parser import HTMLParser
from pathlib import Path
import sys
from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

# Release cards can be absent when their live metadata service is unavailable.
DYNAMIC_ICONS = {"open_in_new"}


class IconNames(HTMLParser):
    """Collect Material Symbols ligature names from rendered markup."""
    def __init__(self) -> None:
        super().__init__()
        self.inside = False
        self.names: set[str] = set()

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag == 'span':
            self.inside = 'material-symbols-outlined' in (dict(attrs).get('class') or '')

    def handle_data(self, data: str) -> None:
        if self.inside and data.strip():
            self.names.add(data.strip())

    def handle_endtag(self, tag: str) -> None:
        if tag == 'span':
            self.inside = False


def ligature_glyph(font: TTFont, text: str) -> str:
    """Resolve names and aliases through the original font's substitution table."""
    glyphs = [font.getBestCmap()[ord(char)] for char in text]
    for lookup in font['GSUB'].table.LookupList.Lookup:
        for table in lookup.SubTable:
            table = getattr(table, 'ExtSubTable', table)
            for ligature in getattr(table, 'ligatures', {}).get(glyphs[0], []):
                if [glyphs[0], *ligature.Component] == glyphs:
                    return ligature.LigGlyph
    raise ValueError(f'No icon ligature for {text}')


def main() -> None:
    original, site, output = map(Path, sys.argv[1:])
    parser = IconNames()
    for page in site.rglob('*.html'):
        parser.feed(page.read_text())
    if not parser.names:
        raise ValueError('Build the website before subsetting icons')
    parser.names.update(DYNAMIC_ICONS)
    font = TTFont(original)
    glyphs = [ligature_glyph(font, name) for name in sorted(parser.names)]
    instantiateVariableFont(font, {axis.axisTag: axis.defaultValue for axis in font['fvar'].axes}, inplace=True)
    options = subset.Options(layout_closure=False)
    reducer = subset.Subsetter(options=options)
    reducer.populate(glyphs=glyphs, text=''.join(parser.names))
    reducer.subset(font)
    font.flavor = 'woff2'
    font.save(output)
    print(f'{len(parser.names)} icons, {output.stat().st_size} bytes: {", ".join(sorted(parser.names))}')


if __name__ == '__main__':
    main()
