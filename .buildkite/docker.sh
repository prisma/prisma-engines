#!/bin/bash

MYSQL_ROOT_PASSWORD=prisma

docker network create test-net
docker run --name test-postgres --network test-net \
    -e POSTGRES_PASSWORD=prisma \
    -e POSTGRES_USER=prisma \
    -e POSTGRES_DB=prisma -d postgres

docker run --name test-mysql --network test-net \
    -e MYSQL_USER=prisma \
    -e MYSQL_DATABASE=prisma \
    -e MYSQL_ROOT_PASSWORD=$MYSQL_ROOT_PASSWORD \
    -e MYSQL_PASSWORD=prisma -d mysql

docker run -w /build --network test-net -v $BUILDKITE_BUILD_CHECKOUT_PATH:/build \
    -e TEST_PG_HOST=test-postgres \
    -e TEST_PG_PORT=5432 \
    -e TEST_PG_DB=prisma \
    -e TEST_PG_USER=prisma \
    -e TEST_PG_PASSWORD=prisma \
    -e TEST_MYSQL_HOST=test-mysql \
    -e TEST_MYSQL_PORT=3306 \
    -e TEST_MYSQL_DB=prisma \
    -e TEST_MYSQL_USER=prisma \
    -e TEST_MYSQL_PASSWORD=prisma \
    -e TEST_MYSQL_ROOT_PASSWORD=$MYSQL_ROOT_PASSWORD \
    prismagraphql/rust-build:latest cargo test

exit_code=$?

docker stop test-postgres
docker rm test-postgres

docker stop test-mysql
docker rm test-mysql

docker network rm test-net

exit $exit_code
