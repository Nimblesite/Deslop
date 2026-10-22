# Website

## [SITE-HOME-VIDEO] Homepage motion reel

The English and Chinese homepages open with the same short animation of what Deslop does. It plays five translated headlines in order: find duplicate code, nine languages, worst offenders first, stop coding agents from creating more, and live in VS Code. Then it loops.

The reel is always in the first screen. It sits beside the headline on wide screens and above it once the hero stacks. The stage is wide on desktop and tablets and square on phones, and its type scales with the stage.

It plays silently on its own. Its only control is one small, faint pause icon. It stops while it is off screen or the tab is hidden, and resumes where it stopped. Visitors who prefer reduced motion see the finished opening scene until they press play; scenes then fade without moving. Without JavaScript the finished opening scene stays and no control is shown.

The animation is built in the page with JavaScript; no encoded video or audio is shipped. `site/tests/homepage-motion.spec.js` and `site/tests/homepage-reel.spec.js` drive it on the page's own clock through every scene, pause, reduced motion, off-screen and hidden-tab stops, both themes, each screen size and the Chinese translation. Together the script and styles must stay within 32 KiB.

## [SITE-ASSET-TYPE] Declared media types match served assets

Every referenced script, stylesheet, image and social card is served successfully with a content type matching its declaration. The site source contains no encoded video or audio files.

The homepage and website-audit tests share `site/tests/helpers/assets.js` to verify actual HTTP asset responses, and the website audit checks the site source for encoded media.

## [SITE-ISSUES-FIXTURE] Issue tests run on a fixed set of issues

The published issue pages are built from live GitHub, but their tests are not. `site/tests/fixtures/github-issues.json` is a committed snapshot of the open issues. Each body is cut down to the parts the report reads: its TL;DR and the issues it references. Two of the issues are labelled `fixed-on-main` so the release-verification views always have something to show. Before the tests run, `npm test` generates the issue report from that snapshot with a fixed publication time. `site/tests/helpers/issues.js` then serves that report to every page in `issues.spec.js` and `issues-publication.spec.js`. The tests pass or fail the same way whatever is open on GitHub that day.
