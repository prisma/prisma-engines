{ self', pkgs, ... }:

let
  devToolchain = pkgs.rustToolchain.default.override { extensions = [ "rust-analyzer" "rust-src" ]; };
in
{
  devShells.default = pkgs.mkShell {
    packages = with pkgs;
      [ devToolchain llvmPackages_latest.bintools ]
      ++ lib.optionals stdenv.isDarwin [ darwin.apple_sdk.frameworks.Security ];
    inputsFrom = [ self'.packages.prisma-engines ];
    shellHook = "export RUSTFLAGS='-C link-arg=-fuse-ld=lld'";
  };
}
