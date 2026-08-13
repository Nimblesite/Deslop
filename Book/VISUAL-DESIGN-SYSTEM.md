# Visual design system — The Kinetic Audit

## Creative north star

The book adapts Deslop's **Kinetic Manuscript** into **The Kinetic Audit**: a technical field guide whose pages feel like evidence sheets moving through an exact review process.

The visual narrative has two verbs:

- **Intercept** — a proposed duplicate stops before entering the repository.
- **Consolidate** — several verified copies resolve into one canonical implementation.

The result should feel academic, urgent, and operational. It must not resemble a soft consumer app, a generic cyberpunk dashboard, or a stack of marketing cards.

## Evidence hierarchy

1. **Direct product captures** show what Deslop actually displayed.
2. **Deterministic diagrams** explain why an authoring or cleanup decision follows.
3. **Generated editorial illustration** may open a part or express the intercept/consolidate metaphor without factual content.

Illustration never substitutes for evidence. Exact code, labels, commands, report values, paths, and typography are deterministic text or direct captures.

## Palette

The EPUB defaults to warm paper for long-form reading while retaining Deslop's obsidian evidence surfaces.

| Role | Hex | Use |
|---|---|---|
| Audit paper | `#fafaf7` | Page background |
| Raised paper | `#ffffff` | Quiet evidence cards |
| Paper recess | `#f1efe8` | Notes, alternating rows |
| Obsidian | `#0b0d10` | Cover, code, terminal, report evidence |
| Raised obsidian | `#13171c` | Dark tonal layer |
| Primary ink | `#14161a` | Body text |
| Muted ink | `#55606b` | Metadata and captions |
| Verdict crimson | `#b3261e` | Prevention stop, worst offender, deletion consequence |
| Link blue | `#0b5fff` | Navigation and neutral interaction |
| Verified green | `#1a7f37` | Rare proved-clean state |
| Review amber | `#9a6700` | Evidence requiring inspection |

Crimson is surgical. It marks the one thing that should stop or the one worst finding that should command attention. Blue means navigation or neutral flow, never success.

## Typography

- Display and headings: Inter, Inter Tight, or a metrically compatible system sans
- Body: Inter or the reader's EPUB-safe sans stack
- Code and evidence: JetBrains Mono, Fira Code, SF Mono, or monospace
- Numbers use tabular figures
- Eyebrows are uppercase monospace with wide tracking

Titles use tight tracking and intentional line breaks. Evidence labels remain readable at a 320 px thumbnail. Generated imagery never owns text.

## Tonal layers, lines, and shape

Prefer background shifts and spacing over decorative containers. The book may use a restrained one-pixel rule where EPUB rendering or accessibility requires an explicit boundary, but it never builds a page from outlined cards.

- Corners are square or at most 4 px.
- Code blocks are square.
- Shadows are diffused and reserved for floating editorial layers.
- Gradients are allowed only to describe movement through a process, not as decoration.
- No pills, oversized radii, emoji, icon packs, mascots, or stock photography.

## Canvas families

| Asset | Master | Publication derivative |
|---|---|---|
| Cover | 1600 × 2560 SVG | 1600 × 2560 PNG |
| Concept diagram | 1600 × 1000 SVG | 1600 × 1000 PNG |
| Product screenshot | Native high-DPI capture | 1600 px-wide crop where practical |
| Part opener | 16:9 raster or SVG | 1600 px-wide PNG/WebP |

Keep at least 72 px of safe margin around diagram content. Publication images are opaque.

## Diagram language

| Idea | Form |
|---|---|
| Proposed code | Warm manuscript sheet entering from the left |
| Repository | Obsidian layered corpus |
| Prevention check | Crimson gate before the corpus |
| Canonical occurrence | One raised sheet with a blue anchor |
| Duplicate group | Offset sheets sharing one vertical spine |
| Strong evidence | Solid connector with an explicit numeric label |
| Borderline evidence | Dashed connector plus “read both” instruction |
| Cleanup | Several layers converging into one retained layer |
| Verification | Return loop through analysis, static checks, and tests |
| Quality ceiling | Horizontal audit plane that ratchets downward |

Every node contains evidence or an action. A box that says only “quality” teaches nothing.

## Product capture contract

- Capture the edition's pinned artifacts in a clean fixture workspace.
- Record OS, architecture, editor, extension, theme, zoom, Deslop version, fixture, and capture method in `figures.json`.
- Keep untouched masters under `assets/screenshots/masters/`.
- Crop and uniformly resize; never repaint product pixels or text.
- Put explanatory callouts outside the captured product area.
- Re-capture when wording, ranking, field shape, or surface behavior changes.

## Cover direction

The cover is a tall audit field: obsidian ground, a vertical evidence spine, repeated manuscript fragments above the midpoint, and one canonical fragment below it. A crimson interception plane separates prevention from cleanup. The real Deslop mark is referenced from the canonical design asset rather than redrawn.

Required text:

```text
THE DESLOP BOOK
Prevent duplicate code before agents write it. Clean up what is already there.
```

The title must remain readable at 160 px. No fake IDE, terminal, source listing, laptop, person, or generated lettering.

## Accessibility and production gates

Every ready visual must have:

- descriptive alt text explaining the lesson;
- a caption explaining why the figure exists;
- a readable 320 px thumbnail;
- sufficient contrast and a grayscale-safe distinction;
- no information carried by color alone;
- a source master and exact dimensions;
- no personal paths, secrets, or private repository names;
- no fictional product output; and
- a matching entry in `figures.json`.
