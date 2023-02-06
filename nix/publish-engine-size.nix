{ pkgs, self', ... }:

let stdenv = pkgs.clangStdenv;
in
{
  packages.update-engine-size = stdenv.mkDerivation {
    name = "update-engine-size";
    inherit (self'.packages.prisma-engines) buildInputs nativeBuildInputs src configurePhase;

    buildPhase = "cargo build --release --bin update-engine-size";

    installPhase = ''
      mkdir -p $out/bin
      cp target/release/update-engine-size $out/bin/update-engine-size
    '';
  };

  packages.publish-engine-size = pkgs.writeShellApplication {
    name = "publish-engine-size";
    text = ''
      set -euxo pipefail

      if ! git diff --exit-code 1> /dev/null; then
        : "The workspace is not clean. Please commit or reset, then try again".
        exit 1
      fi

      CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
      CURRENT_COMMIT_SHORT=$(git rev-parse --short HEAD)
      CURRENT_COMMIT_FULL=$(git rev-parse HEAD)
      REPO_ROOT=$(git rev-parse --show-toplevel)

      pushd "$REPO_ROOT"
      git fetch --depth=1 origin gh-pages
      git checkout origin/gh-pages

      ${self'.packages.update-engine-size}/bin/update-engine-size             \
          --db engines-size/data.csv                                                \
          --branch "$CURRENT_BRANCH"                                          \
          --commit "$CURRENT_COMMIT_FULL"                                     \
          ${self'.packages.query-engine-bin-and-lib}/bin/query-engine         \
          ${self'.packages.query-engine-bin-and-lib}/lib/libquery_engine.node

      git add engines-size/data.csv
      git commit --quiet -m "update engines size for $CURRENT_COMMIT_SHORT"
      git push origin '+HEAD:gh-pages'
      git checkout "$CURRENT_BRANCH"
      popd
    '';
  };
}
