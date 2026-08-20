// [VSIX-METRICS-PANEL] Shared duplication-gate status formatting.
// The CI gate (`.deslop.toml [threshold] max_duplication_percent`,
// [EXIT-CODES]) is informational on the live surfaces — it never gates,
// hides, or re-ranks. Both the sidebar Duplication tree and the report
// webview render the gate through this single formatter so the two
// panels stay byte-identical ([UI] "do not duplicate the rendering").

import { formatPercent } from "../types/format";
import type { ThresholdSummary } from "../types/report";

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
