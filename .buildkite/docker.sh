#!/bin/bash

docker network create test-net
docker run --name test-postgres --network test-net \
    -e POSTGRES_PASSWORD=prisma \
    -e POSTGRES_USER=prisma \
    -e POSTGRES_DB=prisma -d postgres

docker run -w /build --network test-net -v $BUILDKITE_BUILD_CHECKOUT_PATH:/build \
    -e TEST_PG_HOST=test-postgres \
    -e TEST_PG_PORT=5432 \
    -e TEST_PG_DB=prisma \
    -e TEST_PG_USER=prisma \
    -e TEST_PG_PASSWORD=prisma \
    prismagraphql/rust-build:latest cargo
exit_code=$?

docker stop test-postgres
docker rm test-postgres
docker network rm test-net

exit $exit_code