import * as path from "node:path";
import * as vscode from "vscode";

export function resolveWorkspacePath(filePath: string): string {
  if (path.isAbsolute(filePath)) return filePath;
  const root = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  return root ? path.join(root, filePath) : filePath;
}

export function sameFile(left: string, right: string): boolean {
  return left === right || left.endsWith(right) || right.endsWith(left);
}

export function shortPath(filePath: string): string {
  const slash = Math.max(filePath.lastIndexOf("/"), filePath.lastIndexOf("\\"));
  return slash >= 0 ? filePath.slice(slash + 1) : filePath;
}
