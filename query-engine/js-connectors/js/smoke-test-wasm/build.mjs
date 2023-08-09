import * as esbuild from 'esbuild'

await esbuild.build({
  entryPoints: ['src/neon.ts'],
  bundle: true,
  outfile: 'dist/neon.js',
  platform: 'node',
  loader: {
    ".prisma": "text",
    // ".wasm": "copy", // enable this option in Vercel Edge Functions
    ".wasm": "binary", // enable this option in NodeJS builds
  },
})
