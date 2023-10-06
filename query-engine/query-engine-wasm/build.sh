#!/bin/bash

# Call this script as `./build.sh <npm_version>`

OUT_VERSION="$1"
OUT_FOLDER="pkg"
OUT_JSON="${OUT_FOLDER}/package.json"
OUT_TARGET="bundler" # Note(jkomyno): I wasn't able to make it work with `web` target
OUT_NPM_NAME="@prisma/query-engine-wasm"

wasm-pack build --release --target $OUT_TARGET

sleep 1

# Mark the package as a ES module, set the entry point to the query_engine.js file, mark the package as public
printf '%s\n' "$(jq '. + {"type": "module"} + {"main": "./query_engine.js"} + {"private": false}' $OUT_JSON)" > $OUT_JSON

# Add the version
printf '%s\n' "$(jq --arg version $OUT_VERSION '. + {"version": $version}' $OUT_JSON)" > $OUT_JSON

# Add the package name
printf '%s\n' "$(jq --arg name $OUT_NPM_NAME '. + {"name": $name}' $OUT_JSON)" > $OUT_JSON

enable_cf_in_bindings() {
    # Enable Cloudflare Workers in the generated JS bindings.

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
    const instance = new WebAssembly.Instance(wkmod, { "./${BG_FILE}": imports });
    imports.__wbg_set_wasm(instance.exports);
}

export * from "./${BG_FILE}";
EOF
}

enable_cf_in_bindings "query_engine.js"
