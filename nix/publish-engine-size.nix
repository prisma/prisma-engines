{ pkgs, self', ... }:

{
  /* Publish the size of the Query Engine binary and library to the CSV file
     in the `gh-pages` branch of the repository.

     Data: https://github.com/prisma/prisma-engines/blob/gh-pages/engines-size/data.csv
     Dashboard: https://prisma.github.io/prisma-engines/engines-size/
    */
  packages.publish-engine-size = pkgs.writeShellApplication {
    name = "publish-engine-size";
    text = ''
      set -euxo pipefail

      CURRENT_SYSTEM=$(uname -sm)

      if [[ "$CURRENT_SYSTEM" != "Linux x86_64" ]]; then
        : This script publishes the built engine size directly to the gh-pages
        : branch of the repository. Refusing to run on "$CURRENT_SYSTEM" to
        : avoid inconsistent data being published. Please run the script on
        : Linux x86_64 if you want to update the data manually, or use
        : "nix run .#update-engine-size" for local testing without modifying
        : the data in gh-pages branch.
        exit 1
      fi

      if ! git diff --exit-code 1> /dev/null; then
        : "The workspace is not clean. Please commit or reset, then try again."
        exit 1
      fi

      CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
      CURRENT_COMMIT=$(git rev-parse HEAD)
      CURRENT_COMMIT_SHORT=$(git rev-parse --short HEAD)
      REPO_ROOT=$(git rev-parse --show-toplevel)
      CSV_PATH="engines-size/data.csv"

      pushd "$REPO_ROOT"
      git fetch --depth=1 origin gh-pages
      git checkout origin/gh-pages

      export CSV_PATH
      export CURRENT_BRANCH
      export CURRENT_COMMIT

      ${self'.packages.update-engine-size}/bin/update-engine-size             \
          ${self'.packages.query-engine-bin-and-lib}/bin/query-engine         \
          ${self'.packages.query-engine-bin-and-lib}/lib/libquery_engine.node \
          ${self'.packages.query-engine-wasm-gz}/postgresql.wasm.gz           \
          ${self'.packages.query-engine-wasm-gz}/postgresql.wasm              \
          ${self'.packages.query-engine-wasm-gz}/mysql.wasm.gz                \
          ${self'.packages.query-engine-wasm-gz}/mysql.wasm                   \
          ${self'.packages.query-engine-wasm-gz}/sqlite.wasm.gz               \
          ${self'.packages.query-engine-wasm-gz}/sqlite.wasm

      git add "$CSV_PATH"
      git commit --quiet -m "update engines size for $CURRENT_COMMIT_SHORT"
      git push origin '+HEAD:gh-pages'
      git checkout "$CURRENT_BRANCH"
      popd
    '';
  };

  packages.update-engine-size = pkgs.writeShellApplication {
    name = "update-engine-size";
    text = ''
      set -euxo pipefail

      DATE_TIME="$(date -u --iso-8601=seconds)"

      if [[ ! -f $CSV_PATH ]]; then
        echo "date_time,branch,commit,file,size_bytes" > "$CSV_PATH"
      fi

      for file in "$@"; do
        file_name=$(basename "$file")
        size=$(stat -c %s "$file")
        echo "$DATE_TIME,$CURRENT_BRANCH,$CURRENT_COMMIT,$file_name,$size" >> "$CSV_PATH"
      done
    '';
  };
}
