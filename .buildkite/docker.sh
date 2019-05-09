#!/bin/bash

set -e

docker run -w /build -v $BUILDKITE_BUILD_CHECKOUT_PATH:/build prismagraphql/rust-build:latest cargo test