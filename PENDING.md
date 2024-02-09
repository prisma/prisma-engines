workflows/build-prisma-schema-wasm.yml: - uses: cachix/install-nix-action@v24
workflows/build-prisma-schema-wasm.yml: - run: nix build .#prisma-schema-wasm
workflows/build-prisma-schema-wasm.yml: - run: nix flake check

workflows/test-query-engine-driver-adapters.yml: - uses: cachix/install-nix-action@v24
workflows/wasm-benchmarks.yml: - uses: cachix/install-nix-action@v24
workflows/on-push-to-main.yml: - uses: cachix/install-nix-action@v24
workflows/on-push-to-main.yml: extra_nix_config: |
workflows/on-push-to-main.yml: run: nix run .#publish-engine-size

workflows/publish-prisma-schema-wasm.yml: - uses: cachix/install-nix-action@v24
workflows/publish-prisma-schema-wasm.yml: run: nix build .#prisma-schema-wasm
workflows/publish-prisma-schema-wasm.yml: PACKAGE_DIR=$( nix run .#renderPrismaSchemaWasmPackage ${{ github.event.inputs.enginesWrapperVersion }})
