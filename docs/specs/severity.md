# Diagnostic severity

### [SEVERITY-MODEL] Severity means diagnostic severity

Severity controls the editor's diagnostic level. It is separate from clone weight and ranking, and never changes duplication percentages.

### [SEVERITY-DESLOP-MAP] Defaults when diagnostics are enabled

| Category | Default diagnostic |
|---|---|
| Identical code | Warning |
| Nearly identical code | Warning |
| Same behavior, different code | Information |
| Similar code | Information |
| Same shape, different content | None |

One shared map owns these defaults. A clone's size or rank does not change its diagnostic level.

### [SEVERITY-DIAGNOSTICS] Resolve the configured level

Use the finding's kind and `deslop.diagnostics.severityByKind`. Missing entries use the defaults above. `none` means no diagnostic; `hint`, `information`, `warning` and `error` map directly to the editor's levels.

### [SEVERITY-DIAGNOSTICS-STRUCTURAL-ONLY] Shape-only is silent by default

Shape-only produces no diagnostic, including when clone diagnostics are enabled. The user may explicitly override `structural_only` to any diagnostic level. Its clone status still follows [CLONE-BUCKETS-STRUCTURAL-ONLY](taxonomy.md#clone-buckets-structural-only-shape-only-is-not-duplication).

### [SEVERITY-DIAGNOSTICS-GATE] Master switch

`deslop.diagnostics.enabled` defaults to `false`. When false, publish no Deslop diagnostics, including explicit severity overrides. Findings remain available for inspection and navigation.

### [SEVERITY-COLOR] Diagnostic colours

The editor owns diagnostic colours. Deslop's category colours are defined separately in [CLONE-KIND-COLOR](taxonomy.md#clone-kind-color-colour-describes-the-category).

### [SEVERITY-CONFIG] Configuration

```json
{
  "deslop.diagnostics.enabled": false,
  "deslop.diagnostics.severityByKind": {
    "identical": "warning",
    "nearly_identical": "warning",
    "same_behavior": "information",
    "loosely_similar": "information",
    "structural_only": "none"
  }
}
```

Accept only these five kind keys and the levels `none`, `hint`, `information`, `warning`, `error`. Reject unknown keys or levels. Apply user/workspace precedence per entry. Configuration changes refresh or clear diagnostics without re-analysis, in both pull and workspace-push modes. Diagnostic scope and the master switch still apply.

### [SEVERITY-TESTING] Required checks

Verify each default and override, including `none`, omitted entries, invalid configuration and the master switch. Defaults must hold at every clone rank. Pull and push modes must agree and refresh when settings change. No diagnostic setting may change clone weight, rank, counts, percentages or category order.
