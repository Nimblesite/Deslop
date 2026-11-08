# /// script
# dependencies = ["fonttools==4.68.0", "pillow==13.0.0"]
# ///
"""Import official artwork. Run with uv, passing the Seti icons directory and Osprey icon.png."""
import base64
import json
import sys
from pathlib import Path
from io import BytesIO

from PIL import Image

from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.ttLib import TTFont

ICON_SIZE = 24
ARTWORK_SIZE = 22
RASTER_SIZE = 96
LANGUAGES = {
    "c": "c", "csharp": "csharp", "rust": "rust", "python": "python",
    "dart": "dart", "javascript": "javascript", "typescript": "typescript",
    "tsx": "typescriptreact", "fsharp": "fsharp", "php": "php", "go": "go",
}


def glyph_svg(font, definition):
    """Preserve the Seti glyph outline and color, fitting its bounds to the shared canvas."""
    glyphs = font.getGlyphSet()
    codepoint = int(definition["fontCharacter"].lstrip("\\"), 16)
    glyph = glyphs[font.getBestCmap()[codepoint]]
    outline, bounds = SVGPathPen(glyphs), BoundsPen(glyphs)
    glyph.draw(outline)
    glyph.draw(bounds)
    left, bottom, right, top = bounds.bounds
    scale = ARTWORK_SIZE / max(right - left, top - bottom)
    offset_x = ICON_SIZE / 2 - (left + right) * scale / 2
    offset_y = ICON_SIZE / 2 + (bottom + top) * scale / 2
    return (f'<g fill="{definition["fontColor"]}" '
            f'transform="translate({offset_x:g} {offset_y:g}) scale({scale:g} {-scale:g})">'
            f'<path d="{outline.getCommands()}"/></g>')


def raster_icon(path):
    """Resize the official image for crisp 4x rendering without repeating a large source per row."""
    output = BytesIO()
    with Image.open(path) as source:
        image = source.convert("RGBA")
        image.thumbnail((RASTER_SIZE, RASTER_SIZE), Image.Resampling.LANCZOS)
        image.save(output, format="PNG", optimize=True)
    return base64.b64encode(output.getvalue()).decode("ascii")


def main():
    """Write the portable artwork registry consumed by native and webview builds."""
    icons, osprey = map(Path, sys.argv[1:])
    theme = json.loads((icons / "vs-seti-icon-theme.json").read_text())
    font = TTFont(icons / "seti.woff")
    encoded = raster_icon(osprey)
    artwork = {"osprey": f'<image width="24" height="24" href="data:image/png;base64,{encoded}"/>'}
    for language, editor_id in LANGUAGES.items():
        artwork[language] = glyph_svg(font, theme["iconDefinitions"][theme["languageIds"][editor_id]])
    artwork["unknown"] = glyph_svg(font, theme["iconDefinitions"][theme["file"]])
    output = Path(__file__).parent.parent / "src/types/languageArtwork.json"
    output.write_text(json.dumps(artwork, indent=2) + "\n")


if __name__ == "__main__":
    main()
