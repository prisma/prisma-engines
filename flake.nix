{
  # This is needed for ./nix/cli-shell.nix in order to be allowed to have an
  # impure build.
  nixConfig.sandbox = "relaxed";

  inputs = {
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-overlay.follows = "rust-overlay";
      inputs.flake-utils.follows = "flake-utils";
    };
    flake-utils.url = "github:numtide/flake-utils";
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
  };

  outputs = inputs@{ self, nixpkgs, rust-overlay, flake-parts, flake-utils, crane, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = flake-utils.lib.defaultSystems;
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
          ./nix/prisma-fmt-wasm.nix
          ./nix/publish-engine-size.nix
          ./nix/shell.nix
        ];
      };
    };
}
