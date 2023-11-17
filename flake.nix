{
  inputs = {
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";
  };

  outputs = inputs@{ self, nixpkgs, rust-overlay, flake-parts, crane, systems, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import systems;
      perSystem = { config, system, pkgs, craneLib, ... }: {
        config._module.args.flakeInputs = inputs;
        imports = [
          ./nix/all-engines.nix
          ./nix/args.nix
          ./nix/cargo-doc.nix
          ./nix/cli-shell.nix
          ./nix/cli-prisma.nix
          ./nix/dev-vm.nix
          ./nix/memory-profiling.nix
          ./nix/prisma-schema-wasm.nix
          ./nix/publish-engine-size.nix
          ./nix/shell.nix
        ];
      };
    };
}
