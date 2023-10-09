#!/usr/bin/env bash

cd ..
pnpm i && pnpm build
cargo build -p query-engine-node-api
cd smoke-test-js
pnpm i