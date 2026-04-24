---
layout: layouts/docs.njk
title: Design System
eleventyNavigation:
  key: Design System
  order: 9
icon: palette
---

# Design System

> Duplication is the tax LLMs charge for speed. Deslop is the audit.

This document is the single source of visual, typographic, and interaction truth for Deslop's web presence. It is authored with the same rigour as the engine it fronts: every token has a reason, every reason traces to a principle, every principle serves one goal — **making the worst duplication in a repository impossible to ignore**.

## Voice & Positioning

Deslop is a **live duplicate-code analysis server** — an LSP + MCP server that runs in the workspace and streams real-time clone signals to the human's editor and the AI coding agent driving that editor (Claude Code, Cursor, Copilot, Continue, Codex…). Coding agents generate plausible code at a rate that outpaces human review; a human reviewer's instinct for "I have seen this before" does not scale past a few hundred thousand tokens of generated surface area, and a batch CI report arrives after the copy-paste has already landed. Deslop restores that instinct at the instant the keystroke happens — mechanically, deterministically, and fed back to both audiences over the same running engine.

The CLI is the cold-cache fallback. The *server* is the product.

The brand voice is **academic in construction, urgent in tone**. Read it aloud and it should sound like a terse technical paper written by someone who has just watched a well-meaning agent copy-paste the same repository three times. Precise. Unsparing. Confident. Never cute.

### Primary claims

- **Live server, not a batch scanner.** A long-running process (LSP for editors, MCP for agents) that re-analyses on every keystroke within a debounce budget — not a script that prints a report and exits.
- **Feeds AI agents mid-generation.** Over MCP, the agent can ask *"is something like this already in the repo?"* before writing a single token. Duplication gets prevented, not audited.
- **Feeds humans inline.** Over LSP, the editor lights up the cluster at end-of-line while the developer is still typing.
- **Same engine on every surface.** LSP, MCP, CLI, webview — one `deslop-core` pipeline, one cache, one schema. Cold start and hot-path are the same code.
- **Worst-first ranking.** Score = clone_size × clone_count × spanned_LOC. The top of the live view is always where the largest payoff lives.
- **Tree-sitter, not regex.** Four duplication categories — Identical, Nearly identical, Loosely similar, Same behavior — detected through AST fingerprints and embedding fusion, never line-matching.
- **Structured output as product.** JSON is canonical; text and HTML renderers are views. Agents consume the same schema humans read.

### Forbidden phrasings

- "Duplicate-code detector." Too passive — implies a batch scanner. Say *live duplicate-code server* or *live analysis engine*.
- "Runs on your repo." Implies a one-shot scan. Say *runs in your workspace* or *lives alongside your editor*.
- "Solves duplication." It reveals duplication, on the keystroke. Acting on the finding is still the developer's or agent's job.
- "AI-powered." The tool uses embeddings; it is not sold on hype. Lead with the technique, not the buzzword.
- Exclamation marks in product copy. They undermine authority. Reserve urgency for verbs.

## Principles

### 1. Information density wins

Marketing sites for developer tools fail by padding. A developer evaluating Deslop wants to know, inside thirty seconds: what it does, what it parses, how fast it runs, what it costs. Every screen answers at least one of those questions above the fold.

### 2. Monospace is load-bearing

This tool reports on source code. Source code is monospace. The typographic system treats monospace as a first-class body font, not a decoration. Headlines may be proportional; evidence is always fixed-width.

### 3. The report is the hero

Screenshots of a ranked Deslop report carry more conviction than any illustration. Marketing surfaces show the real output: the same three columns (score, file, span) a developer will see five minutes after install.

### 4. Two audiences, one surface

Every page must read naturally to both a human skimming for credibility and an AI agent ingesting via `llms.txt`. That rules out: ambiguous metaphors, image-only information, and decorative text that does not survive stripping to plain prose.

### 5. Ruthless restraint

No gradients as decoration. No stock photography. No hero videos. No emoji. No particle animations. If a visual element does not narrow the distance between the reader and the `deslop` binary, it is cut.

## Color System

Two palettes, both pinned to WCAG AA contrast at every pairing used. The default palette is light; dark mode is a first-class citizen and is selected via the `[data-theme="dark"]` attribute handled by the bundled theme toggle.

### Tokens

| Token | Light | Dark | Usage |
| --- | --- | --- | --- |
| `--color-bg` | `#fafaf7` | `#0b0d10` | Page background |
| `--color-surface` | `#ffffff` | `#13171c` | Cards, code blocks |
| `--color-surface-alt` | `#f1efe8` | `#191e25` | Quiet panels, striped rows |
| `--color-text` | `#14161a` | `#e8eaed` | Body text |
| `--color-text-muted` | `#55606b` | `#9aa3ad` | Captions, metadata |
| `--color-border` | `#dcd7cc` | `#242a31` | Rules, box outlines |
| `--color-primary` | `#b3261e` | `#ff5a4e` | Verdict-red. Reserved for alarms |
| `--color-primary-ink` | `#7a1813` | `#ff8c85` | Primary text on background |
| `--color-accent` | `#0b5fff` | `#6aa8ff` | Links, interactive affordances |
| `--color-success` | `#1a7f37` | `#4ac26b` | Green states (rare) |
| `--color-warn` | `#9a6700` | `#d4a72c` | Medium-severity clusters |
| `--color-code-bg` | `#1a1d21` | `#0a0c0f` | Code block background (always dark) |
| `--color-code-ink` | `#f0efe9` | `#f0efe9` | Code foreground (always light on dark) |

### Why red primary?

Every competitor in the space — jscpd, Simian, Sonar CPD — leans blue or green, signalling "analysis" or "quality." Deslop reports findings that cost you money, carry bugs, and embarrass code review. The primary is the red of a burnt-edge alarm lamp, not a logo. It appears on the top-1 finding, on the count of clusters-above-threshold, and nowhere else.

## Typography

### Families

- **Display / Headings:** `"Inter"`, `"Inter Tight"`, system-ui, sans-serif. Tight tracking. Weights 600–800.
- **Body:** `"Inter"`, system-ui, sans-serif. Weight 400–500. `line-height: 1.6`. Measure capped at `65ch`.
- **Monospace:** `"JetBrains Mono"`, `"Fira Code"`, `"SF Mono"`, `ui-monospace`, `monospace`. Weight 400–600. Used for every file path, every span, every score, every snippet.

### Scale

Desktop type ramp. Mobile drops each step by one.

| Role | Size | Weight | Tracking |
| --- | --- | --- | --- |
| Display | `clamp(2.4rem, 4vw, 3.75rem)` | 800 | `-0.03em` |
| H1 | `2.25rem` | 700 | `-0.02em` |
| H2 | `1.6rem` | 700 | `-0.015em` |
| H3 | `1.2rem` | 600 | `-0.01em` |
| Body | `1rem` | 400 | normal |
| Small | `0.875rem` | 500 | `0.01em` |
| Mono | `0.9375rem` | 500 | `0` |
| Caps eyebrow | `0.75rem` | 600 | `0.12em`, uppercase |

### Rules

- Headlines may break on intent (`&lt;br&gt;` or `text-wrap: balance`) but never on whitespace by accident. Every H1 and Display is manually balanced.
- No italics in product copy. Reserved for book titles and inline variables in prose.
- Numbers in tables are tabular (`font-variant-numeric: tabular-nums`).
- Uppercase is reserved for eyebrow labels and the top-of-report verdict band.

## Layout

### Grid

- Container max-width: `1180px`.
- Reading measure: `65ch` for long-form; `92ch` for reference/tables.
- Docs layout: `260px 1fr` two-column, collapsing to single column under `880px`.
- Gutter: `clamp(1rem, 3vw, 2rem)`.

### Vertical rhythm

Spacing tokens follow a modular scale rooted at `0.25rem`:

```
--space-1: 0.25rem;
--space-2: 0.5rem;
--space-3: 0.75rem;
--space-4: 1rem;
--space-6: 1.5rem;
--space-8: 2rem;
--space-12: 3rem;
--space-16: 4rem;
--space-24: 6rem;
```

Section padding is `--space-16` top and bottom on desktop, `--space-12` on mobile. Nothing between sections floats; borders or background shifts always mark the boundary.

### Borders and radii

- Border width: `1px` everywhere. No thick outlines.
- Radius: `4px` for inline chips, `8px` for cards, `0` for code blocks. Code blocks are square because terminals are square.

## Components

### Report band

The hero of the home page. A dark, monospaced panel rendering a real Deslop report — exactly as the CLI emits it, with ANSI stripped. Structure:

```
SCORE     FILE                            SPAN          KIND
────────────────────────────────────────────────────────────
  2,184   UserRepository.cs               120–180       Nearly identical
  1,903   ProductRepository.cs            58–118        Nearly identical
    710   EmailValidatorRegex.cs          10–62         Same behavior
```

Top row is marked with `--color-primary` on the score column. No chrome — the panel is the report, not a screenshot of one.

### Evidence block

Inline panel pairing two snippets side-by-side with a signal strip underneath (`structural=1.0 · token_jaccard=0.97 · embedding_cos=0.91`). Used anywhere a clone type is introduced.

### Signal chip

Small uppercase pill with the signal name and score:

```
[ STRUCTURAL 1.00 ]   [ TOKEN 0.97 ]   [ EMBEDDING 0.91 ]
```

Rendered in monospace, 1px bordered, transparent background. Chips are informational only — they never act as buttons.

### Call-to-install

The primary call-to-install is the **VS Code extension** — it bundles the LSP server, the MCP server, and the CLI together, and that's the only install that unlocks the live bubble. A single black-on-cream (or cream-on-black in dark mode) block with the install command, a copy affordance, and no marketing prose around it:

```
code --install-extension deslop-vscode-*.vsix
```

…sourced from the latest [GitHub release](https://github.com/Nimblesite/Deslop/releases/latest). A secondary, smaller line offers the CLI-only paths:

```
brew install nimblesite/tap/deslop
scoop install deslop   # after: scoop bucket add nimblesite https://github.com/Nimblesite/scoop-bucket
```

The home page never promotes `cargo install` — there is no published crate. Other IDE extensions (JetBrains, Zed, Neovim) are "coming soon" and the block should say so in small type.

That is the entire conversion surface on the home page. Everything else is documentation.

### Cluster verdict band

Used on the home page above-the-fold. Three stacked lines, monospace, left-aligned:

```
CLUSTERS DETECTED     142
ABOVE-THRESHOLD        17
TOP OFFENDER       2,184  ← red
```

The band is deliberately austere. It sets the expectation that Deslop speaks in numbers.

## Motion

Motion is used for state changes (theme toggle, nav open/close, copy-to-clipboard feedback) and never for decoration. Duration tokens:

```
--motion-fast: 120ms;
--motion-base: 200ms;
--motion-slow: 320ms;
```

Easing is `cubic-bezier(0.2, 0.7, 0.2, 1)` for enters and `cubic-bezier(0.4, 0, 1, 1)` for exits. Any animation over `320ms` is a bug.

## Accessibility

- Minimum contrast ratio 4.5:1 for body text, 3:1 for large text and UI chrome.
- Every interactive element has a visible focus ring: `2px` solid `--color-accent`, `2px` offset.
- Form controls and toggles are operable by keyboard and announce their state via `aria-pressed` / `aria-checked`.
- Code blocks do not rely on color alone to convey duplication; the Type-1/2/3/4 classification is always repeated in text.
- Respect `prefers-reduced-motion`: fallback to opacity-only transitions.
- No `outline: 0` without a replacement ring. Non-negotiable.

## Iconography

Deslop ships no decorative icons. The only glyphs in use are:

- Chevrons (`›`, `‹`) for navigation affordances.
- `·` (interpunct) as a metadata separator.
- `—` (em dash) as sentence-level punctuation.
- `▲` as the "worst offender" marker in ranked reports, rendered in `--color-primary`.

All four are text characters. No icon font. No SVG sprite sheet. If a concept cannot be communicated in prose, it does not appear on the site.

## Content Patterns

### Headline formula

`<Strong verb> <concrete noun> <quantified outcome>.`

Good: `Surface the worst duplication in a 2M-line repo in under 30 seconds.`
Bad: `Unlock next-generation code quality.`

### Numbers first

Every marketing claim is anchored to a measurable number. If we cannot benchmark it, we do not claim it. Benchmarks live in the docs and link from the home page — never vice versa.

### One CTA per screen

The home page ships exactly one conversion surface: the install command. No "book a demo," no newsletter, no modal. The docs sidebar is the secondary navigation. Everything else is friction.

## Implementation Notes

The site is built with Eleventy and the `eleventy-plugin-techdoc` virtual-theme plugin. The plugin ships no colors and no typography — this design system is the entire visual identity, authored in [`/assets/css/styles.css`](../assets/css/styles.css).

Dark mode is handled by the plugin's theme toggle, which sets `[data-theme="dark"]` on `<html>` and persists preference to `localStorage`. Every token in the Color System table above has a matched dark-mode value; nothing else is themed.

The CSS file is the contract: if a token is added here, it is added there, in the same order. Changes to the design system are reviewed with the same bar as changes to the ranking formula — because both are user-visible and both compound across every page they touch.

---

> Deslop is an audit, not a fix. The design system exists so that nothing on the page distracts from what the audit found.
