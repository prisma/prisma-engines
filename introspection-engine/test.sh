#!/bin/bash

if [ "$1" != "" ]; then
    echo "RUNNING WITH $1"
    method="$1"
else
    method="getDatabaseDescription"
    #method="introspect"
    #method="listDatabases"
    #method="getDatabaseMetadata"
    echo "RUNNING WITH DEFAULT: $method"
fi

schema="datasource chinook { provider = \\\"mysql\\\" url = \\\"mysql://:3306/Accidents\\\" }"
echo "RUNNING WITH SCHEMA: $schema"


echo "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":[{\"schema\":\"$schema\"}]}"| ../target/debug/introspection-engine
