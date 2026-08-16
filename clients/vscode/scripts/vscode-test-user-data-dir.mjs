// Where the `@vscode/test-cli` harness keeps its VS Code user profile.
//
// VS Code opens its main IPC endpoint *inside* the user-data directory,
// and on every platform with Unix domain sockets the whole path is capped
// at 103 bytes by the kernel. `@vscode/test-cli` defaults the profile to
// `<extension>/.vscode-test/user-data`, so a checkout more than a few
// directories deep — a git worktree, a CI workspace, a nested monorepo —
// pushes `…/user-data/<version>-main.sock` past the cap and VS Code exits
// before a single test runs (`Error: listen EINVAL`). The suite is not
// slow or flaky there, it is unrunnable, which is exactly the condition
// under which tests quietly stop being evidence.
//
// Anchoring the profile at a short root fixes it for any checkout depth.
// The directory is keyed by a hash of the extension path so two checkouts
// — main and a worktree, or two CI jobs on one runner — never collide in
// one profile and race each other's state.

import { createHash } from "node:crypto";
import os from "node:os";
import path from "node:path";

/** Bytes the kernel allows in a Unix domain socket path (`sun_path`). */
export const UNIX_SOCKET_PATH_LIMIT = 103;

/**
 * Longest basename VS Code appends inside the profile directory. The main
 * endpoint is `<major>.<minor>-main.sock`; the shared-process and other
 * endpoints are shorter. Sized for a three-digit major and minor so a
 * future VS Code release cannot silently eat the headroom.
 */
export const LONGEST_SOCKET_BASENAME = "999.999-main.sock";

/**
 * Resolves the user-data directory for a given extension checkout.
 *
 * @param {string} extensionDir absolute path of the extension root
 * @returns {string} absolute path to use as `--user-data-dir`
 */
export function vscodeTestUserDataDir(extensionDir) {
  const key = createHash("sha256").update(extensionDir).digest("hex").slice(0, 8);
  // `/tmp` is two levels deep and present on every POSIX target; the
  // platform temp dir on macOS is a ~50-byte per-user path that would eat
  // half the budget on its own. Windows named pipes carry no such limit,
  // so there the platform temp dir is the better-behaved choice.
  const root = os.platform() === "win32" ? os.tmpdir() : "/tmp";
  return path.join(root, `deslop-vscode-test-${key}`);
}

/**
 * Length of the longest socket path VS Code will open inside `dir`.
 *
 * @param {string} dir user-data directory
 * @returns {number} byte length of the longest resulting socket path
 */
export function longestSocketPathLength(dir) {
  return Buffer.byteLength(path.join(dir, LONGEST_SOCKET_BASENAME));
}
