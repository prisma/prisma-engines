{ self', pkgs, ... }:

let
  devToolchain = pkgs.rustToolchain.default.override { extensions = [ "rust-analyzer" "rust-src" ]; };
  nodejs = pkgs.nodejs_latest;
in
{
  devShells.default = pkgs.mkShell {
    packages = [
      devToolchain
      pkgs.llvmPackages_latest.bintools

      nodejs
      nodejs.pkgs.typescript-language-server
      nodejs.pkgs.pnpm
    ];
    inputsFrom = [ self'.packages.prisma-engines ];
    shellHook = pkgs.lib.optionalString pkgs.stdenv.isLinux
      "export RUSTFLAGS='-C link-arg=-fuse-ld=lld'";
  };
}
