# Website font delivery

Baseline: bundled assets in commit 5a8ade06. Current sizes are read from the delivered files; every current font is committed as a Git LFS pointer.

| Font | Before (bytes) | Delivered (bytes) | Saved (bytes) |
| --- | ---: | ---: | ---: |
| Material Symbols | 1140348 | 3644 | 1136704 |
| Inter | 876576 | 350616 | 525960 |
| JetBrains Mono | 187208 | 71520 | 115688 |
| Total | 2204132 | 425780 | 1778352 |

Regenerate with `node scripts/site/check-font-delivery.mjs`. Text-font glyph coverage is retained; the symbols font is subset by `scripts/site/subset-icons.py`. Browser checks verify local loading and every published icon ligature.
