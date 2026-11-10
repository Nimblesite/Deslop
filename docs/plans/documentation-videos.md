# Documentation film series [DOC-FILMS]

Create a narrated film for each of the ten English documentation pages, using
Videos' existing Osprey production templates and shared speech/rendering pipeline.
Keep Deslop branding, real source examples, accurate qualifications and readable
captions. Short reference films are explainers; task tutorials demonstrate a real
run and a change. Do not pad films to reach a duration.

## Delivery checklist [DOC-FILMS-DELIVERY]

- [x] Inventory all ten documentation pages and inspect the shared film pipeline.
- [x] Review narration against each source page and choose real visual evidence.
- [x] Produce a shared Deslop series template with distinct chapter compositions.
- [x] Render each film with narration, captions, chapters and a poster.
- [x] Inspect finished frames, review narration text, and verify duration and encoding.
- [x] Integrate accessible video players into the matching documentation pages.
- [x] Store delivered videos in Git LFS; verify desktop and mobile playback.
- [x] Remove disposable render intermediates and check disk space.

## Coverage [DOC-FILMS-COVERAGE]

The mechanically generated [delivery inventory](../../reports/videos/series-status.md)
tracks every page, duration, delivery receipt and LFS rule. Individual media
receipts and measured audio levels live beside it; do not duplicate their figures
in this plan.

## Production constraints [DOC-FILMS-PRODUCTION]

Use local cached narration, one rendering job at a time and the factory's
cleanWork option. The initial free disk measurement was 14 GiB. Do not download
IDE dependencies or build the Rust workspace for film production. Existing
uncommitted deployment changes belong to another task and are excluded.
Stop production at midnight Australia/Melbourne as instructed by the user.

## Current production evidence [DOC-FILMS-EVIDENCE]

The Getting Started film recipe is in Videos at
`projects/deslop/videos/docs-getting-started/film.mjs`; shared composition files
are under `projects/deslop/videos/doc-series/`. Ten source-page snapshots and
SHA-256 hashes are in its `sources/` directory. Preview frames were rendered
and visually inspected before narration. The first publish rejected an
intermediate quality setting of 20; the recipe now uses the supported maximum
of 18. All ten films are delivered. The complete browser suite passes; its mechanical
receipt is [site-validation.md](../../reports/videos/site-validation.md). The ten remote LFS objects were fetched into a fresh temporary store and their
SHA-256 hashes match the committed pointers. Render intermediates are removed;
remaining temporary verification files are removed before the midnight stop.

Reviewed production scripts live in Videos under `projects/deslop/videos/docs-*/`.
Superseded pi drafts were removed after review; model outcomes and corrections
are recorded in PiNotes/2026-11-09-deslop-doc-video-scripts.md.

## Final validation [DOC-FILMS-VALIDATION]

The browser checks cover both language routes, widths from 320px to 1440px,
actual playback and seeking at phone and desktop sizes, caption/chapter tracks,
and keyboard transcript expansion. Native controls remain visitor-operated.
Audio encoding and level measurements do not establish subjective pronunciation
quality; narration was reviewed as text against the corresponding documentation.

## Live VS Code footage [DOC-FILMS-LIVE]

The documentation films open real files, inspect findings, compare source
occurrences, and run the Deslop CLI or MCP in an isolated VS Code profile.
For these takes, Deslop uses the same cursor, click rings, and automatic
camera as SharpLsp. Reference cards remain for the parts that explain concepts.
The recut preserves the narration, score, captions, and chapter timing.

- [x] Review the SharpLsp walkthroughs and use their capture and camera modules.
- [x] Record live VS Code with the packaged Deslop extension and real fixtures.
- [x] Verify visible clicks, source navigation, and both comparison workflows.
- [x] Verify Osprey scan output, MCP responses, and a real threshold failure.
- [x] Verify all recut assets on the website before release.

With the Videos and SharpLsp repositories beside Deslop, run
`node scripts/videos/shoot-doc-footage.mjs <packaged.vsix>`. Run it again with
`DESLOP_FILM_CONTEXT` set to each of `finding`, `files`, `commands`, `mcp`
and `mcp-config` to capture the remaining footage. The terminal takes use
`target/release` for Deslop CLI and MCP binaries.

With the original narrated masters in Videos, run
`node scripts/videos/cut-doc-footage.mjs` to stage the cuts in
`target/docs-live/delivery`. Review the edits, then run the same command with
`--install` to verify and install the media. The
[live footage inventory](../../reports/videos/live-footage.md) records coverage.

## Motion and agent recut [DOC-FILMS-MOTION]

Keep the approved narration and original caption/chapter timestamps. Every
chapter combines animated explanation with relevant real application footage;
none of the old static reference slides remains. Motion sources are in Videos
at `projects/deslop/videos/doc-series/motion.html` and `motion.js`.

- [x] Record packaged-extension navigation, comparisons and terminal demonstrations.
- [x] Record real pi Deslop MCP calls with returned duplicate and similarity findings.
- [x] Render all chapter illustrations through Videos' shared frame renderer.
- [x] Check SharpLsp camera moves and inspect finished pi tool/result frames.
- [x] Verify all delivered video streams and unchanged audio hashes.
- [ ] Deploy the tested website and verify public media against local hashes.
