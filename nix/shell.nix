{ self', pkgs, ... }:

let
  devToolchain = pkgs.rustToolchain.default.override {
    extensions = [ "rust-analyzer" "rust-src" ];
    targets = [ "x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu" ];
  };
in
{
  devShells.default = pkgs.mkShell {
    packages = with pkgs;
      [ devToolchain llvmPackages_latest.bintools cargo-zigbuild ]
      ++ lib.optionals stdenv.isDarwin [ darwin.apple_sdk.frameworks.Security ];
    inputsFrom = [ self'.packages.prisma-engines ];
    shellHook = pkgs.lib.optionalString pkgs.stdenv.isLinux
      "export RUSTFLAGS='-C link-arg=-fuse-ld=lld'";
  };
}
