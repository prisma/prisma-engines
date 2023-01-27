{ pkgs, self', ... }:

{
  packages.qe-data-proxy-logging = pkgs.runCommand "qe-data-proxy-logging"
    {
      nativeBuildInputs = [ self'.packages.prisma-engines pkgs.curl pkgs.jq pkgs.ps ];
    } ''
    set -euo pipefail

    export TEST_DATABASE_URL="file:$(mktemp)"
    export RUST_LOG=debug
    export LOG_QUERIES=y
    # export RUST_LOG_FORMAT=text

    query-engine --datamodel-path ${./schema.prisma} --port 8889 --enable-raw-queries --enable-open-telemetry --log-queries &

    sleep 1;

    echo "Server info:"
    curl localhost:8889/server_info 2> /dev/null
    echo ""
    echo ""

    for filename in ${./.}/*.graphql; do
      echo "⚙️⚙️⚙️ testing $(basename $filename) ⚙️⚙️⚙️"
      QUERY_JSON=$(jq -R -s < $filename)
      REQUEST_BODY="{\"operationName\":null,\"variables\":{},\"query\":$QUERY_JSON}"
      echo $REQUEST_BODY > /tmp/request

        # -H 'traceparent: 00-da8c48c82adde6ecf9f7e613f34f3840-f0b751f456fe04aa-01' \
      curl \
        --max-time 3 \
        -H 'X-capture-telemetry: error,query,tracing' \
        -H 'content-type: application/json' \
        --data @/tmp/request \
        localhost:8889/ | jq . > /tmp/response

      # Add a newline
      echo "" >> /tmp/response

      EXPECTATION_FILENAME=$(echo -n $filename | sed s/graphql$/json/)

      if [[ ! -f $EXPECTATION_FILENAME ]]; then
        echo "Missing expected result for $(basename $filename) at $(basename $EXPECTATION_FILENAME)"
        exit 1
      fi

      if ! diff -q $EXPECTATION_FILENAME /tmp/response; then
        echo "❌❌❌ Test failed ❌❌❌"
        diff $EXPECTATION_FILENAME /tmp/response || true
        exit 1
      fi
    done

    echo "All tests passed."
    touch $out
  '';
}
