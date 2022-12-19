{ config, pkgs, self', ... }:

let engines = self'.packages.prisma-engines; in {
  devShells.cli-shell = pkgs.mkShell {
    packages = [ pkgs.cowsay pkgs.nodejs engines ];
    shellHook = ''
      cowsay -f turtle "Run prisma using \`npx prisma\`. In this shell, engines binaries built from source in this repo will automatically be used."

      export PRISMA_MIGRATION_ENGINE_BINARY=${engines}/bin/migration-engine
      export PRISMA_QUERY_ENGINE_BINARY=${engines}/bin/query-engine
      export PRISMA_QUERY_ENGINE_LIBRARY=${engines}/lib/libquery_engine.node
      export PRISMA_INTROSPECTION_ENGINE_BINARY=${engines}/bin/introspection-engine
      export PRISMA_FMT_BINARY=${engines}/bin/prisma-fmt
    '';
  };
}
