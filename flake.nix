{
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
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = inputs@{ self, nixpkgs, rust-overlay, flake-parts, flake-utils, crane, ... }:
    flake-parts.lib.mkFlake { inherit self; } {
      systems = flake-utils.lib.defaultSystems;
      perSystem = { config, system, pkgs, craneLib, ... }: {
        config._module.args.inputs = inputs;
        imports = [
          ./nix/all-engines.nix
          ./nix/args.nix
          ./nix/shell.nix
          ./prisma-fmt-wasm
        ];
      };
    };
}
