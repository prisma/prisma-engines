{ flakeInputs, system, ... }:
{
  config._module.args =
    let
      overlays = [
        flakeInputs.rust-overlay.overlays.default
      ];
    in rec
    {
      pkgs = import flakeInputs.nixpkgs { inherit system overlays; };
      rustToolchain = pkgs.rust-bin.stable.latest;
    };
}
