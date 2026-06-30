import esbuild from "esbuild";

const watch = process.argv.includes("--watch");
// Coverage mode emits unminified bundles with an inline sourcemap so the
// Playwright V8 coverage pass can map executed ranges back to webview-ui/src
// (minified single-line output collapses every statement onto one line).
const coverage = process.argv.includes("--coverage");

const ctx = await esbuild.context({
  entryPoints: {
    cluster: "src/cluster/main.tsx",
    report: "src/report/main.tsx",
    duplication: "src/duplication/main.tsx",
  },
  bundle: true,
  outdir: "../media/webview",
  platform: "browser",
  target: "es2022",
  format: "esm",
  sourcemap: coverage ? "inline" : true,
  minify: !watch && !coverage,
  jsx: "automatic",
  jsxImportSource: "preact",
  loader: { ".css": "text" },
  logLevel: "info",
});

if (watch) {
  await ctx.watch();
} else {
  await ctx.rebuild();
  await ctx.dispose();
}
