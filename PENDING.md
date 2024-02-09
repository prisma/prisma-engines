workflows/on-push-to-main.yml: - uses: cachix/install-nix-action@v24
workflows/on-push-to-main.yml: extra_nix_config: |
workflows/on-push-to-main.yml: run: nix run .#publish-engine-size
