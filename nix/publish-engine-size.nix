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

      ls ${self'.packages.query-engine-bin-and-lib}/lib

      # if ! git diff --exit-code 1> /dev/null; then
      #   : "The workspace is not clean. Please commit or reset, then try again".
      #   exit 1
      # fi

      # CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
      # CURRENT_COMMIT=$(git rev-parse --short HEAD)
      # REPO_ROOT=$(git rev-parse --show-toplevel)

      # pushd "$REPO_ROOT"
      # git fetch --depth=1 origin gh-pages
      # git checkout origin/gh-pages

      # git push origin '+HEAD:gh-pages'
      # git checkout "$CURRENT_BRANCH"
      # popd
    '';
  };
}
