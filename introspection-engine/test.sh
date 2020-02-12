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

schema="datasource chinook { provider = \\\"postgresql\\\" url = \\\"postgresql://postgres:prisma@127.0.0.1:5432/test?schema=test&connection_limit=1\\\" }"
echo "RUNNING WITH SCHEMA: $schema"


echo "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":[{\"schema\":\"$schema\"}]}"| ../target/debug/introspection-engine
