import { context } from "esbuild"

const ctx = await context({
  entryPoints: ["src/extension.ts"],
  outfile: "dist/extension.js",
  bundle: true,
  format: "cjs",

  platform: "node",
  target: "node22",
  external: ["vscode"],


  minify: false,
  sourcemap: true,
  tsconfig: "tsconfig.json",
})

await ctx.rebuild()

if (process.argv.includes("--watch")) {
  console.log("esbuild: watching...")
  await ctx.watch()
} else {
  await ctx.dispose()
}
