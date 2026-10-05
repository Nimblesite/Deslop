// [VSIX-LIVE-BUBBLE] The hoverable detail cue beside an edited clone.
// It carries the shared cluster card, never a second calculation or pair score.

import * as vscode from "vscode";

import { clusterHoverMarkdown } from "../clusterHover";
import type { ReportCluster } from "../types/report";

const DETAIL_LABEL = "ⓘ Details";

export class BubbleInlayProvider implements vscode.InlayHintsProvider, vscode.Disposable {
  private readonly changed = new vscode.EventEmitter<void>();
  readonly onDidChangeInlayHints = this.changed.event;
  private current: { uri: vscode.Uri; range: vscode.Range; cluster: ReportCluster } | null = null;

  set(uri: vscode.Uri, range: vscode.Range, cluster: ReportCluster): void {
    this.current = { uri, range, cluster };
    this.changed.fire();
  }

  clear(): void {
    if (!this.current) return;
    this.current = null;
    this.changed.fire();
  }

  provideInlayHints(document: vscode.TextDocument, range: vscode.Range): vscode.InlayHint[] {
    const active = this.current;
    if (!active) return [];
    if (document.uri.toString() !== active.uri.toString()) return [];
    if (!range.contains(active.range.end)) return [];
    const hint = new vscode.InlayHint(active.range.end, DETAIL_LABEL, vscode.InlayHintKind.Type);
    hint.paddingLeft = true;
    hint.tooltip = clusterHoverMarkdown(active.cluster, { showDismiss: true });
    return [hint];
  }

  dispose(): void {
    this.clear();
    this.changed.dispose();
  }
}
