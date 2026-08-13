PANDOC ?= pandoc
EPUBCHECK ?= epubcheck
RSVG_CONVERT ?= rsvg-convert
MAGICK ?= magick
JQ ?= jq

BOOK_JSON := book.json
MANUSCRIPT := $(shell $(JQ) -r '.sections[].file' $(BOOK_JSON))
EPUB := dist/the-deslop-book-outline.epub
HTML := dist/index.html

.PHONY: check render-assets epub html release clean

check:
	@$(JQ) -e '(.schemaVersion == 1) and ((.sections | length) == 15)' $(BOOK_JSON) >/dev/null
	@$(JQ) -e '(.schemaVersion == 1) and ((.sources | length) >= 8)' sources.json >/dev/null
	@$(JQ) -e '(.schemaVersion == 1) and ((.figures | length) >= 3)' figures.json >/dev/null
	@for file in $(MANUSCRIPT); do test -f "$$file" || { echo "Missing manuscript file: $$file"; exit 1; }; done
	@$(PANDOC) $(MANUSCRIPT) --from=gfm --to=native >/dev/null

render-assets: assets/cover/cover.png \
	assets/diagrams/00-reading-journey.png \
	assets/diagrams/01-dry-vs-deslop.png \
	assets/diagrams/01-identical-code-needs-a-verdict.png

assets/cover/cover.png: assets/cover/cover.svg
	@$(RSVG_CONVERT) --width 1600 --height 2560 --output $@.base.png $<
	@$(MAGICK) $@.base.png \( ../docs/designs/logo.png -resize 176x176 \) -geometry +112+112 -composite $@
	@rm -f $@.base.png

assets/diagrams/00-reading-journey.png: assets/diagrams/00-reading-journey.svg
	@$(RSVG_CONVERT) --width 1600 --height 1000 --output $@ $<

assets/diagrams/01-dry-vs-deslop.png: assets/diagrams/01-dry-vs-deslop.svg
	@$(RSVG_CONVERT) --width 1600 --height 1000 --output $@ $<

assets/diagrams/01-identical-code-needs-a-verdict.png: assets/diagrams/01-identical-code-needs-a-verdict.svg
	@$(RSVG_CONVERT) --width 1600 --height 1000 --output $@ $<

epub: check render-assets
	@mkdir -p dist
	@$(PANDOC) $(MANUSCRIPT) --from=gfm --metadata-file=metadata.yaml --css=styles/epub.css --epub-cover-image=assets/cover/cover.png --toc --toc-depth=2 --output $(EPUB)
	@$(EPUBCHECK) $(EPUB)

html: check render-assets
	@mkdir -p dist
	@$(PANDOC) $(MANUSCRIPT) --from=gfm --metadata-file=metadata.yaml --css=styles/epub.css --standalone --embed-resources --toc --toc-depth=2 --output $(HTML)

release: epub html

clean:
	@rm -f $(EPUB) $(HTML)
