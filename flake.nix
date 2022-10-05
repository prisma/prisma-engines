{
  inputs = {
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, crane }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [
            rust-overlay.overlays.default
            (self: super:
              let toolchain = pkgs.rust-bin.stable.latest.minimal; in
              { cargo = toolchain; rustc = toolchain; })
          ];
          pkgs = import nixpkgs { inherit system overlays; };
          craneLib = crane.mkLib pkgs;

          prismaEnginesCommonArgs =
            let
              enginesSourceFilter = path: type: (builtins.match "\\.pest$" path != null) ||
                (builtins.match "\\.README.md$" path != null) ||
                (builtins.match "^\\.git/HEAD" path != null) ||
                (builtins.match "^\\.git/refs" path != null) ||
                (craneLib.filterCargoSources path type != null);

              excludeFlags = [
                "--workspace"
                "--exclude mongodb-introspection-connector" # requires running mongo
                "--exclude mongodb-migration-connector" # requires running mongo
                "--exclude query-engine-tests" # too slow to compile
                "--exclude query-tests-setup" # too slow to compile
                "--exclude query-test-macros" # too slow to compile
              ];
            in
            {
              pname = "prisma-engines";
              version = "0.1.0";

              src = lib.cleanSourceWith {
                filter = enginesSourceFilter;
                src = builtins.path {
                  path = ./.;
                  name = "prisma-engines-workspace-root-path";
                };
              };

              buildInputs = [ pkgs.openssl ];

              nativeBuildInputs = with pkgs; [
                git # for our build scripts that bake in the git hash
                perl # for openssl-sys
                pkg-config
                protobuf # for tonic
              ];

              # https://github.com/ipetkov/crane/discussions/115
              postConfigure = "find . -name 'build.rs' -exec touch '{}' ';'";

              cargoBuildCommand = "cargo build --release ${builtins.toString excludeFlags}";
              cargoCheckCommand = "cargo check --all-features ${builtins.toString excludeFlags}";
              cargoTestCommand = "cargo test ${builtins.toString excludeFlags}";

              # Env vars for the check phase. No db connection so we use sqlite.
              TEST_DATABASE_URL = "sqlite";
              RUST_LOG = "debug";
            };

          prisma-engines-deps = craneLib.buildDepsOnly prismaEnginesCommonArgs;
          inherit (pkgs) lib;
        in
        {
          packages = {
            prisma-engines = craneLib.buildPackage (prismaEnginesCommonArgs // { cargoArtifacts = prisma-engines-deps; });
          };

          devShells.default = pkgs.mkShell { inputsFrom = [ prisma-engines-deps ]; };
        }
      );
}
