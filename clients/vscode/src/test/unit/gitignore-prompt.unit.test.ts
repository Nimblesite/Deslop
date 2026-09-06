// Unit: [VSIX-CACHE-IGNORE] (#286). Deslop's cache became 95% of an F#
// user's repo by file count because nothing ignored it. The fix must add
// the entry — and, just as importantly, must never write to a user's
// tracked `.gitignore` without an explicit Yes.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";
import {
  CACHE_IGNORE_ENTRY,
  DECLINED_KEY,
  isCacheIgnored,
  needsCacheIgnore,
  promptToIgnoreCache,
  withCacheIgnored,
  writeCacheIgnore,
} from "../../gitignorePrompt";

/// Creates a throwaway directory that looks like a git repository, with
/// optional `.gitignore` contents.
function repo(gitignore?: string): string {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "deslop-ignore-"));
  fs.mkdirSync(path.join(root, ".git"));
  if (gitignore !== undefined) {
    fs.writeFileSync(path.join(root, ".gitignore"), gitignore, "utf8");
  }
  return root;
}

/// Reads a workspace's `.gitignore`, or undefined when absent.
function readIgnore(root: string): string | undefined {
  const file = path.join(root, ".gitignore");
  return fs.existsSync(file) ? fs.readFileSync(file, "utf8") : undefined;
}

/// A context whose workspaceState is a real in-memory map, so a recorded
/// "No" is observable.
function fakeContext(seed: Record<string, unknown> = {}): vscode.ExtensionContext {
  const store = new Map<string, unknown>(Object.entries(seed));
  return {
    workspaceState: {
      get: (key: string) => store.get(key),
      update: (key: string, value: unknown) => {
        store.set(key, value);
        return Promise.resolve();
      },
    },
  } as unknown as vscode.ExtensionContext;
}

/// Replaces `showInformationMessage` with one that returns `answer` and
/// records the prompts it was given. Restores on dispose.
function stubPrompt(answer: string | undefined): {
  prompts: string[];
  restore: () => void;
} {
  const prompts: string[] = [];
  const original = vscode.window.showInformationMessage;
  const replacement = (message: string): Thenable<string | undefined> => {
    prompts.push(message);
    return Promise.resolve(answer);
  };
  (vscode.window as { showInformationMessage: unknown }).showInformationMessage =
    replacement;
  return {
    prompts,
    restore: () => {
      (vscode.window as { showInformationMessage: unknown }).showInformationMessage =
        original;
    },
  };
}

suite("cache gitignore consent", () => {
  test("isCacheIgnored matches both spellings and ignores comments", () => {
    assert.equal(isCacheIgnored(".deslop/\n"), true);
    assert.equal(isCacheIgnored("node_modules\n.deslop\ntarget\n"), true);
    assert.equal(isCacheIgnored("  .deslop/  \n"), true);
    assert.equal(isCacheIgnored("# .deslop/\n"), false);
    assert.equal(isCacheIgnored("node_modules\ntarget\n"), false);
    assert.equal(isCacheIgnored(""), false);
    assert.equal(
      isCacheIgnored(".deslop-cache/\n"),
      false,
      "the pre-[OUTPUT-DIR] entry no longer covers what Deslop writes",
    );
    assert.equal(
      isCacheIgnored(".deslop.toml\n"),
      false,
      "ignoring the config file is not ignoring the output directory",
    );
  });

  test("withCacheIgnored preserves content and newline hygiene", () => {
    assert.equal(withCacheIgnored(""), `${CACHE_IGNORE_ENTRY}\n`);
    assert.equal(withCacheIgnored("target\n"), `target\n${CACHE_IGNORE_ENTRY}\n`);
    assert.equal(
      withCacheIgnored("target"),
      `target\n${CACHE_IGNORE_ENTRY}\n`,
      "a file with no trailing newline must not glue onto the entry",
    );
  });

  test("needsCacheIgnore is false outside a git repository", () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "deslop-nogit-"));
    assert.equal(
      needsCacheIgnore(root),
      false,
      "with no .git there is nothing to pollute, so nothing to ask",
    );
  });

  test("needsCacheIgnore tracks the .gitignore contents", () => {
    assert.equal(needsCacheIgnore(repo()), true, "no .gitignore at all");
    assert.equal(needsCacheIgnore(repo("target\n")), true, "unrelated entries");
    assert.equal(needsCacheIgnore(repo(".deslop/\n")), false, "already ignored");
    assert.equal(
      needsCacheIgnore(repo(".deslop-cache/\n")),
      true,
      "a repo carrying only the old cache entry still needs .deslop/ ignored",
    );
  });

  test("needsCacheIgnore sees the repository from a workspace subfolder", () => {
    const root = repo("target\n");
    const nested = path.join(root, "packages", "app");
    fs.mkdirSync(nested, { recursive: true });
    assert.equal(
      needsCacheIgnore(nested),
      true,
      "opening a monorepo package directly is still inside the repo, so the cache still pollutes it",
    );
  });

  test("declining leaves the user's .gitignore byte-for-byte untouched", async () => {
    const original = "# my rules\nnode_modules\n";
    const root = repo(original);
    const context = fakeContext();
    const prompt = stubPrompt("No");
    try {
      const written = await promptToIgnoreCache(context, root);
      assert.equal(written, false, "No must not write");
      assert.equal(
        readIgnore(root),
        original,
        "a declined prompt must not modify tracked source",
      );
      assert.deepEqual(prompt.prompts, ["Ignore deslop files from git?"]);
      assert.equal(
        context.workspaceState.get(DECLINED_KEY),
        true,
        "the No must be remembered",
      );
    } finally {
      prompt.restore();
    }
  });

  test("dismissing the prompt is also treated as a No", async () => {
    const root = repo("target\n");
    const context = fakeContext();
    const prompt = stubPrompt(undefined);
    try {
      assert.equal(await promptToIgnoreCache(context, root), false);
      assert.equal(readIgnore(root), "target\n", "dismissal must not write");
      assert.equal(context.workspaceState.get(DECLINED_KEY), true);
    } finally {
      prompt.restore();
    }
  });

  test("a remembered No is never re-prompted", async () => {
    const root = repo("target\n");
    const context = fakeContext({ [DECLINED_KEY]: true });
    const prompt = stubPrompt("Yes");
    try {
      assert.equal(await promptToIgnoreCache(context, root), false);
      assert.deepEqual(prompt.prompts, [], "must not ask a second time");
      assert.equal(readIgnore(root), "target\n");
    } finally {
      prompt.restore();
    }
  });

  test("accepting appends the entry and keeps existing rules", async () => {
    const root = repo("# my rules\nnode_modules\n");
    const context = fakeContext();
    const prompt = stubPrompt("Yes");
    try {
      assert.equal(await promptToIgnoreCache(context, root), true);
      assert.equal(
        readIgnore(root),
        `# my rules\nnode_modules\n${CACHE_IGNORE_ENTRY}\n`,
        "existing rules must survive",
      );
      assert.equal(needsCacheIgnore(root), false, "the cache is now ignored");
    } finally {
      prompt.restore();
    }
  });

  test("accepting creates .gitignore when the repo has none", async () => {
    const root = repo();
    const context = fakeContext();
    const prompt = stubPrompt("Yes");
    try {
      assert.equal(await promptToIgnoreCache(context, root), true);
      assert.equal(readIgnore(root), `${CACHE_IGNORE_ENTRY}\n`);
    } finally {
      prompt.restore();
    }
  });

  test("an already-ignored cache never prompts", async () => {
    const root = repo(`${CACHE_IGNORE_ENTRY}\n`);
    const context = fakeContext();
    const prompt = stubPrompt("Yes");
    try {
      assert.equal(await promptToIgnoreCache(context, root), false);
      assert.deepEqual(prompt.prompts, [], "nothing to ask when already ignored");
    } finally {
      prompt.restore();
    }
  });

  test("no workspace means no prompt and no write", async () => {
    const context = fakeContext();
    const prompt = stubPrompt("Yes");
    try {
      assert.equal(await promptToIgnoreCache(context, undefined), false);
      assert.deepEqual(prompt.prompts, []);
    } finally {
      prompt.restore();
    }
  });

  test("writeCacheIgnore reports failure instead of throwing", () => {
    // The directory must not exist, but a fixed name under the shared OS
    // temp dir is the wrong way to get that: it is CWE-377 (any other user
    // can pre-create it and capture the write) and it makes the test fail
    // outright on a machine where that path is already lying around. Take a
    // 0700 `mkdtemp` parent and name a child inside it that was never made.
    const parent = fs.mkdtempSync(path.join(os.tmpdir(), "deslop-missing-"));
    assert.equal(
      writeCacheIgnore(path.join(parent, "never-created")),
      false,
      "an unwritable workspace must degrade, not crash activation",
    );
  });
});
