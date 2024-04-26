#!/bin/bash

set -ex

TARGET="$1"

if [ "$TARGET" = "" ]; then
    echo "missing argument TARGET"
    echo "Usage: $0 TARGET"
    exit 1
fi

NDK_TARGET=$TARGET

if [ "$TARGET" = "armv7-linux-androideabi" ]; then
    NDK_TARGET="armv7a-linux-androideabi"
fi

API_VERSION="21"
# shellcheck source=/dev/null
source "$ANDROID_NDK_ROOT/build/tools/ndk_bin_common.sh"
echo "Host tag: $HOST_TAG"

if [ -z "$ANDROID_NDK_ROOT" ]; then
  echo "ANDROID NDK IS MISSING 🟥"
  exit 1
fi

TOOLS="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/$HOST_TAG"

AR=$TOOLS/bin/llvm-ar \
CC=$TOOLS/bin/${NDK_TARGET}${API_VERSION}-clang \
CXX=$TOOLS/bin/${NDK_TARGET}${API_VERSION}-clang++ \
RANLIB=$TOOLS/bin/llvm-ranlib \
CXXFLAGS="--target=$NDK_TARGET" \
cargo build --release -p query-engine-c-abi --target "$TARGET" --config "target.$TARGET.linker=\"$TOOLS/bin/${NDK_TARGET}${API_VERSION}-clang\""