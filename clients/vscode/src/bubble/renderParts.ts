// Pure text rendering for the live bubble ([VSIX-LIVE-BUBBLE]). One shared
// rebuild path (`renderBubbleParts`) feeds every surface — inline decoration,
// ghost line, inlay strip, hover card — so their strings can never drift
// apart. No editor state in here: cluster in, strings out.

import * as vscode from "vscode";

import { clusterHoverMarkdown, clusterSlug } from "../clusterHover";
import { SEVERITY_DOT } from "../design";
import { shortPath } from "../pathUtils";
import {
  ReportCluster,
  Severity,
  bucketLabels,
  occurrenceCount,
  resolveBucket,
} from "../types/report";

// The inline bubble and ghost-line decorations are pure-visual
// surfaces (rendered only in the editor, never scraped by agents), so
// they use `plainTitle` per [CLONE-BUCKETS-DUAL-LABEL].
export interface BubbleRenderParts {
  inline: string;
  ghost: string;
  signalStrip: string;
  hover: vscode.MarkdownString;
}

export function renderBubbleParts(
  cluster: ReportCluster,
  severity: Severity,
): BubbleRenderParts {
  const canonical = cluster.occurrences[0];
  const count = occurrenceCount(cluster);
  const title = bucketLabels(resolveBucket(cluster)).plainTitle;
  const slug = clusterSlug(cluster);
  const location = canonical ? ` · ${shortPath(canonical.path)}` : "";
  const strip = signalStrip(cluster);
  return {
    inline: `  ${SEVERITY_DOT[severity]} ${slug} ${title} × ${count}${location}`,
    ghost: `  └─ ${SEVERITY_DOT[severity]} ${slug} ${title}  ${strip}  × ${count}`,
    signalStrip: strip,
    hover: clusterHoverMarkdown(cluster, { showDismiss: true }),
  };
}

export function inlineText(
  cluster: ReportCluster,
  severity: Severity,
): string {
  return renderBubbleParts(cluster, severity).inline;
}

export function ghostText(
  cluster: ReportCluster,
  severity: Severity,
): string {
  return renderBubbleParts(cluster, severity).ghost;
}

// Three bars of one elected pair's evidence: shape, semantic, content
// ([VSIX-LIVE-BUBBLE], [FUSED-CLUSTER-SIGNALS]). `structural` and
// `token_jaccard` are two views of one normalised representation — "summing
// them says nothing beyond 'the shapes matched'" — so drawing both spends
// two of the three slots on a single piece of evidence. The shape bar draws
// the engine's `shape` reading — the stronger of the two shape views,
// reduced once in `deslop-core` and carried on the wire; the second bar
// draws the semantic axis; the third draws `agreement`, the measured
// content evidence. There is no combined-score bar: admission and routing
// are the engine's bucket verdict, not a number this strip re-derives.
export function signalStrip(cluster: ReportCluster): string {
  const signals = cluster.signals;
  return `${bar(signals.shape)}${bar(signals.embedding_cos)}${bar(signals.pair_agreement)}`;
}

// The full block is reserved for an exact 1.0 and nothing else. Rounding
// `value * 7` gave it to everything from 0.929 up, which collapsed the two
// readings the third bar exists to separate: a byte-proven copy renders
// `agreement 1.00` and a near-verbatim clone with one drift pair reads
// `agreement 0.96`, and both drew `█`. Proof and near-proof are exactly
// the distinction a glance at this strip is supposed to make, so the top
// glyph means proof.
function bar(value: number): string {
  if (value >= 1) return BARS[BARS.length - 1] ?? "█";
  const below = BARS.length - 1;
  const index = Math.min(
    below - 1,
    Math.max(0, Math.round(value * (below - 1))),
  );
  return BARS[index] ?? "▁";
}

const BARS = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"] as const;

// Bubble hover: full card with slug, canonical, and dismiss link.
export function bubbleHover(
  cluster: ReportCluster,
): vscode.MarkdownString {
  return renderBubbleParts(cluster, "faint").hover;
}
