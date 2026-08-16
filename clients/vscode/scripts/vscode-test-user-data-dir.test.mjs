// The regression guard for the harness's own runnability. If the profile
// path grows past the socket cap again, VS Code exits before a single
// assertion runs — and a suite that cannot start reports no failures,
// which is the most expensive kind of green.

import assert from "node:assert/strict";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";
import {
  UNIX_SOCKET_PATH_LIMIT,
  longestSocketPathLength,
  vscodeTestUserDataDir,
} from "./vscode-test-user-data-dir.mjs";

// The path that broke it: a git worktree under the repo's own `.claude`
// directory. The default `<extension>/.vscode-test/user-data` profile
// produced a 118-byte socket path there and VS Code refused to listen.
const DEEP_CHECKOUT =
  "/Users/someone/Documents/Code/Deslop/.claude/worktrees/fused-score-followups/clients/vscode";

test("a deep checkout still yields a socket path the kernel will accept", () => {
  const dir = vscodeTestUserDataDir(DEEP_CHECKOUT);
  const length = longestSocketPathLength(dir);
  assert.ok(
    length <= UNIX_SOCKET_PATH_LIMIT,
    `socket path is ${length} bytes, over the ${UNIX_SOCKET_PATH_LIMIT}-byte cap: ${dir}`,
  );
  assert.ok(
    length < UNIX_SOCKET_PATH_LIMIT,
    "leave headroom rather than sitting exactly on the cap",
  );
});

test("the default profile location is what actually overflows", () => {
  // Pins the reason this module exists. If `@vscode/test-cli`'s default
  // ever became short enough, this test failing is the signal to delete
  // the override rather than carry it forever.
  const naive = path.join(DEEP_CHECKOUT, ".vscode-test", "user-data");
  assert.ok(
    longestSocketPathLength(naive) > UNIX_SOCKET_PATH_LIMIT,
    "the default profile path used to overflow — that is the defect being fixed",
  );
});

test("two checkouts never share one profile", () => {
  const worktree = vscodeTestUserDataDir(DEEP_CHECKOUT);
  const mainline = vscodeTestUserDataDir("/Users/someone/Documents/Code/Deslop/clients/vscode");
  assert.notEqual(
    worktree,
    mainline,
    "a shared profile lets two runs race each other's window state",
  );
  assert.equal(
    vscodeTestUserDataDir(DEEP_CHECKOUT),
    worktree,
    "the same checkout must resolve to the same profile across runs",
  );
});

test("the profile is anchored outside the checkout", () => {
  const dir = vscodeTestUserDataDir(DEEP_CHECKOUT);
  assert.ok(
    !dir.startsWith(DEEP_CHECKOUT),
    "a profile inside the checkout reintroduces the depth that caused the overflow",
  );
  const expectedRoot = os.platform() === "win32" ? os.tmpdir() : "/tmp";
  assert.equal(path.dirname(dir), expectedRoot);
});
