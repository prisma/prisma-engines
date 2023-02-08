{ pkgs, self', ... }:

let stdenv = pkgs.clangStdenv;
in
{
  packages.publish-engine-size = pkgs.writeShellApplication {
    name = "publish-engine-size";
    text = ''
      set -euxo pipefail

      if ! git diff --exit-code 1> /dev/null; then
        : "The workspace is not clean. Please commit or reset, then try again".
        exit 1
      fi

      export CSV_PATH="engines-size/data.csv"
      export CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
      export CURRENT_COMMIT=$(git rev-parse HEAD)

      CURRENT_COMMIT_SHORT=$(git rev-parse --short HEAD)
      REPO_ROOT=$(git rev-parse --show-toplevel)

      pushd "$REPO_ROOT"
      git fetch --depth=1 origin gh-pages
      git checkout origin/gh-pages

      ${self'.packages.update-engine-size}/bin/update-engine-size             \
          ${self'.packages.query-engine-bin-and-lib}/bin/query-engine         \
          ${self'.packages.query-engine-bin-and-lib}/lib/libquery_engine.node

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
