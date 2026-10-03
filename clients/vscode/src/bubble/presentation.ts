// [VSIX-LIVE-BUBBLE] One setting selects the shared bubble presentation.

import * as vscode from "vscode";

export type BubblePresentation = "classic" | "compact";

export const DEFAULT_BUBBLE_PRESENTATION: BubblePresentation = "classic";

export function bubblePresentation(): BubblePresentation {
  const configured = vscode.workspace.getConfiguration("deslop")
    .get<string>("liveBubble.presentation", DEFAULT_BUBBLE_PRESENTATION);
  return configured === "compact" ? "compact" : DEFAULT_BUBBLE_PRESENTATION;
}
