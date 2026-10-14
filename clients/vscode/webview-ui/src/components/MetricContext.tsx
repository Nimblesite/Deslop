// [METRICS-TOTAL-INCLUDE] [EXIT-CODES-KINDS] Every metrics panel prints the same server verdicts.
import { totalCountsCaption, type RepoMetrics } from "../../../src/types/report";
import { metricsThresholdStatuses } from "../../../src/tree/threshold";

export function MetricContext({ metrics }: { metrics: RepoMetrics }) {
  const caption = totalCountsCaption(metrics.total_includes);
  return <div>
    {caption ? <div class="total-counts-caption">total counts: {caption}</div> : null}
    {metricsThresholdStatuses(metrics).map((status) => <p key={status.label} class={status.breached ? "threshold-verdict--breached" : "threshold-verdict"}>{status.label}</p>)}
  </div>;
}
