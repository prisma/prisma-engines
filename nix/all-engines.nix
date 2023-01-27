{ pkgs, flakeInputs, lib, ... }:

let
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
  craneLib = flakeInputs.crane.mkLib pkgs;
  deps = craneLib.vendorCargoDeps { inherit src; };
  libSuffix = if pkgs.stdenv.isDarwin then "dylib" else "so";
in
{
  packages.prisma-engines-deps = deps;
  packages.prisma-engines = pkgs.stdenv.mkDerivation {
    name = "prisma-engines";
    inherit src;

    buildInputs = [ pkgs.openssl.out ];
    nativeBuildInputs = with pkgs; [
      cargo
      git # for our build scripts that bake in the git hash
      protobuf # for tonic
      openssl.dev
      pkg-config
    ] ++ lib.optionals stdenv.isDarwin [
      perl # required to build openssl
      darwin.apple_sdk.frameworks.Security
    ];

    buildPhase = ''
      mkdir .cargo
      ln -s ${deps}/config.toml .cargo/config.toml
      cargo build --release --bins
      cargo build --release -p query-engine-node-api
    '';

    installPhase = ''
      mkdir -p $out/bin $out/lib
      cp target/release/query-engine $out/bin/
      cp target/release/migration-engine $out/bin/
      cp target/release/introspection-engine $out/bin/
      cp target/release/prisma-fmt $out/bin/
      cp target/release/libquery_engine.${libSuffix} $out/lib/libquery_engine.node
    '';
  };
}
