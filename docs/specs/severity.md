# Severity model

> **Status: ⏳ Partly shipped, remainder planned (#177).** [SEVERITY-DESLOP-MAP] and [SEVERITY-COLOR] **ship**: `clients/vscode/src/severity.ts::resolveSeverity` resolves the two channels and every VSIX paint surface consumes it. What remains planned is the *configurability* — the `deslop.severity.*` / `deslop.diagnostics.severity.*` settings, the master gate, and the percentile floors; today the LSP publishes every cluster at its bucket default with no gate. See [LSP-SEVERITY-BUCKET](lsp.md#lsp-severity-bucket).
>
> **The VSIX previously coloured by rank percentile, and that was a defect, not a staging decision.** Colour and glyph density are two channels ([SEVERITY-COLOR]); driving both from the ranking left the colour channel carrying no bucket information at all. On a report whose top cluster was a content-gated `StructuralOnly` family, the paint came out *inverted*: the demoted family rendered crimson — the "safe to extract" colour — while the byte-proven `Identical` clone one rank below it rendered blue. Pinned by `severity.unit.test.ts::a demoted shape-only family is not painted with act-now severity`.

Deslop owns a **severity** concept that is independent of any one editor surface. A cluster's severity is derived from its **bucket** ([taxonomy.md §CLONE-BUCKETS](taxonomy.md#clone-buckets)) through user-configurable maps, and is then **projected** onto two surfaces that consume it differently. This file is the single source of truth for that model; [lsp.md §LSP-SEVERITY](lsp.md#lsp-severity) describes the diagnostic projection and [vsix.md §VSIX-SEVERITY-CONTROL](vsix.md#vsix-severity-control) describes the in-panel UI that drives it.

### [SEVERITY-MODEL] One identity, two maps, two projections

The bucket is the identity ([CLONE-BUCKETS-DUAL-LABEL](taxonomy.md#clone-buckets-dual-label)). Severity is resolved at render time rather than stored on a cluster, so settings can recolour and republish without re-analysis ([VSIX-VIEW-STATE-UI-ONLY](vsix.md#vsix-view-state-ui-only)).

Two maps key off the bucket, deliberately separated because the user asked for both knobs:

| Map | Setting prefix | Values | Always active? | Drives |
|---|---|---|---|---|
| **Deslop severity** | `deslop.severity.*` | `error · warning · information · hint` | **Yes** — colour is never silenced | Bubble / tree dot / code-lens glyph / gutter decoration **colour** on pure-visual surfaces ([SEVERITY-COLOR]) |
| **Diagnostic severity** | `deslop.diagnostics.severity.*` | `error · warning · information · hint · none` | **No** — gated off by default | The VS Code **Problems panel** and squiggle, via the LSP ([SEVERITY-DIAGNOSTICS]) |

The maps are independent: visual surfaces remain coloured while the Problems panel stays quiet by default.

### [SEVERITY-DESLOP-MAP] Deslop severity — drives colour, always on

Every bucket resolves to one of four levels. There is **no `none`** here: colour is how the live surfaces communicate severity, and a cluster that exists is always coloured. Suppressing a cluster on a visual surface is the job of the dirty projection ([VSIX-STATE-DIRTY]) and silence-when-clean ([VSIX-PRINCIPLES] principle 2), never of the severity map.

| Bucket | Default level | Colour band |
|---|---|---|
| `Identical` | `error` | red |
| `NearlyIdentical` | `warning` | amber |
| `LooselySimilar` | `information` | blue |
| `StructuralOnly` | `hint` | muted |
| `SameBehavior` | `hint` | grey |

**`error` is reserved for `Identical`, the only bucket whose evidence is byte proof.** Severity paints how strong the measured evidence is, never what to do about it ([PRINCIPLES-REPORT-NOT-DICTATE](principles.md#principles-report-not-dictate)). `StructuralOnly` sits at `hint` with the muted/outline band [taxonomy.md §CLONE-BUCKETS](taxonomy.md#clone-buckets) already assigns it: its evidence sentence records that the content did *not* agree, so a family the content gate demoted ([FUSED-CONTENT-GATE](fused.md)) must never wear the paint reserved for proof — however heavily it ranks. `severity.unit.test.ts` asserts the reservation directly, so a future remap cannot hand crimson back to an unvouched bucket.

Configurable per bucket via `deslop.severity.identical` / `.nearlyIdentical` / `.looselySimilar` / `.structuralOnly` / `.sameBehavior`. Lowering `Identical` to `hint` greys its bubble; raising `SameBehavior` to `error` paints its bubble red. The map only changes presentation — it never reorders the worst-first ranking ([pipeline.md §PIPELINE-RANK-WORST-FIRST](pipeline.md#pipeline-rank-worst-first)).

### [SEVERITY-DIAGNOSTICS] Diagnostic severity — drives the Problems panel, off by default

The diagnostic map projects each bucket onto an LSP `DiagnosticSeverity`, with one extra value — `none` — that suppresses the diagnostic for that bucket entirely.

| Bucket | Default | Can be set to |
|---|---|---|
| `Identical` | `error` | `warning · information · hint · none` |
| `NearlyIdentical` | `warning` | `error · information · hint · none` |
| `LooselySimilar` | `warning` | `error · information · hint · none` |
| `SameBehavior` | `warning` | `error · information · hint · none` |

These per-bucket defaults **coincide** with [SEVERITY-DESLOP-MAP] (`Identical → error`, the rest `warning`) so a user who simply enables diagnostics gets Problems severities that match the bubble colours with zero further configuration. The two maps may diverge deliberately — e.g. keep `Identical` red on screen (`deslop.severity.identical = error`) but publish it only as a `warning` (`deslop.diagnostics.severity.identical = warning`) so it does not fail an `error`-gated CI lint.

### [SEVERITY-DIAGNOSTICS-GATE] Master gate — diagnostics default OFF

A single boolean, **`deslop.diagnostics.enabled`, defaults to `false`.** With it off, **no clone diagnostics are published** regardless of the per-bucket map — the Problems panel and squiggle gutter stay empty. The live bubble, Top Offenders tree, code lens, and hover are unaffected and remain fully populated and coloured; the bubble is still the flagship in-your-face surface ([VSIX-PRINCIPLES] principle 1).

Diagnostics are opt-in because the bubble and tree already surface duplication, while publishing every existing offender would flood the Problems panel. [VSIX-SEVERITY-CONTROL] provides the toggle.

A diagnostic is published for an occurrence iff **all three** hold, in order:

1. `deslop.diagnostics.enabled` is `true` (the gate), **and**
2. the bucket's `deslop.diagnostics.severity.*` is not `none`, **and**
3. the cluster meets its severity's percentile floor ([lsp.md §LSP-SEVERITY-PERCENTILE](lsp.md#lsp-severity-percentile)).

The gate and the map compose without overlap: the gate is the global on/off the user flips constantly; the map is the per-bucket shape they set once. `crates/deslop-lsp/src/diagnostics.rs` is the single resolver for all three checks — every client (VSIX, Neovim, Helix, agents) consumes the published diagnostics rather than recomputing the gate.

### [SEVERITY-COLOR] Colour projection — orthogonal to the percentile glyph

Two visual channels carry two orthogonal facts on every cluster row and bubble:

- **Colour** = the cluster's Deslop severity ([SEVERITY-DESLOP-MAP]). Answers *how alarming is this kind of duplicate*: red / amber / blue / grey.
- **Glyph density** = the cluster's weight percentile ([lsp.md §LSP-SEVERITY-PERCENTILE](lsp.md#lsp-severity-percentile)). Answers *how big an offender is this specific cluster*: `●●` (worst) · `●` (top 10%) · `◐` (top 50%) · `○` (rest).

A faint identical clone therefore renders as a red `○`, while a high-impact loosely-similar cluster renders as a blue `●●`. `resolveSeverity(cluster)` in `clients/vscode/src/severity.ts` is the single resolver for every visual surface ([VSIX-PRINCIPLES] principle 6). It returns **both** channels together — `{ level, band }` — precisely so a caller cannot reach for one and silently render the other fact. Both channels are *read*, never derived: the level maps the engine's bucket label, the band is the engine's `rank_band` ([SEVERITY-BAND]).

**Neither channel may answer for the other, and the percentile channel structurally cannot.** The band is monotonic down the ranking by construction, so any answer it gives for one demoted cluster it must give for every cluster below it too; asking it to also encode the bucket is unsatisfiable, not merely unimplemented. That is why the demotion evidence lives in colour. A content-gated family topping the report renders as a muted `●●`: loud about *impact*, quiet about *kind*. Pinned across the two languages that own the two channels: `report_weight::rank_band_never_brightens_down_the_report` (band) and `severity.unit.test.ts::the colour channel is a pure function of the bucket, at every band` (level) hold simultaneously only because the channels are separate.

### [SEVERITY-BAND] The band is the engine's, computed once

Glyph density is a *classification of a rank percentile*, which makes it a calculation and therefore the engine's ([PRINCIPLES-ONE-CALCULATION](principles.md#principles-one-calculation)). `report_weight::stamp_ranks` runs at the end of the worst-first sort and writes two fields onto every rendered cluster:

| Field | Value |
|---|---|
| `rank` | one-based position in the report's worst-first list ([PIPELINE-RANK-WORST-FIRST](pipeline.md#pipeline-rank-worst-first)) |
| `rank_band` | `worst` · `top10` · `mid` · `faint` |

The percentile is `1 - (rank - 1) / (total - 1)`, and `0` for a single-cluster report — one cluster has no spread to express. The cut points are `≥ 0.99 → worst`, `≥ 0.9 → top10`, `≥ 0.5 → mid`, otherwise `faint`. All of it is held by `report_weight::rank_band_cut_points`.

Both fields are restamped wherever the carried list changes: after a diff-scoped filter (`diff_scope::tag::apply_only_changed`) and when a report is replayed from disk (`report_restamp::restamp_derived_fields`). Consumers render `rank` and `rank_band` verbatim. Re-numbering from array position is prohibited and was a shipped defect: any client that filtered or projected the list before numbering rebanded every cluster in it, so a `faint` cluster in a two-item filtered view rendered as the repository's worst. An empty `rank_band` — a report written before the field existed — reads as `faint`.

### [SEVERITY-CONFIG] Configuration surface

All severity settings are VS Code workspace settings under `deslop.*`, forwarded to the LSP at `initialize` and on `workspace/didChangeConfiguration`, and hot-reload with no restart ([vsix.md §VSIX-SETTINGS](vsix.md#vsix-settings)). They are window-scoped (`scope: "window"`) so a repo can pin a team posture in `.vscode/settings.json` while individuals keep machine defaults, matching the Top Offenders view-state settings. Unknown / missing values fall back to the defaults in the tables above — never panic, never write user settings back.

The CLI and HTML report are **not** governed by these maps in v1: their colour comes from the static `[CLONE-BUCKETS]` colour band, and they have no Problems panel. Per-`.deslop.toml` severity overrides for the CLI are a possible follow-up, explicitly out of scope here to keep the live-surface model focused.

### [SEVERITY-TESTING] Acceptance

Coarse E2E only, per CLAUDE.md, against the real LSP binary and a real extension host:

- **Default posture:** open a fixture workspace with known clones; assert the Problems panel is **empty** (gate off) while the Top Offenders tree and bubble are **fully populated and coloured** (`Identical` row resolves to the red colour token).
- **Gate on:** flip `deslop.diagnostics.enabled = true`; after one config-change round-trip, assert diagnostics appear with `Identical → Error`, others `→ Warning`, and that turning the gate back off clears them all.
- **Map remap:** set `deslop.diagnostics.severity.identical = none`; assert identical clusters lose their Problems entries while their bubble/tree colour is unchanged. Set `deslop.severity.sameBehavior = error`; assert the same-behavior bubble repaints red without touching the Problems panel.
- **Decoupling:** with `deslop.severity.identical = error` and `deslop.diagnostics.severity.identical = warning`, assert the bubble is red but the published diagnostic severity is `Warning`.
