{ self', pkgs, ... }:

let
  devToolchain = pkgs.rustToolchain.default.override { extensions = [ "rust-analyzer" "rust-src" ]; };
in
{
  devShells.default = pkgs.mkShell {
    packages = [ devToolchain pkgs.llvmPackages.bintools ];
    inputsFrom = [ self'.packages.prisma-engines ];
    shellHook = "export RUSTFLAGS='-C link-arg=-fuse-ld=lld'";
  };
}
