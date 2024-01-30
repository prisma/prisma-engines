#!/bin/bash 
# Call this script as `./build.sh <npm_version>`
set -euo pipefail

OUT_VERSION="${1:-}"
OUT_TARGET="bundler"
OUT_NPM_NAME="@prisma/query-engine-wasm"
CURRENT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
OUT_FOLDER="${OUT_FOLDER:-$CURRENT_DIR/pkg}"
OUT_JSON="${OUT_FOLDER}/package.json"

if [[ -z "${WASM_BUILD_PROFILE:-}" ]]; then
    # use `wasm-pack build --release` by default on CI only
    if [[ -z "${BUILDKITE:-}" ]] && [[ -z "${GITHUB_ACTIONS:-}" ]]; then
        WASM_BUILD_PROFILE="dev"
    else
        WASM_BUILD_PROFILE="release"
    fi
fi

echo "Using build profile: \"${WASM_BUILD_PROFILE}\"" 

if ! command -v wasm-pack &> /dev/null
then
    echo "wasm-pack could not be found, installing now..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

echo "ℹ️  Configuring rust toolchain to use nightly and rust-src component"
rustup default nightly-2024-01-25 
rustup target add wasm32-unknown-unknown
rustup component add rust-src rust-std --target wasm32-unknown-unknown

# export RUSTFLAGS="-Zlocation-detail=none"
echo "Building query-engine-wasm using $WASM_BUILD_PROFILE profile"
CARGO_PROFILE_RELEASE_OPT_LEVEL="z" wasm-pack build "--$WASM_BUILD_PROFILE" --target "$OUT_TARGET" --out-name query_engine  . \
-Zbuild-std=std,panic_abort -Zbuild-std-features=panic_immediate_abort

# wasm-opt pass
WASM_OPT_ARGS=(
    "-Os"                                 # execute size-focused optimization passes (-Oz actually increases size by 1KB)
    "--vacuum"                            # removes obviously unneeded code
    "--duplicate-function-elimination"    # removes duplicate functions 
    "--duplicate-import-elimination"      # removes duplicate imports
    "--remove-unused-module-elements"     # removes unused module elements
    "--dae-optimizing"                    # removes arguments to calls in an lto-like manner
    "--remove-unused-names"               # removes names from location that are never branched to
    "--rse"                               # removes redundant local.sets
    "--gsi"                               # global struct inference, to optimize constant values    
    "--strip-dwarf"                       # removes DWARF debug information
    "--strip-producers"                   # removes the "producers" section
    "--strip-target-features"             # removes the "target_features" section
)

echo "🗜️  Optimizing with wasm-opt with $WASM_BUILD_PROFILE profile..."
echo "ℹ️  before raw: $(du -h "${OUT_FOLDER}/query_engine_bg.wasm")"
echo "ℹ️  before zip: $(gzip -c "${OUT_FOLDER}/query_engine_bg.wasm" | wc -c) bytes"
case "$WASM_BUILD_PROFILE" in
    release)
        # In release mode, we want to strip the debug symbols.
        wasm-opt "${WASM_OPT_ARGS[@]}" \
            "--strip-debug" \
            "${OUT_FOLDER}/query_engine_bg.wasm" \
            -o "${OUT_FOLDER}/query_engine_bg.wasm"
        ;;
    profiling)
        # In profiling mode, we want to keep the debug symbols.
        wasm-opt "${WASM_OPT_ARGS[@]}" \
            "--debuginfo" \
            "${OUT_FOLDER}/query_engine_bg.wasm" \
            -o "${OUT_FOLDER}/query_engine_bg.wasm"
        ;;
    *)
        # In other modes (e.g., "dev"), skip wasm-opt.
        echo "Skipping wasm-opt."
        ;;
esac
echo "ℹ️  after raw: $(du -h "${OUT_FOLDER}/query_engine_bg.wasm")"
echo "ℹ️  after zip: $(gzip -c "${OUT_FOLDER}/query_engine_bg.wasm" | wc -c) bytes"

# Convert the `.wasm` file to its human-friendly `.wat` representation for debugging purposes, if `wasm2wat` is installed
if ! command -v wasm2wat &> /dev/null; then
    echo "Skipping wasm2wat, as it is not installed."
else
    wasm2wat "${OUT_FOLDER}/query_engine_bg.wasm" -o "./query_engine.wat"
fi

sleep 1
# Mark the package as a ES module, set the entry point to the query_engine.js file, mark the package as public
printf '%s\n' "$(jq '. + {"type": "module"} + {"main": "./query_engine.js"} + {"private": false}' "$OUT_JSON")" > "$OUT_JSON"

# Add the version
printf '%s\n' "$(jq --arg version "$OUT_VERSION" '. + {"version": $version}' "$OUT_JSON")" > "$OUT_JSON"

# Add the package name
printf '%s\n' "$(jq --arg name "$OUT_NPM_NAME" '. + {"name": $name}' "$OUT_JSON")" > "$OUT_JSON"

# Some info: enabling Cloudflare Workers in the bindings generated by wasm-package
# is useful for local experiments, but it's not needed here.
# `@prisma/client` has its own `esbuild` plugin for CF-compatible bindings
# and import of `.wasm` files.
enable_cf_in_bindings() {
    # Enable Cloudflare Workers in the generated JS bindings.
    # The generated bindings are compatible with:
    # - Node.js
    # - Cloudflare Workers / Miniflare

    local FILE="$1" # e.g., `query_engine.js`
    local BG_FILE="${FILE%.js}_bg.js"
    local OUTPUT_FILE="${OUT_FOLDER}/${FILE}"

    cat <<EOF > "$OUTPUT_FILE"
import * as imports from "./${BG_FILE}";

// switch between both syntax for Node.js and for workers (Cloudflare Workers)
import * as wkmod from "./${BG_FILE%.js}.wasm";
import * as nodemod from "./${BG_FILE%.js}.wasm";
if ((typeof process !== 'undefined') && (process.release.name === 'node')) {
    imports.__wbg_set_wasm(nodemod);
} else {
    const instance = new WebAssembly.Instance(wkmod.default, { "./${BG_FILE}": imports });
    imports.__wbg_set_wasm(instance.exports);
}

export * from "./${BG_FILE}";
EOF
}

enable_cf_in_bindings "query_engine.js"