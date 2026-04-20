# Design System Specification: The Kinetic Manuscript
 
## 1. Overview & Creative North Star
**Creative North Star: The Kinetic Manuscript**
 
This design system is built for speed, precision, and academic authority. It avoids the "bubbly" playfulness of consumer apps, opting instead for a high-density, editorial aesthetic inspired by technical whitepapers and high-end CLI tools (Rust, Turbo, Stripe). We treat code deduplication not just as a utility, but as a formal science.
 
The visual identity breaks the "standard template" look through **Kinetic Asymmetry**—using extreme typographic scales and overlapping technical data to create a sense of movement and "lightning-fast" processing. We prioritize information density over white space, but we manage that density through rigorous tonal hierarchy rather than structural clutter.
 
---
 
## 2. Color & Tonal Architecture
The palette is rooted in deep obsidian tones, punctuated by a high-energy "Crimson Executioner" accent.
 
### Surface Hierarchy & Nesting
We reject the flat grid. Instead, we use **Tonal Layering** to define depth. The UI should feel like a series of physical layers—stacked sheets of tinted glass or fine industrial paper.
*   **Base:** `surface` (#131313) for the main application background.
*   **Recess:** `surface_container_lowest` (#0e0e0e) for "well" areas like the terminal output or code blocks.
*   **Elevation:** `surface_container_high` (#2a2a2a) and `highest` (#353534) for floating panels and interactive cards.
 
### The "No-Line" Rule
**Explicit Instruction:** Do not use 1px solid borders for sectioning or containment. 
Boundaries must be defined solely through background color shifts. For example, a `surface_container_low` section sitting on a `surface` background creates a natural, sophisticated edge without the visual "noise" of a line.
 
### The "Glass & Gradient" Rule
To move beyond a "generic" dark mode, use Glassmorphism for floating overlays (e.g., Command Palettes). 
*   **Effect:** Apply `surface_container_highest` at 80% opacity with a `20px` backdrop-blur. 
*   **Gradients:** Use subtle linear gradients from `primary` (#ffb4aa) to `primary_container` (#b3261e) for primary CTAs and progress bars to provide a "liquid" feel to the speed of the CLI.
 
---
 
## 3. Typography
The system uses a dual-font strategy to balance UI clarity with technical authority.
 
*   **UI Foundation:** **Inter** (Sans-Serif). Used for all functional interface elements, labels, and navigation. It is invisible, efficient, and modern.
*   **Technical Data:** **JetBrains Mono** (Monospace). Used for code, file paths, hashes, and performance metrics (e.g., "0.003ms").
 
### Typographic Hierarchy
*   **Display (Large/Medium):** Use for "Impact Stats" (e.g., "4.2GB Saved"). These should be tightly tracked and bold to convey authority.
*   **Label (Medium/Small):** Use JetBrains Mono for these. Even non-code data—like version numbers or status tags—should use Mono to lean into the "Academic-Cool" aesthetic.
*   **Body:** Inter at `0.875rem` (`body-md`) is our workhorse. Keep line heights tight (1.4) to maintain the high-density professional look.
 
---
 
## 4. Elevation & Depth
Depth is achieved through light and layering, never through heavy shadows.
 
*   **The Layering Principle:** Stack `surface-container` tiers. A `surface_container_lowest` card placed on a `surface_container_low` section creates a "sunken" effect, perfect for input areas or terminal windows.
*   **Ambient Shadows:** For floating elements (Modals, Popovers), use extra-diffused shadows: `box-shadow: 0 20px 40px rgba(0,0,0, 0.4)`. The shadow color should never be pure black; it should be a tinted version of `surface_container_lowest`.
*   **The "Ghost Border" Fallback:** If a border is strictly required for accessibility (e.g., input focus), use the **Ghost Border**: `outline_variant` (#5a403d) at 20% opacity. 100% opaque borders are strictly forbidden.
 
---
 
## 5. Components
 
### Buttons (The Kinetic Trigger)
*   **Primary:** `primary_container` background with `on_primary_container` text. Use `sm` (0.125rem) roundedness for a sharp, technical feel. No icons unless necessary for clarity.
*   **Secondary:** `surface_container_highest` background. Text in `on_surface`.
*   **Tertiary:** Transparent background with `primary` text. No border.
 
### Terminal Cards
Instead of traditional cards, use "Terminal Blocks."
*   **Background:** `surface_container_low`.
*   **Header:** A small top bar using `surface_container_high` containing the file path in `label-sm` (JetBrains Mono).
*   **Content:** No dividers. Use `1.5rem` vertical padding to separate content blocks.
 
### Data Grids & Lists
*   **Constraint:** No horizontal or vertical divider lines.
*   **Separation:** Use alternating row fills (`surface` vs `surface_container_low`) or simply whitespace. 
*   **Interaction:** On hover, change the row background to `surface_container_highest` to create a "highlight" effect.
 
### Chips (Status Indicators)
*   **Aesthetic:** Rectangular, sharp corners (`none` or `sm`). 
*   **Color:** Use `error_container` (#93000a) for deleted duplicates and `tertiary_container` (#00619e) for kept files.
 
---
 
## 6. Do's and Don'ts
 
### Do:
*   **Use Monospace for Numbers:** Any time a number appears (count, size, time), use JetBrains Mono. It aligns the decimals and feels more "calculated."
*   **Embrace Asymmetry:** Align high-level stats to the right while labels stay left. Break the standard centered layout.
*   **Use Crimson as a Surgical Tool:** Use the red accent (#b3261e) only for the most important actions or "deletion" stats. Overusing it dilutes its authority.
 
### Don't:
*   **Don't use "Soft" UI:** Avoid large corner radii (keep to `sm` or `none`). Avoid vibrant blues or greens that look like consumer SaaS.
*   **Don't use 1px Dividers:** If you feel the need to separate two things, use a 16px or 24px gap or a subtle background shift. Lines are a sign of weak hierarchy.
*   **Don't hide complexity:** This is a tool for developers. Show the SHA-256 hashes, show the millisecond timings. Professionalism comes from transparency, not over-simplification.