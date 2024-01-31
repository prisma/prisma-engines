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

  packages.build-engine-wasm = pkgs.writeShellApplication { 
    name = "build-engine-wasm";
      runtimeInputs = with pkgs; [ git rustup wasm-pack wasm-bindgen-cli binaryen jq iconv];
      text = ''            
      cd query-engine/query-engine-wasm        
      WASM_BUILD_PROFILE=release ./build.sh "$1" "$2"
      '';
  };

  packages.query-engine-wasm-gz = lib.makeOverridable
    ({ profile }: stdenv.mkDerivation {
      name = "query-engine-wasm-gz";
      inherit src;

      buildPhase = ''
      export HOME=$(mktemp -dt wasm-engine-home-XXXX)
      
      OUT_FOLDER=$(mktemp -dt wasm-engine-out-XXXX)
      ${self'.packages.build-engine-wasm}/bin/build-engine-wasm "0.0.0" "$OUT_FOLDER" 
      gzip -ckn "$OUT_FOLDER/query_engine_bg.wasm" > query_engine_bg.wasm.gz
      '';

      installPhase = ''
      mkdir -p $out
      cp "$OUT_FOLDER/query_engine_bg.wasm" $out/
      cp query_engine_bg.wasm.gz $out/
      '';
    })
    { profile = "release"; };

  packages.export-query-engine-wasm =
    pkgs.writeShellApplication {
      name = "export-query-engine-wasm";
      runtimeInputs = with pkgs; [ jq ];
      text = ''              
        OUT_VERSION="$1"
        OUT_FOLDER="$2"

        mkdir -p "$OUT_FOLDER"
        ${self'.packages.build-engine-wasm}/bin/build-engine-wasm "$OUT_VERSION" "$OUT_FOLDER"
        chmod -R +rw "$OUT_FOLDER"
        mv "$OUT_FOLDER/package.json" "$OUT_FOLDER/package.json.bak"         
        jq --arg new_version "$OUT_VERSION" '.version = $new_version' "$OUT_FOLDER/package.json.bak" > "$OUT_FOLDER/package.json"        
      '';
    };
}
