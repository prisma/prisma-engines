{ pkgs, self', ... }:

{
  packages.cargo-docs = pkgs.clangStdenv.mkDerivation {
    name = "prisma-engines-cargo-docs";
    inherit (self'.packages.prisma-engines) buildInputs nativeBuildInputs src;

    buildPhase = ''
      mkdir .cargo
      ln -s ${self'.packages.prisma-engines-deps}/config.toml .cargo/config.toml
      cargo doc --workspace
    '';

    installPhase = ''
      mkdir -p $out/share
      mv target/doc/ $out/share/docs
    '';
  };

  packages.publish-cargo-docs = pkgs.writeShellApplication {
    name = "publish-cargo-docs";
    text = ''
      set -euxo pipefail

      if ! git diff --exit-code 1> /dev/null; then
        : "The workspace is not clean. Please commit or reset, then try again".
        exit 1
      fi

      CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
      CURRENT_COMMIT=$(git rev-parse --short HEAD)
      REPO_ROOT=$(git rev-parse --show-toplevel)

      pushd "$REPO_ROOT"
      git fetch --depth=1 origin gh-pages
      git checkout origin/gh-pages
      rm -rf ./doc
      cp \
        --recursive \
        --no-preserve=mode,ownership \
        ${self'.packages.cargo-docs}/share/docs \
        ./doc
      git add doc
      git commit --quiet -m "cargo docs for $CURRENT_COMMIT"
      git push origin '+HEAD:gh-pages'
      git checkout "$CURRENT_BRANCH"
      popd
    '';
  };
}
