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

OPENSSL_ARCH="android-arm64"
# if [ "$TARGET" = "aarch64-linux-android" ]; then
# fi

if [ "$TARGET" = "x86_64-linux-android" ]; then
  OPENSSL_ARCH="android-x86_64"
fi

if [ "$TARGET" = "armv7-linux-androideabi" ]; then
  OPENSSL_ARCH="android-arm"
fi

if [ "$TARGET" = "i686-linux-android" ]; then
  OPENSSL_ARCH="android-x86"
fi


API_VERSION="21"
NDK_VERSION="26.0.10792818"
NDK_HOST="darwin-x86_64"

if [ -z "$ANDROID_SDK_ROOT" ]; then
  echo "ANDROID SDK IS MISSING 🟥"
  exit 1
fi

if [ -z "$NDK" ]; then
  NDK="$ANDROID_SDK_ROOT/ndk/$NDK_VERSION"
fi

TOOLS="$NDK/toolchains/llvm/prebuilt/$NDK_HOST"

CWD=$(pwd)

export OPENSSL_DIR=$CWD/libs/$OPENSSL_ARCH
export OPENSSL_STATIC=1

# OPENSSL_DIR=./libs/android/clang/${OPENSSL_ARCH} \
AR=$TOOLS/bin/llvm-ar \
CC=$TOOLS/bin/${NDK_TARGET}${API_VERSION}-clang \
CXX=$TOOLS/bin/${NDK_TARGET}${API_VERSION}-clang++ \
RANLIB=$TOOLS/bin/llvm-ranlib \
CXXFLAGS="--target=$NDK_TARGET" \
cargo build --release --target "$TARGET"