{ craneLib, pkgs, ... }:

let
  enginesSourceFilter = path: type: (builtins.match "\\.pest$" path != null) ||
    (builtins.match "\\.README.md$" path != null) ||
    (builtins.match "^\\.git/HEAD" path != null) ||
    (builtins.match "^\\.git/refs" path != null) ||
    (craneLib.filterCargoSources path type != null);
  src = pkgs.lib.cleanSourceWith {
    filter = enginesSourceFilter;
    src = builtins.path {
      path = ../.;
      name = "prisma-engines-workspace-root-path";
    };
  };

  craneArgs = {
    pname = "prisma-engines";
    version = "0.1.0";
    buildInputs = [ pkgs.openssl ];
    cargoExtraArgs = "--workspace --all-features --bins";
    nativeBuildInputs = with pkgs; [
      git # for our build scripts that bake in the git hash
      perl # for openssl-sys
      pkg-config
      protobuf # for tonic
    ];
    doCheck = false;
    inherit src;
  };
in
  {
    packages.prisma-engines-deps = craneLib.buildDepsOnly craneArgs;
    packages.prisma-engines = craneLib.buildPackage craneArgs;
  }
