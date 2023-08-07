## How to generate the Wasm bindings

From the root of the repo, run:

```bash
cargo build -p query-engine-wasm-api --target="wasm32-unknown-unknown" --release
```

To optimize the Wasm binary, run:

```bash
# Optimization flags:
# -Os: Optimize for size
# --strip-debug: Remove debugging information
# --strip-producers: Strip producer section
# --strip-dwarf: Strip DWARF debug information
# --disable-threads: Disable threads support
# --code-folding: Enable code folding optimization
# --minify-imports: Minify import names
# --remove-non-js-ops: Remove non-JS operations
# --duplicate-function-elimination: Eliminate duplicate functions
WASM_FILE="./target/wasm32-unknown-unknown/release/libquery_wasm.wasm" \
  wasm-opt $WASM_FILE \
  -Os \
  --strip-debug \
  --strip-producers \
  --strip-dwarf \
  --disable-threads \
  --code-folding \
  --minify-imports \
  --remove-non-js-ops \
  --duplicate-function-elimination \
  -o $WASM_FILE
```

The resulting Wasm binary should be around 4.1MB.

Then, run

```bash
wasm-bindgen --target bundler --out-dir ./query-engine/js-connectors/js/smoke-test-js/src/wasm ./target/wasm32-unknown-unknown/release/libquery_wasm.wasm
```

TODO: evaluate using `wasm-pack`.
