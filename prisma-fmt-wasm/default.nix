{ pkgs, system, self', ... }:

let
  toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
  deps = self'.packages.prisma-engines-deps;
  inherit (pkgs) jq nodejs coreutils wasm-bindgen-cli stdenv;
  inherit (builtins) readFile replaceStrings;
in
{
  packages.prisma-fmt-wasm = stdenv.mkDerivation {
    name = "prisma-fmt-wasm";
    src = ../.;
    nativeBuildInputs = with pkgs; [ git wasm-bindgen-cli toolchain ];

    configurePhase = "mkdir .cargo && ln -s ${deps}/config.toml .cargo/config.toml";
    buildPhase = "cargo build --release --target=wasm32-unknown-unknown -p prisma-fmt-build";
    installPhase = readFile ./scripts/install.sh;
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

  packages.syncWasmBindgenVersions = let template = readFile ./scripts/syncWasmBindgenVersions.sh; in
    pkgs.writeShellApplication {
      name = "syncWasmBindgenVersions";
      runtimeInputs = [ coreutils toolchain ];
      text = replaceStrings [ "$WASM_BINDGEN_VERSION" ] [ wasm-bindgen-cli.version ] template;
    };

  checks.prismaFmtWasmE2E = pkgs.runCommand "prismaFmtWasmE2E"
    { PRISMA_FMT_WASM = self'.packages.prisma-fmt-wasm; NODE = "${nodejs}/bin/node"; }
    (readFile ./scripts/check.sh);
}
