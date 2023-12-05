#!/bin/bash

set -e

# Call this script as `./build.sh <npm_version>`
set -euo pipefail

OUT_VERSION="${1:-}"
OUT_FOLDER="pkg"
OUT_JSON="${OUT_FOLDER}/package.json"
OUT_TARGET="bundler"
OUT_NPM_NAME="@prisma/query-engine-wasm"

# use `wasm-pack build --release` on CI only
if [[ -z "${BUILDKITE:-}" ]] && [[ -z "${GITHUB_ACTIONS:-}" ]]; then
    BUILD_PROFILE="--dev"
else
    BUILD_PROFILE="--release"
fi

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null
then
    echo "wasm-pack could not be found, installing now..."
    # Install wasm-pack
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

wasm-pack build $BUILD_PROFILE --target $OUT_TARGET --out-name query_engine

sleep 1

# Mark the package as a ES module, set the entry point to the query_engine.js file, mark the package as public
printf '%s\n' "$(jq '. + {"type": "module"} + {"main": "./query_engine.js"} + {"private": false}' $OUT_JSON)" > $OUT_JSON

# Add the version
printf '%s\n' "$(jq --arg version "$OUT_VERSION" '. + {"version": $version}' $OUT_JSON)" > $OUT_JSON

# Add the package name
printf '%s\n' "$(jq --arg name "$OUT_NPM_NAME" '. + {"name": $name}' $OUT_JSON)" > $OUT_JSON

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
