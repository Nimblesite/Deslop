// [VSIX-CACHE-IGNORE] Deslop writes everything it produces for a
// workspace into `<workspace>/.deslop/` ([OUTPUT-DIR]) — reports, logs,
// fingerprints, one blob per embedded subtree, the live report, and the
// IPC endpoint records. On a large repo that is hundreds of thousands of
// files. An F# user reported it reaching 700 MB and becoming "95% of the
// files in the repo by count" because nothing ever told git to ignore it
// (#286). Deslop's own `.gitignore` has carried the entry since day one,
// which is exactly why nobody here ever felt it.
//
// The output directory is ours to write. The user's `.gitignore` is not:
// it is tracked source that lands in their next commit and changes what
// their whole team sees. So this asks, once, and writes only on an
// explicit Yes — never silently, and never again after a No.

import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import { log } from "./logging";

/// The entry written into `.gitignore` on consent. Trailing slash so it
/// matches the output directory only — never the user's `.deslop.toml`
/// config file — exactly as Deslop's own repo spells it.
export const CACHE_IGNORE_ENTRY = ".deslop/";

/// Prompt text and its two answers. `No` is a real button rather than a
/// dismissal so declining is recorded rather than re-asked next session.
export const IGNORE_PROMPT = "Ignore deslop files from git?";
const YES = "Yes";
const NO = "No";

/// Workspace-state key remembering a `No`. Workspace-scoped, not global:
/// the answer is about this repository's `.gitignore`.
export const DECLINED_KEY = "deslop.cacheIgnoreDeclined";

/// True when `gitignoreText` already ignores the output directory, with
/// or without the trailing slash. Comment lines never count as a match,
/// so a commented-out entry still prompts. A pre-[OUTPUT-DIR]
/// `.deslop-cache/` entry does not count: that directory is no longer
/// written, and the workspace still needs `.deslop/` covered.
export function isCacheIgnored(gitignoreText: string): boolean {
  return gitignoreText
    .split("\n")
    .map((line) => line.trim())
    .some((line) => line === ".deslop/" || line === ".deslop");
}

/// `existing` with the cache entry appended, preserving whatever was
/// already there and never producing a missing or doubled newline.
export function withCacheIgnored(existing: string): string {
  const needsBreak = existing.length > 0 && !existing.endsWith("\n");
  return `${existing}${needsBreak ? "\n" : ""}${CACHE_IGNORE_ENTRY}\n`;
}

/// Reads `.gitignore` at `workspaceRoot`, returning "" when absent.
function readGitignore(gitignorePath: string): string {
  try {
    return fs.readFileSync(gitignorePath, "utf8");
  } catch {
    return "";
  }
}

/// True when `directory` sits inside a git working tree. Walks to the
/// filesystem root because a VS Code workspace is often a subfolder of
/// the repository (monorepo package, `src/` opened directly) — checking
/// only the workspace root would silently never ask those users. `.git`
/// is a file, not a directory, in worktrees and submodules, so this
/// tests existence rather than directory-ness.
function insideGitRepository(directory: string): boolean {
  let current = path.resolve(directory);
  for (;;) {
    if (fs.existsSync(path.join(current, ".git"))) return true;
    const parent = path.dirname(current);
    if (parent === current) return false;
    current = parent;
  }
}

/// True when the cache still needs ignoring in `workspaceRoot` — i.e. it
/// is inside a git working tree and the `.gitignore` sitting beside the
/// cache does not already cover it. Outside a repository there is
/// nothing to pollute, so nothing to ask.
export function needsCacheIgnore(workspaceRoot: string): boolean {
  if (!insideGitRepository(workspaceRoot)) return false;
  return !isCacheIgnored(readGitignore(path.join(workspaceRoot, ".gitignore")));
}

/// Appends the cache entry to `workspaceRoot`'s `.gitignore`, creating
/// the file when absent. Returns false when the write fails.
export function writeCacheIgnore(workspaceRoot: string): boolean {
  const gitignorePath = path.join(workspaceRoot, ".gitignore");
  try {
    fs.writeFileSync(gitignorePath, withCacheIgnored(readGitignore(gitignorePath)), "utf8");
    return true;
  } catch (error) {
    log("cache ignore write failed", { message: String(error) });
    return false;
  }
}

/// Asks once whether to ignore Deslop's cache, and writes `.gitignore`
/// only on an explicit Yes. Returns true when the entry was written.
///
/// Silent no-op when there is no workspace, no git repository, the entry
/// is already present, or the user previously said No.
export async function promptToIgnoreCache(
  context: vscode.ExtensionContext,
  workspaceRoot: string | undefined,
): Promise<boolean> {
  if (!workspaceRoot) return false;
  if (context.workspaceState.get<boolean>(DECLINED_KEY) === true) return false;
  if (!needsCacheIgnore(workspaceRoot)) return false;

  const answer = await vscode.window.showInformationMessage(IGNORE_PROMPT, YES, NO);
  if (answer !== YES) {
    // Both No and a dismissal are recorded: re-prompting every activation
    // is how a helpful question becomes nagging.
    await context.workspaceState.update(DECLINED_KEY, true);
    log("cache ignore declined", { answered: answer === NO });
    return false;
  }
  const written = writeCacheIgnore(workspaceRoot);
  log("cache ignore accepted", { written });
  return written;
}
