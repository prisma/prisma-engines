{ self', pkgs, ... }:

let
  devToolchain = pkgs.rustToolchain.default.override { extensions = [ "rust-analyzer" "rust-src" ]; };
in
{
  devShells.default = pkgs.mkShell {
    packages = [ devToolchain pkgs.llvmPackages_latest.bintools ];
    inputsFrom = [ self'.packages.prisma-engines ];
    shellHook = pkgs.lib.optionalString pkgs.stdenv.isLinux
      "export RUSTFLAGS='-C link-arg=-fuse-ld=lld'";
  };
}
