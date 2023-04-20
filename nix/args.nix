{ flakeInputs, system, ... }:
{
  config._module.args =
    let
      overlays = [
        flakeInputs.rust-overlay.overlays.default
        (self: super:
          let toolchain = super.rust-bin.stable.latest; in
          { cargo = toolchain.minimal; rustc = toolchain.minimal; rustToolchain = toolchain; })
      ];
    in
    { pkgs = import flakeInputs.nixpkgs { inherit system overlays; }; };
}
