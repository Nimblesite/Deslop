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
