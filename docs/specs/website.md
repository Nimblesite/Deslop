# Website

## [SITE-HOME-VIDEO] Homepage release motion

The English and Chinese homepages share one JavaScript motion reel with translated headings, playback controls and accuracy-post links. The animation renders its scenes in the page with Web Animations; no encoded video files are shipped. One responsive stage changes between wide desktop and tall phone layouts, retaining the opening report artwork. The original soundtrack loads only after the visitor presses play and clocks the 21-second sequence. Play, pause, restart, mute and seek operate on that same timeline. Playback pauses when the page is hidden. Without JavaScript, the opening frame and accuracy link remain available. Reduced-motion preferences suppress movement. Metadata describes the page animation as a CreativeWork, without advertising a nonexistent hosted video.

`site/tests/homepage-motion.spec.js` and `site/tests/homepage-reel.spec.js` drive real playback, pause, seek, responsive layouts, translated controls, links and metadata against the built site. Together the script, styles, soundtrack and both posters must remain within 512 KiB.

## [SITE-ASSET-TYPE] Declared media types match served assets

Every referenced soundtrack, poster, script, stylesheet and social card is served successfully with a content type matching its declaration. Playback retains the original soundtrack and duration. The site build contains no encoded video assets.

The homepage and website-audit tests share `site/tests/helpers/assets.js` to verify actual HTTP asset responses. Shared motion assertions verify real playback and complete responsive stage dimensions.
