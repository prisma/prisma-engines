{ pkgs, self', ... }:

let
  # A convenience package to open the DHAT Visualizer
  # (https://valgrind.org/docs/manual/dh-manual.html) in a browser.
  dhat-viewer = pkgs.writeShellScriptBin "dhat-viewer" ''
    xdg-open ${valgrind}/libexec/valgrind/dh_view.html
  '';

  # DHAT (https://valgrind.org/docs/manual/dh-manual.html) and Massif
  # (https://valgrind.org/docs/manual/ms-manual.html#ms-manual.overview)
  # profiles for schema-builder::build() with the odoo.prisma example schema.
  # This is just the data, please read the docs of both tools to understand how
  # to use that data.
  #
  # Usage example:
  #
  # $ nix build .#schema-builder-odoo-memory-profile
  # $ nix run .#dhat-viewer
  # $ : At this point your browser will open on the DHAT UI and you can
  # $ : open the dhat-profile.json file in ./result.
  schema-builder-odoo-memory-profile = stdenv.mkDerivation {
    name = "schema-builder-odoo-memory-profile";
    inherit (self'.packages.prisma-engines) nativeBuildInputs configurePhase src;
    buildInputs = self'.packages.prisma-engines.buildInputs ++ [ valgrind ];

    buildPhase = ''
      cargo build --profile=release --example schema_builder_build_odoo
      valgrind --tool=dhat --dhat-out-file=dhat-profile.json \
        ./target/release/examples/schema_builder_build_odoo
      valgrind --tool=massif --massif-out-file=massif-profile \
        ./target/release/examples/schema_builder_build_odoo
    '';

    installPhase = ''
      mkdir $out
      mv dhat-profile.json massif-profile $out/
    '';
  };

  # Valgrind is not available on all platforms. We substitute the memory
  # profiling derivations with an error in these scenarios.
  wrongSystem = runCommand "wrongSystem" { } "echo 'Not available on this system'; exit 1";

  inherit (pkgs) stdenv runCommand valgrind;
in
{
  packages.dhat-viewer = if stdenv.isLinux then dhat-viewer else wrongSystem;
  packages.schema-builder-odoo-memory-profile = if stdenv.isLinux then schema-builder-odoo-memory-profile else wrongSystem;
}
