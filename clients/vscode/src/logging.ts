// Single output channel for everything user-visible; stderr from the LSP lands here too.

import * as vscode from "vscode";

let channel: vscode.OutputChannel | undefined;

export function initOutputChannel(): vscode.OutputChannel {
  if (!channel) channel = vscode.window.createOutputChannel("CodeDedup");
  return channel;
}

export function log(message: string): void {
  initOutputChannel().appendLine(`[${new Date().toISOString()}] ${message}`);
}

export function logError(err: unknown, context: string): void {
  const message = err instanceof Error ? `${err.name}: ${err.message}` : String(err);
  log(`error in ${context}: ${message}`);
}
