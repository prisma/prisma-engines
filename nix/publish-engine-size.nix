/*
* Deprecated: This file is deprecated and will be removed soon.
* See https://github.com/prisma/team-orm/issues/943
*/
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
  fakeGitHash = "0000000000000000000000000000000000000000";
in
{
  packages.prisma-engines = stdenv.mkDerivation {
    name = "prisma-engines";
    inherit src;

    GIT_HASH = "${fakeGitHash}";

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
      darwin.apple_sdk.frameworks.SystemConfiguration
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

      GIT_HASH = "${fakeGitHash}";

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

      GIT_HASH = "${fakeGitHash}";

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

      GIT_HASH = "${fakeGitHash}";

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
      runtimeInputs = with pkgs; [ git rustup wasm-bindgen-cli binaryen jq iconv ];
      text = ''            
      cd query-engine/query-engine-wasm        
      WASM_BUILD_PROFILE=release ./build.sh "$1" "$2"
      '';
  };

  packages.query-engine-wasm-gz = lib.makeOverridable
    ({ profile }: stdenv.mkDerivation {
      name = "query-engine-wasm-gz";
      inherit src;
      buildInputs = with pkgs; [ iconv ];

      GIT_HASH = "${fakeGitHash}";

      buildPhase = ''
      export HOME=$(mktemp -dt wasm-engine-home-XXXX)
      
      OUT_FOLDER=$(mktemp -dt wasm-engine-out-XXXX)
      ${self'.packages.build-engine-wasm}/bin/build-engine-wasm "0.0.0" "$OUT_FOLDER" 

      for provider in "postgresql" "mysql" "sqlite"; do
        gzip -ckn "$OUT_FOLDER/$provider/query_engine_bg.wasm" > "query-engine-$provider.wasm.gz"
      done
      '';

      installPhase = ''
      set +x
      mkdir -p $out
      for provider in "postgresql" "mysql" "sqlite"; do
        cp "$OUT_FOLDER/$provider/query_engine_bg.wasm" "$out/query-engine-$provider.wasm"
        cp "query-engine-$provider.wasm.gz" "$out/"
      done
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

    /* Publish the size of the Query Engine binary and library to the CSV file
     in the `gh-pages` branch of the repository.

     Data: https://github.com/prisma/prisma-engines/blob/gh-pages/engines-size/data.csv
     Dashboard: https://prisma.github.io/prisma-engines/engines-size/
    */
  packages.publish-engine-size = pkgs.writeShellApplication {
    name = "publish-engine-size";
    text = ''
      set -euxo pipefail

      CURRENT_SYSTEM=$(uname -sm)

      if [[ "$CURRENT_SYSTEM" != "Linux x86_64" ]]; then
        : This script publishes the built engine size directly to the gh-pages
        : branch of the repository. Refusing to run on "$CURRENT_SYSTEM" to
        : avoid inconsistent data being published. Please run the script on
        : Linux x86_64 if you want to update the data manually, or use
        : "nix run .#update-engine-size" for local testing without modifying
        : the data in gh-pages branch.
        exit 1
      fi

      if ! git diff --exit-code 1> /dev/null; then
        : "The workspace is not clean. Please commit or reset, then try again."
        exit 1
      fi

      CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
      CURRENT_COMMIT=$(git rev-parse HEAD)
      CURRENT_COMMIT_SHORT=$(git rev-parse --short HEAD)
      REPO_ROOT=$(git rev-parse --show-toplevel)
      CSV_PATH="engines-size/data.csv"

      pushd "$REPO_ROOT"
      git fetch --depth=1 origin gh-pages
      git checkout origin/gh-pages

      export CSV_PATH
      export CURRENT_BRANCH
      export CURRENT_COMMIT

      ${self'.packages.update-engine-size}/bin/update-engine-size             \
          ${self'.packages.query-engine-bin-and-lib}/bin/query-engine         \
          ${self'.packages.query-engine-bin-and-lib}/lib/libquery_engine.node \
          ${self'.packages.query-engine-wasm-gz}/query-engine-postgresql.wasm.gz           \
          ${self'.packages.query-engine-wasm-gz}/query-engine-postgresql.wasm              \
          ${self'.packages.query-engine-wasm-gz}/query-engine-mysql.wasm.gz                \
          ${self'.packages.query-engine-wasm-gz}/query-engine-mysql.wasm                   \
          ${self'.packages.query-engine-wasm-gz}/query-engine-sqlite.wasm.gz               \
          ${self'.packages.query-engine-wasm-gz}/query-engine-sqlite.wasm

      git add "$CSV_PATH"
      git commit --quiet -m "update engines size for $CURRENT_COMMIT_SHORT"
      git push origin '+HEAD:gh-pages'
      git checkout "$CURRENT_BRANCH"
      popd
    '';
  };

  packages.update-engine-size = pkgs.writeShellApplication {
    name = "update-engine-size";
    text = ''
      set -euxo pipefail

      DATE_TIME="$(date -u --iso-8601=seconds)"

      if [[ ! -f $CSV_PATH ]]; then
        echo "date_time,branch,commit,file,size_bytes" > "$CSV_PATH"
      fi

      for file in "$@"; do
        file_name=$(basename "$file")
        size=$(stat -c %s "$file")
        echo "$DATE_TIME,$CURRENT_BRANCH,$CURRENT_COMMIT,$file_name,$size" >> "$CSV_PATH"
      done
    '';
  };
}
