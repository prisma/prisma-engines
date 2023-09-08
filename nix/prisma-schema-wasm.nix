{ pkgs, system, self', ... }:

let
  toolchain = pkgs.rust-bin.fromRustupToolchainFile ../prisma-schema-wasm/rust-toolchain.toml;
  scriptsDir = ../prisma-schema-wasm/scripts;
  inherit (pkgs) jq nodejs coreutils wasm-bindgen-cli stdenv;
  inherit (builtins) readFile replaceStrings;
in
{
  packages.prisma-schema-wasm = stdenv.mkDerivation {
    name = "prisma-schema-wasm";
    nativeBuildInputs = with pkgs; [ git wasm-bindgen-cli toolchain ];
    inherit (self'.packages.prisma-engines) configurePhase src;

    buildPhase = "cargo build --release --target=wasm32-unknown-unknown -p prisma-schema-build";
    installPhase = readFile "${scriptsDir}/install.sh";
  };

  # Takes a package version as its single argument, and produces
  # prisma-schema-wasm with the right package.json in a temporary directory,
  # then prints the directory's path. This is used by the publish pipeline in CI.
  packages.renderPrismaSchemaWasmPackage =
    pkgs.writeShellApplication {
      name = "renderPrismaSchemaWasmPackage";
      runtimeInputs = [ jq ];
      text = ''
        set -euxo pipefail

        PACKAGE_DIR=$(mktemp -d)
        cp -r --no-target-directory ${self'.packages.prisma-schema-wasm} "$PACKAGE_DIR"
        rm -f "$PACKAGE_DIR/package.json"
        jq ".version = \"$1\"" ${self'.packages.prisma-schema-wasm}/package.json > "$PACKAGE_DIR/package.json"
        echo "$PACKAGE_DIR"
      '';
    };

  packages.syncWasmBindgenVersions = let template = readFile "${scriptsDir}/syncWasmBindgenVersions.sh"; in
    pkgs.writeShellApplication {
      name = "syncWasmBindgenVersions";
      runtimeInputs = [ coreutils toolchain ];
      text = replaceStrings [ "$WASM_BINDGEN_VERSION" ] [ wasm-bindgen-cli.version ] template;
    };

  checks.prismaSchemaWasmE2E = pkgs.runCommand "prismaSchemaWasmE2E"
    { PRISMA_SCHEMA_WASM = self'.packages.prisma-schema-wasm; NODE = "${nodejs}/bin/node"; }
    (readFile "${scriptsDir}/check.sh");
}
