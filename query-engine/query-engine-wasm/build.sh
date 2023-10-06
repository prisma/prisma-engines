#!/bin/bash

# Call this script as `./build.sh <npm_version>`

OUT_VERSION="$1"
OUT_JSON="pkg/package.json"
OUT_TARGET="bundler" # Note(jkomyno): I wasn't able to make it work with `web` target
OUT_NPM_NAME="@jkomyno/query-engine-wasm"

wasm-pack build --release --target $OUT_TARGET

# Mark the package as a ES module, set the entry point to the query_engine.js file, mark the package as public
printf '%s\n' "$(jq '. + {"type": "module"} + {"main": "./query_engine.js"} + {"private": false}' $OUT_JSON)" > $OUT_JSON

# Add the version
printf '%s\n' "$(jq --arg version $OUT_VERSION '. + {"version": $version}' $OUT_JSON)" > $OUT_JSON

# Add the package name
printf '%s\n' "$(jq --arg name $OUT_NPM_NAME '. + {"name": $name}' $OUT_JSON)" > $OUT_JSON
