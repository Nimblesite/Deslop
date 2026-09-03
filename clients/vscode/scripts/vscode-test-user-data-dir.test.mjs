// The regression guard for the harness's own runnability. If the profile
// path grows past the socket cap again, VS Code exits before a single
// assertion runs — and a suite that cannot start reports no failures,
// which is the most expensive kind of green.

import assert from "node:assert/strict";
import path from "node:path";
import { test } from "node:test";
import {
  UNIX_SOCKET_PATH_LIMIT,
  longestSocketPathLength,
  platformPath,
  profileRoot,
  socketPathIsCapped,
  vscodeTestUserDataDir,
} from "./vscode-test-user-data-dir.mjs";

// The path that broke it: a git worktree under the repo's own `.claude`
// directory. The default `<extension>/.vscode-test/user-data` profile
// produced a 118-byte socket path there and VS Code refused to listen.
const DEEP_CHECKOUT =
  "/Users/someone/Documents/Code/Deslop/.claude/worktrees/fused-score-followups/clients/vscode";

// Platform inputs are injected rather than read off the host so both
// behaviours are asserted on every runner. Reading `os.platform()` inside
// the test means each CI leg only ever exercises its own branch, and the
// other one ships unverified.
const POSIX = ["darwin", "linux"];
const WINDOWS_TMP =
  "C:\\Users\\a-rather-long-corporate-account-name\\AppData\\Local\\Temp\\vscode-test";

for (const platform of POSIX) {
  test(`a deep checkout still yields a socket path the kernel will accept on ${platform}`, () => {
    const dir = vscodeTestUserDataDir(DEEP_CHECKOUT, platform, "/unused");
    const length = longestSocketPathLength(dir, platform);
    assert.ok(socketPathIsCapped(platform), `${platform} is a capped platform`);
    assert.ok(
      length <= UNIX_SOCKET_PATH_LIMIT,
      `socket path is ${length} bytes, over the ${UNIX_SOCKET_PATH_LIMIT}-byte cap: ${dir}`,
    );
    assert.ok(
      length < UNIX_SOCKET_PATH_LIMIT,
      "leave headroom rather than sitting exactly on the cap",
    );
  });
}

test("the default profile location is what actually overflows", () => {
  // Pins the reason this module exists. If `@vscode/test-cli`'s default
  // ever became short enough, this test failing is the signal to delete
  // the override rather than carry it forever.
  const naive = path.posix.join(DEEP_CHECKOUT, ".vscode-test", "user-data");
  assert.ok(
    longestSocketPathLength(naive, "linux") > UNIX_SOCKET_PATH_LIMIT,
    "the default profile path used to overflow — that is the defect being fixed",
  );
});

test("Windows is not held to the Unix socket cap", () => {
  // Windows opens the same endpoint as a named pipe, which carries no
  // comparable length limit. Enforcing the POSIX cap there fails a
  // harness that runs perfectly well on any machine with a long %TEMP%.
  assert.equal(socketPathIsCapped("win32"), false);
  const dir = vscodeTestUserDataDir(DEEP_CHECKOUT, "win32", WINDOWS_TMP);
  assert.ok(
    longestSocketPathLength(dir, "win32") > UNIX_SOCKET_PATH_LIMIT,
    "this %TEMP% is exactly the long one the POSIX cap would have rejected",
  );
  assert.equal(profileRoot("win32", WINDOWS_TMP), WINDOWS_TMP);
});

test("POSIX anchors at /tmp rather than the platform temp dir", () => {
  // macOS `os.tmpdir()` is a ~50-byte per-user path that would spend half
  // the budget before the profile name is appended.
  assert.equal(profileRoot("darwin", "/var/folders/xy/T/"), "/tmp");
  assert.equal(profileRoot("linux", "/var/tmp"), "/tmp");
});

test("two checkouts never share one profile", () => {
  const worktree = vscodeTestUserDataDir(DEEP_CHECKOUT, "linux", "/unused");
  const mainline = vscodeTestUserDataDir(
    "/Users/someone/Documents/Code/Deslop/clients/vscode",
    "linux",
    "/unused",
  );
  assert.notEqual(
    worktree,
    mainline,
    "a shared profile lets two runs race each other's window state",
  );
  assert.equal(
    vscodeTestUserDataDir(DEEP_CHECKOUT, "linux", "/unused"),
    worktree,
    "the same checkout must resolve to the same profile across runs",
  );
});

test("the profile is anchored outside the checkout", () => {
  for (const platform of [...POSIX, "win32"]) {
    const dir = vscodeTestUserDataDir(DEEP_CHECKOUT, platform, WINDOWS_TMP);
    assert.ok(
      !dir.startsWith(DEEP_CHECKOUT),
      `a profile inside the checkout reintroduces the depth that caused the overflow (${platform})`,
    );
    assert.equal(
      platformPath(platform).dirname(dir),
      profileRoot(platform, WINDOWS_TMP),
      `the profile must sit directly under the ${platform} anchor`,
    );
    assert.equal(
      dir,
      platformPath(platform).normalize(dir),
      `a ${platform} profile must be spelled with ${platform} separators, whatever host resolved it`,
    );
  }
});
