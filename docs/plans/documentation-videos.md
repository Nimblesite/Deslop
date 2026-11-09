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
- [ ] Remove disposable render intermediates and check disk space.

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
receipt is [site-validation.md](../../reports/videos/site-validation.md). Repository
cleanup and remote upload verification remain.

Reviewed production scripts live in Videos under `projects/deslop/videos/docs-*/`.
Superseded pi drafts were removed after review; model outcomes and corrections
are recorded in PiNotes/2026-11-09-deslop-doc-video-scripts.md.

## Final validation [DOC-FILMS-VALIDATION]

The browser checks cover both language routes, widths from 320px to 1440px,
actual playback and seeking at phone and desktop sizes, caption/chapter tracks,
and keyboard transcript expansion. Native controls remain visitor-operated.
Audio encoding and level measurements do not establish subjective pronunciation
quality; narration was reviewed as text against the corresponding documentation.
