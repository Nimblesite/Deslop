// [VSIX-METRICS-PANEL] Shared duplication-gate status formatting.
// The CI gate (`.deslop.toml [threshold] max_duplication_percent`,
// [EXIT-CODES]) is informational on the live surfaces — it never gates,
// hides, or re-ranks. Both the sidebar Duplication tree and the report
// webview render the gate through this single formatter so the two
// panels stay byte-identical ([UI] "do not duplicate the rendering").

import { formatPercent } from "../types/format";
import { kindTitle, type RepoMetrics, type ThresholdSummary } from "../types/report";

/** Panel-ready verdict for the configured duplication gate. */
export interface ThresholdStatus {
  /** A gate is configured — `.deslop.toml [threshold]` opted in. */
  readonly configured: boolean;
  /** Measured duplication exceeds the configured gate. */
  readonly breached: boolean;
  /**
   * Human label — `⚠ over 20.0% gate` when breached, `✓ within 20.0% gate`
   * otherwise. Empty string when no gate is configured.
   */
  readonly label: string;
}

/**
 * Formats a {@link ThresholdSummary} into a {@link ThresholdStatus}.
 * Returns an unconfigured, empty status when the workspace opted out of a
 * gate (`source === "none"`), so callers render nothing rather than a
 * meaningless `0.0% gate`.
 */
export function thresholdStatus(threshold: ThresholdSummary): ThresholdStatus {
  if (threshold.source === "none") {
    return { configured: false, breached: false, label: "" };
  }
  const gate = `${formatPercent(threshold.percent)} gate`;
  return {
    configured: true,
    breached: threshold.breached,
    label: threshold.breached ? `⚠ over ${gate}` : `✓ within ${gate}`,
  };
}

/** [EXIT-CODES-KINDS] Every panel uses these same configured gate labels. */
export function metricsThresholdStatuses(metrics: RepoMetrics): ThresholdStatus[] {
  return [thresholdStatus(metrics.threshold), ...(metrics.per_kind ?? []).map((row) => {
    const status = thresholdStatus(row.threshold);
    return { ...status, label: status.configured ? `${kindTitle(row.kind)}: ${status.label}` : "" };
  })].filter((status) => status.configured);
}

/** [EXIT-CODES-KINDS] A panel's verdict includes every configured kind gate. */
export function metricsThresholdStatus(metrics: RepoMetrics): ThresholdStatus {
  const statuses = metricsThresholdStatuses(metrics);
  return { configured: statuses.length > 0, breached: statuses.some((status) => status.breached),
    label: statuses.map((status) => status.label).join(" · ") };
}
