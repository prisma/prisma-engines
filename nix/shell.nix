{ self', pkgs, ... }:

let
  nodejs = pkgs.nodejs_latest;
in
{
  devShells.default = pkgs.mkShell {
    packages = with pkgs; [
      rustup
      llvmPackages_latest.bintools

      nodejs_20
      nodejs_20.pkgs.typescript-language-server
      nodejs_20.pkgs.pnpm

      binaryen
      cargo-insta
      cargo-nextest
      jq
      graphviz
      wabt
      wasm-bindgen-cli
    ];

    inputsFrom = [ self'.packages.prisma-engines ];
    shellHook = pkgs.lib.optionalString pkgs.stdenv.isLinux
      "export RUSTFLAGS='-C link-arg=-fuse-ld=lld'";
  };
}
