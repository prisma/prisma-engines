{ config, pkgs, self', ... }:

# Run it with nix develop .#cli-shell.
let
  engines = self'.packages.prisma-engines;
  prisma = self'.packages.cli-prisma;
in
{
  devShells.cli-shell = pkgs.mkShell {
    packages = [ pkgs.cowsay pkgs.nodejs engines prisma ];
    shellHook = ''
      cowsay -f turtle "Run prisma by just typing 'prisma <command>', e.g. 'prisma --version'"

      export PRISMA_MIGRATION_ENGINE_BINARY=${engines}/bin/migration-engine
      export PRISMA_QUERY_ENGINE_BINARY=${engines}/bin/query-engine
      export PRISMA_QUERY_ENGINE_LIBRARY=${engines}/lib/libquery_engine.node
      export PRISMA_INTROSPECTION_ENGINE_BINARY=${engines}/bin/introspection-engine
      # Does this even do anything anymore?
      export PRISMA_FMT_BINARY=${engines}/bin/prisma-fmt
    '';
  };
}
