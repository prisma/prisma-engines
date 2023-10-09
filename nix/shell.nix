{ self', pkgs, system, rustToolchain, ... }:

let
  devToolchain = rustToolchain.default.override { extensions = [ "rust-analyzer" "rust-src" ]; };
  nodejs = pkgs.nodejs_latest;

  asanToolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
    extensions = [ "rust-src" ];
  });

  targetTripleMap = {
    "x86_64-linux" = "x86_64-unknown-linux-gnu";
    "x86_64-darwin" = "x86_64-apple-darwin";
    "aarch64-linux" = "aarch64-unknown-linux-gnu";
    "aarch64-darwin" = "aarch64-apple-darwin";
  };
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

  devShells.asan = pkgs.mkShell {
    inputsFrom = [ self'.packages.prisma-engines ];

    packages = [
      asanToolchain
    ];

    shellHook = ''
      export RUSTFLAGS="-Zsanitizer=address"
      export RUSTDOCFLAGS="-Zsanitizer=address"
      export CFLAGS="-fsanitize=address"
      export CXXFLAGS="-fsanitize=address"
      export LDFLAGS="-fsanitize=address"
      alias cargo-asan-build="${asanToolchain}/bin/cargo build -Zbuild-std --target ${targetTripleMap.${system}}"
    '';
  };

  devShells.asan-node = pkgs.mkShell {
    packages = [
      nodejs
      nodejs.pkgs.pnpm
    ];

    shellHook =
      if pkgs.stdenv.isDarwin then ''
        export DYLD_INSERT_LIBRARIES=${asanToolchain}/lib/rustlib/aarch64-apple-darwin/lib/librustc-nightly_rt.asan.dylib
      ''
      else ''
        export LD_PRELOAD=$(gcc --print-file-name=libasan.so)
      '';
  };
}
