# Website

## [SITE-HOME-VIDEO] Homepage product demo

The English and Chinese homepages place one narrated product demonstration in
the hero beside the introduction. The same real extension footage is available
on both routes, with a localized player label and English captions. Playback is
visitor-controlled: native controls, no autoplay, no loop, and no media preload.
The poster remains visible until playback starts. The player fits mobile and
desktop widths and remains usable without JavaScript and with reduced motion.

`site/tests/homepage-motion.spec.js` and `site/tests/homepage-reel.spec.js` verify
playback, pause, seeking, captions, poster loading and responsive placement.

## [DOC-FILMS-DELIVERY] Documentation films

Each of the ten documentation topics has a concise narrated film, shared by its
English and Chinese page. English narration and captions are explicitly labelled.
Every delivery includes a 1920x1080 H.264/AAC MP4, poster, English captions and
chapter track. A readable transcript comes from the delivered narration timeline.
Players use native controls, never autoplay, and fit widths from 320px upward.

The factory lives in Videos under `projects/deslop/videos/docs-*`, reusing its
Osprey production templates and shared narration/rendering machinery. Deslop's
importer rejects stale scripts or invalid media and verifies every rendered
chapter title plus targeted visible content before adding a film to the catalog.
MP4 files use Git LFS. Production intermediates are never published.

`site/tests/documentation-films.spec.js` exercises each catalog entry on both
language routes, including responsive geometry, playback, seeking, captions,
chapters and transcript expansion. `scripts/videos/check-doc-films.mjs --complete`
requires all ten topics to have imported deliveries and verification receipts.

## [SITE-ASSET-TYPE] Declared media types match served assets

Every referenced script, stylesheet, image and social card is served successfully
with a content type matching its declaration. Encoded media is limited to the
homepage product demo and the ten requested documentation films. The website
audit verifies this exact inventory; unrelated audio/video and render masters
are rejected. The homepage and audit tests share the HTTP checks in
`site/tests/helpers/assets.js`.

## [SITE-ISSUES-FIXTURE] Issue tests run on a fixed set of issues

The published issue pages are built from live GitHub, but their tests are not. `site/tests/fixtures/github-issues.json` is a committed snapshot of the open issues. Each body is cut down to the parts the report reads: its TL;DR and the issues it references. Two of the issues are labelled `fixed-on-main` so the release-verification views always have something to show. Before the tests run, `npm test` generates the issue report from that snapshot with a fixed publication time. `site/tests/helpers/issues.js` then serves that report to every page in `issues.spec.js` and `issues-publication.spec.js`. The tests pass or fail the same way whatever is open on GitHub that day.
