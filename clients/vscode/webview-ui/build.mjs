import esbuild from "esbuild";

const watch = process.argv.includes("--watch");

const ctx = await esbuild.context({
  entryPoints: {
    cluster: "src/cluster/main.tsx",
    report: "src/report/main.tsx",
  },
  bundle: true,
  outdir: "../media/webview",
  platform: "browser",
  target: "es2022",
  format: "esm",
  sourcemap: true,
  minify: !watch,
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
