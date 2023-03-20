{ config, pkgs, self', ... }:

# This is an impure build for prisma/prisma. We need this because of the way we
# pakage `prisma-fmt-wasm` and the fact that there's no `pnpm2nix`.
# See https://zimbatm.com/notes/nix-packaging-the-heretic-way for more details
# on impure builds.
let
  fmt-wasm = self'.packages.prisma-fmt-wasm;
  version = "4.11.0";
in
{
  packages.cli-prisma = pkgs.runCommand "prisma-cli-${version}"
    {
      # Disable the Nix build sandbox for this specific build.
      # This means the build can freely talk to the Internet.
      __noChroot = true;

      nativeBuildInputs = [
        pkgs.autoPatchelfHook
      ];

      buildInputs = [
        pkgs.nodejs
      ];
    }
    ''
      # NIX sets this to something that doesn't exist for purity reasons.
      export HOME=$(mktemp -d)

      # Install prisma locally, and impurely.
      npm install prisma@${version}

      # Fix shebang scripts recursively.
      patchShebangs .

      # Remove prisma-fmt and copy it over from our local build.
      rm node_modules/prisma/build/prisma_fmt_build_bg.wasm
      cp ${fmt-wasm}/src/prisma_fmt_build_bg.wasm node_modules/prisma/build/prisma_fmt_build_bg.wasm

      # Copy node_modules and everything else.
      mkdir -p $out/share
      cp -r . $out/share/$name

      # Add a symlink to the binary.
      mkdir $out/bin
      ln -s $out/share/$name/node_modules/.bin/prisma $out/bin/prisma
    '';
}
