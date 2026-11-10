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

Nine documentation topics and the Osprey launch post have concise narrated films,
shared by their English and Chinese pages. The former Osprey docs URLs redirect
to the corresponding launch posts. The shared player fits the reading column
even if the prose stylesheet is stale or unavailable. English narration and
captions are explicitly labelled.
Every delivery includes a 1920x1080 H.264/AAC MP4, poster, English captions and
chapter track. A readable transcript comes from the delivered narration timeline.
Players use native controls, never autoplay, and fit widths from 320px upward.
Video carries a keyframe every second so Safari changes playback speed
immediately instead of freezing while it decodes back to a distant keyframe.
`site/tests/documentation-films-playback-speed.spec.js` reads each delivery's
own sync-sample table, so the check needs no decoder and runs on every platform.

Production lives in Videos under `projects/deslop/videos/docs-*`. SharpLsp's
capture and automatic camera modules record the real packaged extension and
Deslop commands. Shared Videos sound and delivery modules supply narration,
score, captions and encoding. The installer rejects stale scripts or invalid
media. MP4 files use Git LFS; production intermediates are never published.
Every film must stay re-renderable from its preserved inputs with the
documented steps; delivered MP4s are never patched directly.

`site/tests/documentation-films.spec.js` exercises each catalog entry on both
language routes, including responsive geometry, playback, seeking, captions,
chapters and transcript expansion. `scripts/videos/check-doc-films.mjs --complete`
requires all ten films to have imported deliveries and verification receipts.

## [DOC-FILMS-MOTION] Recorded user workflows

Every film shows real application footage throughout. Narration describes the
visible editing, configuration, file navigation, comparisons or command results
in plain English. No concept cards, diagrams or parser explanations substitute
for using the product. Small step labels and short captions may overlay footage.
The agent films show pi making real Deslop MCP calls and receiving useful file
locations. SharpLsp's automatic camera follows the pointer, editor and results;
zoom must visibly move without cutting off the action. Finished-frame checks,
speech transcription checks and browser playback checks supplement media probes.

## [SITE-NAV-FONTS] Reliable navigation icons

Material Symbols, Inter and JetBrains Mono ship with the website, including
their upstream licenses. Icons and reading/code fonts must remain available
when external font services are unreachable. `site/tests/navigation-fonts.spec.js` verifies that behavior with
both external font endpoints blocked and verifies every published icon still
shapes to a single glyph. The icon subset retains the release-card icon even
when live release metadata is unavailable.

## [SITE-LOAD-ANALYTICS] Analytics follows page readiness

Analytics commands are queued immediately, but its external script is requested
only after the page's load event. An unavailable analytics service must not
hold page loading open. `site/tests/page-loading.spec.js` stalls that service
and verifies the page loads with its event queue intact.

## [SITE-ASSET-TYPE] Declared media types match served assets

Every referenced script, stylesheet, image and social card is served successfully
with a content type matching its declaration. Encoded media is limited to the
homepage product demo and the ten requested films. The website
audit verifies this exact inventory; unrelated audio/video and render masters
are rejected. The homepage and audit tests share the HTTP checks in
`site/tests/helpers/assets.js`.

## [SITE-BLOG-ARTICLE] Blog posts carry their own freshness, authorship and substance

Every blog post, in both languages, publishes `article:published_time`, `article:modified_time` and `article:author`, matching the `BlogPosting` dates and author. When a post has an `updated` date that differs from its publication date, the byline shows it as a visible "Updated" date. A post's social-image alt text describes its own card, falling back to its hero image alt text rather than the site-wide default. Every English post body has at least 1,000 words, and its title and description stay within search-result display limits. Implemented in `site/lib/overrides/layouts.js`; verified by `site/tests/website-audit.spec.js`.

## [SITE-ISSUES-FIXTURE] Issue tests run on a fixed set of issues

The published issue pages are built from live GitHub, but their tests are not. `site/tests/fixtures/github-issues.json` is a committed snapshot of the open issues. Each body is cut down to the parts the report reads: its TL;DR and the issues it references. Two of the issues are labelled `fixed-on-main` so the release-verification views always have something to show. Before the tests run, `npm test` generates the issue report from that snapshot with a fixed publication time. `site/tests/helpers/issues.js` then serves that report to every page in `issues.spec.js` and `issues-publication.spec.js`. The tests pass or fail the same way whatever is open on GitHub that day.
