{ craneLib, pkgs, inputs, ... }:

let
  srcPath = builtins.path { path = ../.; name = "prisma-engines-workspace-root-path"; };
  src = pkgs.lib.cleanSourceWith { filter = enginesSourceFilter; src = srcPath; };
  craneLib = inputs.crane.mkLib pkgs;
  deps = craneLib.vendorCargoDeps { inherit src; };

  enginesSourceFilter = path: type: (builtins.match "\\.pest$" path != null) ||
    (builtins.match "\\.README.md$" path != null) ||
    (builtins.match "^\\.git/HEAD" path != null) ||
    (builtins.match "^\\.git/refs" path != null) ||
    (craneLib.filterCargoSources path type != null);
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
      cp target/release/libquery_engine.so $out/lib/libquery_engine.node
    '';
  };
}
