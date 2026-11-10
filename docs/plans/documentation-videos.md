# Documentation films [DOC-FILMS]

Record ten practical user workflows in Videos, using the real packaged Deslop
extension, CLI and pi MCP connection. Follow SharpLsp's capture and automatic
camera implementation. Show the application throughout; narration explains the
visible action in plain English. No concept cards or parser explanations.

## Delivery checklist [DOC-FILMS-DELIVERY]

- [x] Inspect the SharpLsp scripts, recordings and automatic camera.
- [x] Rewrite all forty spoken steps around useful visible user actions.
- [x] Generate Aiden narration with local Qwen3-TTS and check its transcription.
- [x] Record source navigation, both comparison methods, settings and CLI checks.
- [x] Record and verify real pi tool calls and returned file locations.
- [x] Cut all ten films with automatic zoom and short captions.
- [x] Inspect finished application frames and verify all delivery streams.
- [x] Test both language routes, mobile playback, tracks and transcripts.
- [ ] Commit, push and deploy the website; verify public media hashes.
- [ ] Remove owned temporary files and model caches; check disk space.

## Production [DOC-FILMS-PRODUCTION]

Stop at 12:01 am Australia/Melbourne on 11 November 2026. Generate speech locally,
use one encoding job at a time and reuse Videos' sound and delivery modules.
Source footage and voice receipts remain in Videos; website media uses Git LFS.
Never stop a VS Code process with a signal.

## Recorded workflows [DOC-FILMS-LIVE]

`node scripts/videos/shoot-user-workflows.mjs` records the real packaged
extension. Its session receipt supports resuming with `DESLOP_TUTORIAL_CDP`.
`shoot-agent-workflows.mjs` records pi against that workspace and Deslop MCP.
Raw recordings include pointer tracks and click cues. Failed takes are removed.

In Videos, run `doc-series/narrate-workflows.py`, `check-narration.py`, then
`prepare-workflows.mjs` under `projects/deslop/videos/`. In Deslop, run
`node scripts/videos/cut-user-workflows.mjs --install`, with
`CHROMIUM_EXECUTABLE` pointing to an installed browser. The installer checks
current narration receipts and emits per-film Markdown media receipts.

## Picture and sound [DOC-FILMS-MOTION]

Real application footage fills the complete timeline. SharpLsp's camera follows
pointer and editor activity; captions contain at most seven words per cue.
Narration uses Qwen3-TTS Aiden with a quiet score. Diagrams, slide decks and
standalone text compositions are excluded. Finished-frame review must establish
that each spoken action has useful visible evidence.

## Evidence [DOC-FILMS-COVERAGE]

`check-doc-films.mjs --complete` emits the delivery inventory in
`reports/videos/series-status.md`. Per-film receipts record raw source duration,
playback speed, automatic camera movement and delivered hashes. These checks
supplement visual review; they cannot establish useful footage on their own.

## Browser and deployment checks [DOC-FILMS-VALIDATION]

Website tests exercise both languages, responsive geometry, playback, seeking,
caption counts, four chapters, transcript access and Osprey blog redirects.
`record-site-check.mjs` emits evidence from a successful browser run.
`verify-deployed-films.mjs <Pages-run-URL>` downloads the public videos and
compares their hashes with the local deliveries.
