{ config, pkgs, ... }:

{
  devShells.default = pkgs.mkShell {
    packages = [ (pkgs.rustToolchain.default.override { extensions = [ "rust-analyzer" "rust-src" ]; }) ];
    inputsFrom = [ config.packages.prisma-engines-deps ];
  };
}
