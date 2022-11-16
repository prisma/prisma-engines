args@{ pkgs, system, src, ... }:

let
  toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
  craneLib = args.craneLib.overrideToolchain toolchain;

  inherit (pkgs) jq nodejs coreutils rustPlatform wasm-bindgen-cli;
  inherit (builtins) readFile replaceStrings;
in
rec {
  packages = {
    prisma-fmt-wasm = craneLib.buildPackage {
      pname = "prisma-fmt-wasm";
      version = "0.1.0";

      nativeBuildInputs = with pkgs; [ git wasm-bindgen-cli ];

      cargoBuildCommand = "cargo build --release --target=wasm32-unknown-unknown --manifest-path=prisma-fmt-wasm/Cargo.toml";
      cargoCheckCommand = "cargo check --target=wasm32-unknown-unknown --manifest-path=prisma-fmt-wasm/Cargo.toml";
      doCheck = false; # do not run cargo test
      cargoArtifacts = null; # do not cache dependencies

      installPhase = readFile ./scripts/install.sh;

      inherit src;
    };

    # Takes a package version as its single argument, and produces
    # prisma-fmt-wasm with the right package.json in a temporary directory,
    # then prints the directory's path. This is used by the publish pipeline in CI.
    renderPrismaFmtWasmPackage =
      pkgs.writeShellApplication {
        name = "renderPrismaFmtWasmPackage";
        runtimeInputs = [ jq ];
        text = ''
          set -euxo pipefail

          PACKAGE_DIR=$(mktemp -d)
          cp -r --no-target-directory ${packages.prisma-fmt-wasm} "$PACKAGE_DIR"
          rm -f "$PACKAGE_DIR/package.json"
          jq ".version = \"$1\"" ${packages.prisma-fmt-wasm}/package.json > "$PACKAGE_DIR/package.json"
          echo "$PACKAGE_DIR"
        '';
      };

    syncWasmBindgenVersions = let template = readFile ./scripts/syncWasmBindgenVersions.sh; in
      pkgs.writeShellApplication {
        name = "syncWasmBindgenVersions";
        runtimeInputs = [ coreutils ];
        text = replaceStrings [ "$WASM_BINDGEN_VERSION" ] [ wasm-bindgen-cli.version ] template;
      };
  };

  checks = {
    prismaFmtWasmE2E = pkgs.runCommand "prismaFmtWasmE2E"
      { PRISMA_FMT_WASM = packages.prisma-fmt-wasm; NODE = "${nodejs}/bin/node"; }
      (readFile ./scripts/check.sh);
  };
}
