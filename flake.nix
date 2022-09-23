{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        shell = import ./shell.nix { inherit pkgs; };
        inherit (pkgs) lib rustPlatform;
      in
      {
        packages = {
          prisma-engines = rustPlatform.buildRustPackage
            {
              name = "prisma-engines";
              src = builtins.path {
                path = ./.;
                name = "prisma-engines-workspace-root-path";
              };
              cargoLock = {
                lockFile = ./Cargo.lock;
                # Hashes are required for git dependencies.
                outputHashes = {
                  "barrel-0.6.6-alpha.0" = "sha256-USh0lQ1z+3Spgc69bRFySUzhuY79qprLlEExTmYWFN8=";
                  "graphql-parser-0.3.0" = "sha256-0ZAsj2mW6fCLhwTETucjbu4rPNzfbNiHu2wVTBlTNe4=";
                  "mobc-0.7.3" = "sha256-88jSFqOyMy2E7TP1HtMcE4CQXoKhBpO8XuSFKGtfgqA=";
                  "mysql_async-0.30.0" = "sha256-I1Q9G3H3BW/Paq9aOYGcxQf4JVwN/ZNhGuHwTqbuxWc=";
                  "postgres-native-tls-0.5.0" = "sha256-kwqHalfwrvNQYUdAqObTAab3oWzBLl6hab2JGXVyJ3k=";
                  "quaint-0.2.0-alpha.13" = "sha256-gsTvnn6RGkhiMQVNXDZWTbWoHEiD9TdSduivkWFHFIE=";
                  "tokio-native-tls-0.3.0" = "sha256-ayH3TJ1iUQeZicR2nrsuxLykMoPL1fYBqRb21ValR5Q=";
                };
              };

              cargoBuildFlags = ''
                --package introspection-core
                --package migration-engine-cli
                --package prisma-fmt
                --package query-engine
                --package query-engine-node-api
              '';

              # Exclude the test suites that rely on a live database.
              cargoTestFlags = ''
                --workspace
                --exclude query-engine-tests
                --exclude migration-engine-tests
                --exclude introspection-engine-tests
                --exclude migration-engine-cli
                --exclude mongodb-introspection-connector
                --exclude mongodb-migration-connector
                --exclude sql-schema-describer
              '';

              buildInputs = with pkgs; [
                openssl
              ];

              nativeBuildInputs = with pkgs; [
                pkg-config
                protobuf
                rust-bin.stable.latest.minimal
              ];
            };
        };

        devShell = shell;
      }
    );
}
