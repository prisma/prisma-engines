{ pkgs, system, self', ... }:

let
  toolchain = pkgs.rust-bin.fromRustupToolchainFile ../prisma-fmt-wasm/rust-toolchain.toml;
  scriptsDir = ../prisma-fmt-wasm/scripts;
  inherit (pkgs) jq nodejs coreutils wasm-bindgen-cli stdenv;
  inherit (builtins) readFile replaceStrings;
in
{
  packages.prisma-fmt-wasm = stdenv.mkDerivation {
    name = "prisma-fmt-wasm";
    nativeBuildInputs = with pkgs; [ git wasm-bindgen-cli toolchain ];
    inherit (self'.packages.prisma-engines) configurePhase src;

    buildPhase = "cargo build --release --target=wasm32-unknown-unknown -p prisma-fmt-build";
    installPhase = readFile "${scriptsDir}/install.sh";
  };

  # Takes a package version as its single argument, and produces
  # prisma-fmt-wasm with the right package.json in a temporary directory,
  # then prints the directory's path. This is used by the publish pipeline in CI.
  packages.renderPrismaFmtWasmPackage =
    pkgs.writeShellApplication {
      name = "renderPrismaFmtWasmPackage";
      runtimeInputs = [ jq ];
      text = ''
        set -euxo pipefail

        PACKAGE_DIR=$(mktemp -d)
        cp -r --no-target-directory ${self'.packages.prisma-fmt-wasm} "$PACKAGE_DIR"
        rm -f "$PACKAGE_DIR/package.json"
        jq ".version = \"$1\"" ${self'.packages.prisma-fmt-wasm}/package.json > "$PACKAGE_DIR/package.json"
        echo "$PACKAGE_DIR"
      '';
    };

  packages.syncWasmBindgenVersions = let template = readFile "${scriptsDir}/syncWasmBindgenVersions.sh"; in
    pkgs.writeShellApplication {
      name = "syncWasmBindgenVersions";
      runtimeInputs = [ coreutils toolchain ];
      text = replaceStrings [ "$WASM_BINDGEN_VERSION" ] [ wasm-bindgen-cli.version ] template;
    };

  checks.prismaFmtWasmE2E = pkgs.runCommand "prismaFmtWasmE2E"
    { PRISMA_FMT_WASM = self'.packages.prisma-fmt-wasm; NODE = "${nodejs}/bin/node"; }
    (readFile "${scriptsDir}/check.sh");
}
