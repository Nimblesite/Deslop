import esbuild from "esbuild";

const watch = process.argv.includes("--watch");
const coverage = process.argv.includes("--coverage");

const ctx = await esbuild.context({
  entryPoints: ["src/extension.ts"],
  bundle: true,
  outfile: "dist/extension.js",
  platform: "node",
  target: "node20",
  format: "cjs",
  sourcemap: true,
  external: ["vscode"],
  logLevel: "info",
  minify: !watch && !coverage,
});

if (watch) {
  await ctx.watch();
} else {
  await ctx.rebuild();
  await ctx.dispose();
}
