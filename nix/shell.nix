{ self', pkgs, ... }:

{
  devShells.default = pkgs.mkShell {
    packages = with pkgs; [
      rustup
      llvmPackages_latest.bintools

      nodejs_22
      pnpm_9

      binaryen
      cargo-insta
      cargo-nextest
      jq
      graphviz
      wabt
      wasm-bindgen-cli
      wasm-pack
    ];

    inputsFrom = [ self'.packages.prisma-engines ];
    shellHook =
      let
        useLld = "-C link-arg=-fuse-ld=lld";
      in
        pkgs.lib.optionalString pkgs.stdenv.isLinux ''
          export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="${useLld}"
          export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS="${useLld}"
        '';
  };
}
