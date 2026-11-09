# Documentation film series [DOC-FILMS]

Create a narrated film for each of the ten English documentation pages, using
Videos' existing Osprey production templates and shared speech/rendering pipeline.
Keep Deslop branding, real source examples, accurate qualifications and readable
captions. Short reference films are explainers; task tutorials demonstrate a real
run and a change. Do not pad films to reach a duration.

## Delivery checklist [DOC-FILMS-DELIVERY]

- [x] Inventory all ten documentation pages and inspect the shared film pipeline.
- [ ] Review narration against each source page and choose real visual evidence.
- [ ] Produce a shared Deslop series template with distinct chapter compositions.
- [ ] Render each film with narration, captions, chapters and a poster.
- [ ] Inspect finished frames and verify narration, duration and media encoding.
- [ ] Integrate accessible video players into the matching documentation pages.
- [ ] Store delivered videos in Git LFS; verify desktop and mobile playback.
- [ ] Remove disposable render intermediates and check disk space.

## Coverage [DOC-FILMS-COVERAGE]

| Documentation source | Film focus | Delivery |
| --- | --- | --- |
| index.md | Install, find, inspect and compare | Script reviewed; four previews checked; rendering |
| ai-integration.md | Check for existing code before writing | Script reviewed; preview rendered |
| vscode-cluster-panel.md | Navigate findings and compare copies | Pending |
| github-action.md | Review duplicates in a pull request | Pending |
| osprey.md | Analyse supported Osprey source | Pending |
| configuration.md | Choose analysis settings deliberately | Script corrected; preview checked; narration generating |
| how-it-works.md | Follow source code into ranked findings | Script corrected; preview rendering |
| research-background.md | Understand clone types and evidence | Pending |
| accuracy-transparency.md | Read accuracy evidence and limitations | Pending |
| for-ai.md | Use the live MCP workflow correctly | Pending |

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
