{
  inputs = {
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
      inputs.rust-overlay.follows = "rust-overlay";
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
              let toolchain = pkgs.rust-bin.stable.latest; in
              { cargo = toolchain.minimal; rustc = toolchain.minimal; rustToolchain = toolchain; })
          ];
          pkgs = import nixpkgs { inherit system overlays; };
          craneLib = crane.mkLib pkgs;

          src =
            let
              enginesSourceFilter = path: type: (builtins.match "\\.pest$" path != null) ||
                (builtins.match "\\.README.md$" path != null) ||
                (builtins.match "^\\.git/HEAD" path != null) ||
                (builtins.match "^\\.git/refs" path != null) ||
                (craneLib.filterCargoSources path type != null);
            in
            lib.cleanSourceWith {
              filter = enginesSourceFilter;
              src = builtins.path {
                path = ./.;
                name = "prisma-engines-workspace-root-path";
              };
            };

          prismaEnginesCommonArgs =
            let
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

              inherit src;

              buildInputs = [ pkgs.openssl ];

              nativeBuildInputs = with pkgs; [
                git # for our build scripts that bake in the git hash
                perl # for openssl-sys
                pkg-config
                protobuf # for tonic
              ];

              cargoBuildCommand = "cargo build --release ${builtins.toString excludeFlags}";
              cargoCheckCommand = "cargo check --all-features ${builtins.toString excludeFlags}";
              cargoTestCommand = "TEST_DATABASE_URL=sqlite RUST_LOG=debug cargo test ${builtins.toString excludeFlags}";
            };

          prisma-engines-deps = craneLib.buildDepsOnly prismaEnginesCommonArgs;

          prisma-fmt-wasm = import ./prisma-fmt-wasm { inherit crane nixpkgs rust-overlay system src; };

          inherit (pkgs) lib;
        in
        {
          packages = {
            prisma-engines = craneLib.buildPackage (prismaEnginesCommonArgs // { cargoArtifacts = prisma-engines-deps; });
          } // prisma-fmt-wasm.packages;

          checks = prisma-fmt-wasm.checks;

          devShells.default = pkgs.mkShell {
            packages = [ (pkgs.rustToolchain.default.override { extensions = [ "rust-analyzer" "rust-src" ]; }) ];
            inputsFrom = [ prisma-engines-deps ];
          };
        }
      );
}
