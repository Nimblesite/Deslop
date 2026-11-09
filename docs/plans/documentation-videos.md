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
- [ ] Render each film with narration, captions, chapters and a poster.
- [ ] Inspect finished frames and verify narration, duration and media encoding.
- [ ] Integrate accessible video players into the matching documentation pages.
- [ ] Store delivered videos in Git LFS; verify desktop and mobile playback.
- [ ] Remove disposable render intermediates and check disk space.

## Coverage [DOC-FILMS-COVERAGE]

| Documentation source | Film focus | Delivery |
| --- | --- | --- |
| index.md | Install, find, inspect and compare | 82.9s rendered; codec/OCR verified; LFS committed; responsive playback passed |
| ai-integration.md | Check for existing code before writing | 81.2s rendered; all chapter titles checked; English/Chinese playback passed |
| vscode-cluster-panel.md | Navigate findings and compare copies | Script reviewed; previews rendered; narration pending |
| github-action.md | Review duplicates in a pull request | Script reviewed; previews rendered; narration pending |
| osprey.md | Analyse supported Osprey source | Script reviewed; previews rendered; narration pending |
| configuration.md | Choose analysis settings deliberately | 79.5s rendered; imported with LFS; English/Chinese playback passed |
| how-it-works.md | Follow source code into ranked findings | 86.9s rendered; delivery review running |
| research-background.md | Understand clone types and evidence | Script reviewed; previews rendered; narration pending |
| accuracy-transparency.md | Read accuracy evidence and limitations | Script reviewed; previews rendered; narration pending |
| for-ai.md | Use the live MCP workflow correctly | Script reviewed; previews rendered; narration pending |

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
of 18. Final video review and delivery remain outstanding.
