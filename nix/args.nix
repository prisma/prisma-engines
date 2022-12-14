{ inputs, system, ... }:
{
  config._module.args =
    let
      overlays = [
        inputs.rust-overlay.overlays.default
        (self: super:
          let toolchain = super.rust-bin.stable.latest; in
          { cargo = toolchain.minimal; rustc = toolchain.minimal; rustToolchain = toolchain; })
      ];
    in
    { pkgs = import inputs.nixpkgs { inherit system overlays; }; };
}
