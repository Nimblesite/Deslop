// Pure text rendering for the live bubble ([VSIX-LIVE-BUBBLE]). One shared
// rebuild path (`renderBubbleParts`) feeds every surface — inline decoration,
// ghost line, hover card — so their strings can never drift apart. No editor
// state in here: cluster in, strings out.
//
// The admission signals (structural, token, embedding, content similarity)
// are pair measurements and never touch the cluster ([FUSED-PAIR-SIGNALS]).
// The live bubble renders the cluster's duplicated-mass facts only; an
// explicit pair comparison is the sole surface allowed to quote a
// pair's values.

import * as vscode from "vscode";

import { clusterHoverMarkdown, clusterSlug } from "../clusterHover";
import { SEVERITY_DOT } from "../design";
import { shortPath } from "../pathUtils";
import {
  ReportCluster,
  Severity,
  occurrenceCount,
} from "../types/report";

// The inline bubble and ghost-line decorations are pure-visual
// surfaces (rendered only in the editor, never scraped by agents); the
// short verdict is the spec'd `DUPLICATION` label ([VSIX-LIVE-BUBBLE]).
const SHORT_VERDICT = "DUPLICATION";

export interface BubbleRenderParts {
  inline: string;
  ghost: string;
  hover: vscode.MarkdownString;
}

export function renderBubbleParts(
  cluster: ReportCluster,
  severity: Severity,
): BubbleRenderParts {
  const canonical = cluster.occurrences[0];
  const count = occurrenceCount(cluster);
  const title = SHORT_VERDICT;
  const slug = clusterSlug(cluster);
  const location = canonical ? ` · ${shortPath(canonical.path)}` : "";
  return {
    inline: `  ${SEVERITY_DOT[severity]} ${slug} ${title} × ${count}${location}`,
    ghost: `  └─ ${SEVERITY_DOT[severity]} ${slug} ${title} × ${count}`,
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

// Bubble hover: full card with slug, canonical, and dismiss link.
export function bubbleHover(
  cluster: ReportCluster,
): vscode.MarkdownString {
  return renderBubbleParts(cluster, "faint").hover;
}
