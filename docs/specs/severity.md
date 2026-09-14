# Severity model

### [SEVERITY-MODEL] Severity is a projection of mass rank

Cluster severity communicates duplicated impact. It is derived only from the engine-stamped mass rank band. Structural, Jaccard, embedding, content, rename, and literal values are forbidden inputs because they belong to concrete pairs, and the clone kind ([CLONE-KIND-FOLD]) is a separate channel that never changes severity.

Severity is presentation metadata, not a second weight. It never changes cluster mass, rank, membership, kind, or repository metrics.

### [SEVERITY-DESLOP-MAP] One fixed mass-band map

| Rank band | Severity |
|---|---|
| `worst` | `error` |
| `top10` | `warning` |
| `mid` | `information` |
| `faint` | `hint` |

The map is shared by the CLI, HTML, LSP, VSIX, and agent surfaces. Consumers read `rank_band` and do not recompute percentiles.

### [SEVERITY-DIAGNOSTICS] Diagnostic severity uses the same map

When diagnostics are enabled, the LSP projects [SEVERITY-DESLOP-MAP] into the editor's diagnostic enum. There is no bucket-specific or pair-specific override. A disabled diagnostic does not remove the cluster from other surfaces.

### [SEVERITY-DIAGNOSTICS-GATE] Diagnostics default off

`deslop.diagnostics.enabled` defaults to `false`. When false, the LSP publishes no Deslop diagnostics. Code lenses, cluster navigation, the Top Offenders tree, and explicit pair comparison remain available.

An optional mass-percentile floor may suppress diagnostics below a configured impact threshold. The floor consumes the engine-stamped mass percentile only and cannot inspect pair evidence.

### [SEVERITY-COLOR] The glyph follows mass severity; colour follows the clone kind

Severity has one visual channel on cluster surfaces: glyph density (`●●`, `●`, `◐`, `○`) for `worst`, `top10`, `mid`, and `faint`. Colour belongs to the other channel, the clone kind ([CLONE-KIND-COLOR]): crimson means byte-identical code, never "biggest finding". The two channels are never mixed, so a rank-1 cluster that only shares shape is muted and a faint byte-identical cluster is crimson.

Diagnostics keep the host editor's own severity colours for `Error`, `Warning`, `Information`, and `Hint` ([SEVERITY-DIAGNOSTICS]); those are the Problems panel's, not Deslop's paint.

### [SEVERITY-BAND] The engine computes the band once

After sorting by [RANK-MASS-SUM], the engine stamps every cluster with one band:

| Band | Population |
|---|---|
| `worst` | Top 1% by mass rank, at least the first cluster when non-empty. |
| `top10` | Remaining top 10%. |
| `mid` | Remaining top 50%. |
| `faint` | Remaining clusters. |

The thresholds are applied to stable one-based rank over the full report. Filtering a client-side view does not renumber or recolour clusters.

### [SEVERITY-CONFIG] Configuration surface

```json
{
  "deslop.diagnostics.enabled": false,
  "deslop.diagnostics.massPercentileFloor": 0
}
```

The floor is finite and in `[0, 100]`. The retired per-bucket severity maps are invalid configuration.

### [SEVERITY-TESTING] Acceptance

Tests assert that equal-mass ordering uses cluster id, rank bands never brighten down the report, every cluster surface uses the engine-stamped band for its glyph, diagnostics-off publishes nothing, no pair evidence changes severity, and the band never chooses a colour ([CLONE-KIND-TESTING]).
