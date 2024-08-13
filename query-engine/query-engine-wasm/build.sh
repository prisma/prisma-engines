#!/bin/bash
# Call this script as `./build.sh <npm_version>`
set -euo pipefail

CURRENT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
REPO_ROOT="$( cd "$( dirname "$CURRENT_DIR/../../../" )" >/dev/null 2>&1 && pwd )"
OUT_VERSION="${1:-"0.0.0"}"
OUT_FOLDER="${2:-"query-engine/query-engine-wasm/pkg"}"
OUT_TARGET="bundler"
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
    "--gufa-optimizing"                   # optimize the entire program using type monomorphization
    "--strip-dwarf"                       # removes DWARF debug information
    "--strip-producers"                   # removes the "producers" section
    "--strip-target-features"             # removes the "target_features" section
)

# if it's a relative path, let it be relative to the repo root
if [[ "$OUT_FOLDER" != /* ]]; then
    OUT_FOLDER="$REPO_ROOT/$OUT_FOLDER"
fi
OUT_JSON="${OUT_FOLDER}/package.json"

echo "ℹ️  target version: $OUT_VERSION"
echo "ℹ️  out folder: $OUT_FOLDER"

if [[ -z "${WASM_BUILD_PROFILE:-}" ]]; then
    if [[ -z "${BUILDKITE:-}" ]] && [[ -z "${GITHUB_ACTIONS:-}" ]]; then
        WASM_BUILD_PROFILE="dev"
    else
        WASM_BUILD_PROFILE="release"
    fi
fi

if [ "$WASM_BUILD_PROFILE" = "dev" ]; then
    WASM_TARGET_SUBDIR="debug"
else
    WASM_TARGET_SUBDIR="$WASM_BUILD_PROFILE"
fi



build() {
    echo "ℹ️  Note that query-engine compiled to WASM uses a different Rust toolchain"
    cargo --version

    local CONNECTOR="$1"
    local CARGO_TARGET_DIR
    CARGO_TARGET_DIR=$(cargo metadata --format-version 1 | jq -r .target_directory)
    echo "🔨 Building $CONNECTOR"
    RUSTFLAGS="-Zlocation-detail=none" CARGO_PROFILE_RELEASE_OPT_LEVEL="z" cargo build \
        -p query-engine-wasm \
        --profile "$WASM_BUILD_PROFILE" \
        --features "$CONNECTOR" \
        --target wasm32-unknown-unknown \
        -Zbuild-std=std,panic_abort -Zbuild-std-features=panic_immediate_abort

    local IN_FILE="$CARGO_TARGET_DIR/wasm32-unknown-unknown/$WASM_TARGET_SUBDIR/query_engine_wasm.wasm"
    local OUT_FILE="$OUT_FOLDER/$CONNECTOR/query_engine_bg.wasm"

    wasm-bindgen --target "$OUT_TARGET" --out-name query_engine --out-dir "$OUT_FOLDER/$CONNECTOR" "$IN_FILE"
    optimize "$OUT_FILE"

    if ! command -v wasm2wat &> /dev/null; then
        echo "Skipping wasm2wat, as it is not installed."
    else
        wasm2wat "$OUT_FILE" -o "./query_engine.$CONNECTOR.wat"
    fi
}

optimize() {
    local OUT_FILE="$1"
    case "$WASM_BUILD_PROFILE" in
        release)
            # In release mode, we want to strip the debug symbols.
            wasm-opt "${WASM_OPT_ARGS[@]}" \
                "--strip-debug" \
                "$OUT_FILE" \
                -o "$OUT_FILE"
            ;;
        profiling)
            # In profiling mode, we want to keep the debug symbols.
            wasm-opt "${WASM_OPT_ARGS[@]}" \
                "--debuginfo" \
                "${OUT_FILE}" \
                -o "${OUT_FILE}"
            ;;
        *)
            # In other modes (e.g., "dev"), skip wasm-opt.
            echo "Skipping wasm-opt."
            ;;
    esac
}

report_size() {
    local CONNECTOR
    local GZ_SIZE
    local FORMATTED_GZ_SIZE

    CONNECTOR="$1"
    GZ_SIZE=$(gzip -c "${OUT_FOLDER}/$CONNECTOR/query_engine_bg.wasm" | wc -c)
    FORMATTED_GZ_SIZE=$(echo "$GZ_SIZE"|numfmt --format '%.3f' --to=iec-i --suffix=B)

    echo "$CONNECTOR:"
    echo "ℹ️  raw: $(du -h "${OUT_FOLDER}/$CONNECTOR/query_engine_bg.wasm")"
    echo "ℹ️  zip: $GZ_SIZE bytes ($FORMATTED_GZ_SIZE)"
    echo ""
}

echo "Building query-engine-wasm using $WASM_BUILD_PROFILE profile"

build "postgresql"
build "sqlite"
build "mysql"

jq '.version=$version' --arg version "$OUT_VERSION" package.json > "$OUT_JSON"

report_size "postgresql"
report_size "sqlite"
report_size "mysql"
