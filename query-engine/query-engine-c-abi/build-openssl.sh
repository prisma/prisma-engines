#!/bin/bash

#set -v
set -ex

export OPENSSL_VERSION="openssl-3.1.4"
rm -rf ${OPENSSL_VERSION}
# check if the tar is already downloaded and if not download and extract it
if [ ! -d ${OPENSSL_VERSION}.tar.gz ]; then
	curl -O "https://www.openssl.org/source/${OPENSSL_VERSION}.tar.gz"
	tar xfz "${OPENSSL_VERSION}.tar.gz"
fi

PATH_ORG=$PATH
OUTPUT_DIR="libs"

# Clean output:
rm -rf $OUTPUT_DIR
mkdir $OUTPUT_DIR

build_android_clang() {

	echo ""
	echo "----- Build libcrypto & libssl.so for $1 -----"
	echo ""

	ARCHITECTURE=$1
	TOOLCHAIN=$2

	# Set toolchain
	export TOOLCHAIN_ROOT=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64
	export SYSROOT=$TOOLCHAIN_ROOT/sysroot
	export CC=${TOOLCHAIN}21-clang
	export CXX=${TOOLCHAIN}21-clang++
	export CXXFLAGS="-fPIC"
	export CPPFLAGS="-DANDROID -fPIC"

	export PATH=$TOOLCHAIN_ROOT/bin:$SYSROOT/usr/local/bin:$PATH

	cd "${OPENSSL_VERSION}"

	./Configure "$ARCHITECTURE" no-asm no-shared -D__ANDROID_API__=21

	make clean
	# Apply patch that fixes the armcap instruction
	# Linux version
	# sed -e '/[.]hidden.*OPENSSL_armcap_P/d; /[.]extern.*OPENSSL_armcap_P/ {p; s/extern/hidden/ }' -i -- crypto/*arm*pl crypto/*/asm/*arm*pl
	# macOS version
	sed -E -i '' -e '/[.]hidden.*OPENSSL_armcap_P/d' -e '/[.]extern.*OPENSSL_armcap_P/ {p; s/extern/hidden/; }' crypto/*arm*pl crypto/*/asm/*arm*pl

	make

	mkdir -p ../$OUTPUT_DIR/"${ARCHITECTURE}"/lib
	mkdir -p ../$OUTPUT_DIR/"${ARCHITECTURE}"/include

	# file libcrypto.so
	# file libssl.so

	cp libcrypto.a ../$OUTPUT_DIR/"${ARCHITECTURE}"/lib/libcrypto.a
	cp libssl.a ../$OUTPUT_DIR/"${ARCHITECTURE}"/lib/libssl.a
	# cp libcrypto.so ../$OUTPUT_DIR/${ARCHITECTURE}/lib/libcrypto.so
	# cp libssl.so ../$OUTPUT_DIR/${ARCHITECTURE}/lib/libssl.so

	cp -R include/openssl ../$OUTPUT_DIR/"${ARCHITECTURE}"/include

	cd ..
}

build_android_clang "android-arm" "armv7a-linux-androideabi"
build_android_clang "android-x86" "i686-linux-android"
build_android_clang "android-x86_64" "x86_64-linux-android"
build_android_clang "android-arm64" "aarch64-linux-android"

export PATH=$PATH_ORG

# pingme "OpenSSL finished compiling"