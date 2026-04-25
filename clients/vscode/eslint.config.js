// Flat-config ESLint for the VSIX extension.
//
// Rules locked to `error` on every category that can produce a "silent
// bug" in async code:
//   - no-floating-promises:  unawaited Promise = compile error (the
//     exact bug that let a test "pass" while VS Code's DialogService
//     rejection escaped mocha across suite boundaries).
//   - no-misused-promises:   Promise passed where void expected, or
//     an async fn used as an event handler without an await.
//   - await-thenable:        awaiting something that isn't thenable
//     (usually means a missing `.ok()` or a mistyped return).
//   - require-await:         async fn with no await is either
//     mislabeled or missing the real async call.
//
// These rules are TYPE-AWARE — they need the TS type graph. Hence
// `parserOptions.projectService: true`, which hands the parser the
// tsconfig.json in this directory.

const tseslint = require("typescript-eslint");

function isFunctionNode(node) {
  return [
    "ArrowFunctionExpression",
    "FunctionDeclaration",
    "FunctionExpression",
  ].includes(node?.type);
}

function propertyName(member) {
  const property = member.property;
  if (!property) return null;
  if (property.type === "Identifier") return property.name;
  if (property.type === "Literal") return String(property.value);
  return null;
}

function isQuickPickFactoryCall(node) {
  return (
    node?.type === "CallExpression" &&
    node.callee?.type === "MemberExpression" &&
    propertyName(node.callee) === "createQuickPick"
  );
}

function collectQuickPickLifecycle(node, root, out) {
  if (!node || typeof node.type !== "string") return;
  if (node !== root && isFunctionNode(node)) return;
  if (
    node.type === "VariableDeclarator" &&
    node.id?.type === "Identifier" &&
    isQuickPickFactoryCall(node.init)
  ) {
    out.creations.set(node.id.name, node);
  }
  if (node.type === "AwaitExpression") out.awaits.push(node);
  if (
    node.type === "CallExpression" &&
    node.callee?.type === "MemberExpression" &&
    node.callee.object?.type === "Identifier" &&
    propertyName(node.callee) === "onDidHide"
  ) {
    out.hideCalls.set(node.callee.object.name, node);
  }
  for (const [key, value] of Object.entries(node)) {
    if (key === "parent") continue;
    if (Array.isArray(value)) {
      for (const child of value) collectQuickPickLifecycle(child, root, out);
    } else if (value && typeof value.type === "string") {
      collectQuickPickLifecycle(value, root, out);
    }
  }
}

const quickPickLifecyclePlugin = {
  rules: {
    "quick-pick-hide-before-await": {
      meta: {
        type: "problem",
        docs: {
          description: "Require QuickPick hide/dispose handlers before the first await.",
        },
        messages: {
          handlerAfterAwait:
            "Register {{name}}.onDidHide before the first await after createQuickPick, so loading pickers can always close/dispose.",
        },
        schema: [],
      },
      create(context) {
        return {
          ":function"(node) {
            if (!node.body || node.body.type !== "BlockStatement") return;
            const out = {
              creations: new Map(),
              awaits: [],
              hideCalls: new Map(),
            };
            collectQuickPickLifecycle(node.body, node.body, out);
            for (const [name, creation] of out.creations) {
              const creationStart = creation.range?.[0] ?? Number.MAX_SAFE_INTEGER;
              const firstAwait = out.awaits.find(
                (awaitNode) => (awaitNode.range?.[0] ?? 0) > creationStart,
              );
              if (!firstAwait) continue;
              const hideCall = out.hideCalls.get(name);
              const hideStart = hideCall?.range?.[0] ?? Number.MAX_SAFE_INTEGER;
              if (hideStart > (firstAwait.range?.[0] ?? 0)) {
                context.report({
                  node: firstAwait,
                  messageId: "handlerAfterAwait",
                  data: { name },
                });
              }
            }
          },
        };
      },
    },
  },
};

module.exports = tseslint.config(
  {
    // Lint only source + tests. node_modules, out, dist, coverage,
    // .vscode-test/ and webview-ui get their own tooling.
    ignores: [
      "node_modules/**",
      "out/**",
      "dist/**",
      "coverage/**",
      ".vscode-test/**",
      "bin/**",
      "media/webview/**",
      "webview-ui/**",
      "*.vsix",
      "eslint.config.js",
      "esbuild.mjs",
      ".vscode-test.mjs",
      ".vscode-test-ollama.mjs",
      "scripts/**",
    ],
  },
  ...tseslint.configs.recommendedTypeChecked,
  {
    plugins: {
      "deslop-local": quickPickLifecyclePlugin,
    },
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: __dirname,
      },
    },
    rules: {
      // === Tier 1: async bug catchers (the reason this config exists) ===
      "@typescript-eslint/no-floating-promises": [
        "error",
        { ignoreVoid: false, ignoreIIFE: false },
      ],
      "@typescript-eslint/no-misused-promises": [
        "error",
        {
          checksConditionals: true,
          checksVoidReturn: true,
          checksSpreads: true,
        },
      ],
      "@typescript-eslint/await-thenable": "error",
      "@typescript-eslint/require-await": "error",
      "@typescript-eslint/return-await": ["error", "always"],
      "deslop-local/quick-pick-hide-before-await": "error",

      // === Tier 2: type-safety escape hatches (all elevated from warn) ===
      "@typescript-eslint/no-unsafe-argument": "error",
      "@typescript-eslint/no-unsafe-assignment": "error",
      "@typescript-eslint/no-unsafe-call": "error",
      "@typescript-eslint/no-unsafe-member-access": "error",
      "@typescript-eslint/no-unsafe-return": "error",

      // === Tier 3: the 10 most critical upgraded to error ===
      //  1. No explicit `any` — the foundation of type safety.
      "@typescript-eslint/no-explicit-any": "error",
      //  2. No non-null assertions (`x!`) — silently coerces
      //     null/undefined into type-checker-invisible bugs.
      "@typescript-eslint/no-non-null-assertion": "error",
      //  3. Use `??` when the intent is "null or undefined", not `||`
      //     (which also fires on 0/''/false).
      "@typescript-eslint/prefer-nullish-coalescing": "error",
      //  4. Optional chaining over manual `&&` ladders — equivalent
      //     output, fewer edge cases (e.g. `x && x.y` vs `x?.y`).
      "@typescript-eslint/prefer-optional-chain": "error",
      //  5. Stop stringifying objects via template literals or `+`
      //     — `[object Object]` is a bug, not output.
      "@typescript-eslint/restrict-template-expressions": [
        "error",
        { allowNumber: true, allowBoolean: true, allowNullish: false },
      ],
      //  6. `switch` on a union must handle every branch, or
      //     explicitly default. Catches silently dropped cases when
      //     unions grow.
      "@typescript-eslint/switch-exhaustiveness-check": "error",
      //  7. Unbound methods lose `this` — classic JS footgun when
      //     passing a method as a callback.
      "@typescript-eslint/unbound-method": "error",
      //  8. `for (const x of arr)` etc. over for-in for arrays,
      //     eliminates prototype-pollution surface area.
      "@typescript-eslint/prefer-for-of": "error",
      //  9. No shadowing — same name in inner + outer scope is the
      //     source of hundreds of subtle bugs per year.
      "@typescript-eslint/no-shadow": "error",
      // 10. `.forEach(async ...)` and equivalent patterns that
      //     discard returned Promises — a specialized floating-promise
      //     catch the general rule misses.
      "no-promise-executor-return": "error",

      // Unused identifiers (allow underscore-prefixed).
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
    },
  },
);
