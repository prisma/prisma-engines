# @prisma/analyse-query-engine-wasm

> Internal utility tool to analyse the size impact of Rust crates on the Wasm binary.

## Scripts

- `pnpm i`: Install dependencies.
- `pnpm build`: Build the Wasm binary in `"profiling"` mode.
- `pnpm prepare:crates`: Invoke `twiggy top` on the Wasm binary, saving the results in `./twiggy.profiling.json`.
- `pnpm crates`: Filter and analyse the `twiggy top` results, printing a summary Markdown table of the size impact of each Rust crate involved. Wasm sections, like `data` (bunch of static strings) also appear in the table, and are marked with a `🧩`.
