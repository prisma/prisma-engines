{ pkgs, flakeInputs, lib, self', rustToolchain, ... }:

let
  stdenv = pkgs.clangStdenv;
  srcPath = ../.;
  srcFilter = flakeInputs.gitignore.lib.gitignoreFilterWith {
    basePath = srcPath;
    extraRules = ''
      /nix
      /flake.*
    '';
  };
  src = lib.cleanSourceWith {
    filter = srcFilter;
    src = srcPath;
    name = "prisma-engines-source";
  };
  craneLib = (flakeInputs.crane.mkLib pkgs).overrideToolchain rustToolchain;
  deps = craneLib.vendorCargoDeps { inherit src; };
  libSuffix = stdenv.hostPlatform.extensions.sharedLibrary;
in
{
  packages.prisma-engines = stdenv.mkDerivation {
    name = "prisma-engines";
    inherit src;

    buildInputs = [ pkgs.openssl.out ];
    nativeBuildInputs = with pkgs; [
      rustToolchain
      git # for our build scripts that bake in the git hash
      protobuf # for tonic
      openssl.dev
      pkg-config
    ] ++ lib.optionals stdenv.isDarwin [
      perl # required to build openssl
      darwin.apple_sdk.frameworks.Security
      iconv
    ];

    configurePhase = ''
      mkdir .cargo
      ln -s ${deps}/config.toml .cargo/config.toml
    '';

    buildPhase = ''
      cargo build --release --bins
      cargo build --release -p query-engine-node-api
    '';

    installPhase = ''
      mkdir -p $out/bin $out/lib
      cp target/release/query-engine $out/bin/
      cp target/release/schema-engine $out/bin/
      cp target/release/prisma-fmt $out/bin/
      cp target/release/libquery_engine${libSuffix} $out/lib/libquery_engine.node
    '';

    dontStrip = true;
  };

  packages.test-cli = lib.makeOverridable
    ({ profile }: stdenv.mkDerivation {
      name = "test-cli";
      inherit src;
      inherit (self'.packages.prisma-engines) buildInputs nativeBuildInputs configurePhase dontStrip;

      buildPhase = "cargo build --profile=${profile} --bin=test-cli";

      installPhase = ''
        set -eu
        mkdir -p $out/bin
        QE_PATH=$(find target -name 'test-cli')
        cp $QE_PATH $out/bin
      '';
    })
    { profile = "release"; };

  packages.query-engine-bin = lib.makeOverridable
    ({ profile }: stdenv.mkDerivation {
      name = "query-engine-bin";
      inherit src;
      inherit (self'.packages.prisma-engines) buildInputs nativeBuildInputs configurePhase dontStrip;

      buildPhase = "cargo build --profile=${profile} --bin=query-engine";

      installPhase = ''
        set -eu
        mkdir -p $out/bin
        QE_PATH=$(find target -name 'query-engine')
        cp $QE_PATH $out/bin
      '';
    })
    { profile = "release"; };

  # TODO: try to make caching and sharing the build artifacts work with crane.  There should be
  # separate `query-engine-lib` and `query-engine-bin` derivations instead, but we use this for now
  # to make the CI job that uses it faster.
  packages.query-engine-bin-and-lib = lib.makeOverridable
    ({ profile }: stdenv.mkDerivation {
      name = "query-engine-bin-and-lib";
      inherit src;
      inherit (self'.packages.prisma-engines) buildInputs nativeBuildInputs configurePhase dontStrip;

      buildPhase = ''
        cargo build --profile=${profile} --bin=query-engine
        cargo build --profile=${profile} -p query-engine-node-api
      '';

      installPhase = ''
        set -eu
        mkdir -p $out/bin $out/lib
        cp target/${profile}/query-engine $out/bin/query-engine
        cp target/${profile}/libquery_engine${libSuffix} $out/lib/libquery_engine.node
      '';
    })
    { profile = "release"; };

  packages.query-engine-wasm = lib.makeOverridable
    ({ profile }: stdenv.mkDerivation {
      name = "query-engine-wasm";
      inherit src;
      inherit (self'.packages.prisma-engines) buildInputs configurePhase dontStrip;

      nativeBuildInputs = self'.packages.prisma-engines.nativeBuildInputs ++ (with pkgs; [wasm-pack wasm-bindgen-cli jq binaryen]);

      buildPhase = ''
        cd query-engine/query-engine-wasm
        HOME=$(mktemp -dt wasm-pack-home-XXXX) WASM_BUILD_PROFILE=${profile} ./build.sh
      '';

      installPhase = ''
      cp -r pkg $out
      '';
    })
    { profile = "release"; };

  packages.query-engine-wasm-gz = lib.makeOverridable
    ({ profile }: stdenv.mkDerivation {
      name = "query-engine-wasm-gz";
      inherit src;

      buildPhase = ''
      gzip -cn ${self'.packages.query-engine-wasm}/query_engine_bg.wasm > query_engine_bg.wasm.gz
      '';

      installPhase = ''
      mkdir -p $out
      cp query_engine_bg.wasm.gz $out/
      '';
    })
    { profile = "release"; };
}
