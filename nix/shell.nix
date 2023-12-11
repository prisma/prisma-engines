{ self', pkgs, rustToolchain, ... }:

let
  devToolchain = rustToolchain.override { extensions = [ "rust-analyzer" "rust-src" ]; };
  nodejs = pkgs.nodejs_latest;
in
{
  devShells.default = pkgs.mkShell {
    packages = with pkgs; [
      devToolchain
      llvmPackages_latest.bintools

      nodejs
      nodejs.pkgs.typescript-language-server
      nodejs.pkgs.pnpm

      cargo-insta
      jq
      graphviz
      wasm-bindgen-cli
      wasm-pack
    ];

    inputsFrom = [ self'.packages.prisma-engines ];
    shellHook = pkgs.lib.optionalString pkgs.stdenv.isLinux
      "export RUSTFLAGS='-C link-arg=-fuse-ld=lld'";
  };
}
