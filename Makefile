REPO_ROOT := $(shell git rev-parse --show-toplevel)

CONFIG_PATH = ./query-engine/connector-test-kit-rs/test-configs
CONFIG_FILE = .test_config
SCHEMA_EXAMPLES_PATH = ./query-engine/example_schemas
DEV_SCHEMA_FILE = dev_datamodel.prisma
DRIVER_ADAPTERS_BRANCH ?= main
ENGINE_SIZE_OUTPUT ?= /dev/stdout
QE_WASM_VERSION ?= 0.0.0
SCHEMA_WASM_VERSION ?= 0.0.0

LIBRARY_EXT := $(shell                            \
    case "$$(uname -s)" in                        \
        (Darwin)               echo "dylib" ;;    \
        (MINGW*|MSYS*|CYGWIN*) echo "dll"   ;;    \
        (*)                    echo "so"    ;;    \
    esac)

PROFILE ?= dev

default: build

###############
# clean tasks #
###############

clean-qe-wasm:
	@echo "Cleaning query-engine/query-engine-wasm/pkg" && \
	cd query-engine/query-engine-wasm/pkg && find . ! -name '.' ! -name '..' ! -name 'README.md' -exec rm -rf {} +

clean-cargo:
	@echo "Cleaning cargo" && \
	cargo clean

clean: clean-qe-wasm clean-cargo

###################
# script wrappers #
###################

bootstrap-darwin:
	script/bootstrap-darwin

profile-shell:
	script/profile-shell

##################
# Build commands #
##################

build:
	cargo build

build-qe:
	cargo build --package query-engine

build-qe-napi:
	cargo build --package query-engine-node-api --profile $(PROFILE)

build-qe-wasm:
	cd query-engine/query-engine-wasm && \
	./build.sh $(QE_WASM_VERSION) query-engine/query-engine-wasm/pkg

build-qe-wasm-gz: build-qe-wasm
	@cd query-engine/query-engine-wasm/pkg && \
    for provider in postgresql mysql sqlite; do \
        gzip -knc $$provider/query_engine_bg.wasm > $$provider.gz; \
    done;

integrate-qe-wasm:
	cd query-engine/query-engine-wasm && \
	./build.sh $(QE_WASM_VERSION) ../prisma/packages/client/node_modules/@prisma/query-engine-wasm

build-schema-wasm:
	@printf '%s\n' "🛠️  Building the Rust crate"
	cargo build --profile $(PROFILE) --target=wasm32-unknown-unknown -p prisma-schema-build

	@printf '\n%s\n' "📦 Creating the npm package"
	WASM_BUILD_PROFILE=$(PROFILE) \
	NPM_PACKAGE_VERSION=$(SCHEMA_WASM_VERSION) \
	out="$(REPO_ROOT)/target/prisma-schema-wasm" \
	./prisma-schema-wasm/scripts/install.sh

# Emulate pedantic CI compilation.
pedantic:
	RUSTFLAGS="-D warnings" cargo fmt -- --check
	RUSTFLAGS="-D warnings" cargo clippy --all-features --all-targets
	RUSTFLAGS="-D warnings" cargo clippy --all-features --all-targets -p query-engine-wasm -p prisma-schema-build --target wasm32-unknown-unknown

release:
	cargo build --release

#################
# Test commands #
#################

test-qe:
ifndef DRIVER_ADAPTER
	cargo test --package query-engine-tests
else
	@echo "Executing query engine tests with $(DRIVER_ADAPTER) driver adapter"; \
	if [ "$(ENGINE)" = "wasm" ]; then \
		$(MAKE) test-driver-adapter-$(DRIVER_ADAPTER)-wasm; \
	else \
		$(MAKE) test-driver-adapter-$(DRIVER_ADAPTER); \
	fi
endif

test-qe-verbose:
	cargo test --package query-engine-tests -- --nocapture

# Single threaded thread execution.
test-qe-st:
	cargo test --package query-engine-tests -- --test-threads 1

# Single threaded thread execution, verbose.
test-qe-verbose-st:
	cargo test --package query-engine-tests -- --nocapture --test-threads 1

# Black-box tests, exercising the query engine HTTP apis (metrics, tracing, etc)
test-qe-black-box: build-qe
	cargo test --package black-box-tests -- --test-threads 1

check-schema-wasm-package: build-schema-wasm
	PRISMA_SCHEMA_WASM="$(REPO_ROOT)/target/prisma-schema-wasm" \
	out=$(shell mktemp -d) \
	NODE=$(shell which node) \
	./prisma-schema-wasm/scripts/check.sh

###########################
# Database setup commands #
###########################

all-dbs-up:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans

all-dbs-down:
	docker compose -f docker-compose.yml down -v --remove-orphans

start-sqlite:

dev-sqlite:
	cp $(CONFIG_PATH)/sqlite $(CONFIG_FILE)

dev-react-native:
	cp $(CONFIG_PATH)/react-native $(CONFIG_FILE)

dev-libsql-js: build-qe-napi build-driver-adapters-kit
	cp $(CONFIG_PATH)/libsql-js $(CONFIG_FILE)

test-libsql-js: dev-libsql-js test-qe-st

test-driver-adapter-libsql: test-libsql-js

dev-libsql-wasm: build-qe-wasm build-driver-adapters-kit
	cp $(CONFIG_PATH)/libsql-wasm $(CONFIG_FILE)

test-libsql-wasm: dev-libsql-wasm test-qe-st
test-driver-adapter-libsql-wasm: test-libsql-wasm

dev-d1: build-qe-wasm build-driver-adapters-kit
	cp $(CONFIG_PATH)/cloudflare-d1 $(CONFIG_FILE)

test-d1: dev-d1 test-qe-st
test-driver-adapter-d1: test-d1

start-postgres9:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans postgres9

dev-postgres9: start-postgres9
	cp $(CONFIG_PATH)/postgres9 $(CONFIG_FILE)

start-postgres10:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans postgres10

dev-postgres10: start-postgres10
	cp $(CONFIG_PATH)/postgres10 $(CONFIG_FILE)

start-postgres11:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans postgres11

dev-postgres11: start-postgres11
	cp $(CONFIG_PATH)/postgres11 $(CONFIG_FILE)

start-postgres12:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans postgres12

dev-postgres12: start-postgres12
	cp $(CONFIG_PATH)/postgres12 $(CONFIG_FILE)

start-postgres13:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans postgres13

dev-postgres13: start-postgres13
	cp $(CONFIG_PATH)/postgres13 $(CONFIG_FILE)

start-pg-js: start-postgres13

dev-pg-js: start-pg-js build-qe-napi build-driver-adapters-kit
	cp $(CONFIG_PATH)/pg-js $(CONFIG_FILE)

test-pg-js: dev-pg-js test-qe-st

dev-pg-wasm: start-pg-js build-qe-wasm build-driver-adapters-kit
	cp $(CONFIG_PATH)/pg-wasm $(CONFIG_FILE)

test-pg-wasm: dev-pg-wasm test-qe-st

test-driver-adapter-pg: test-pg-js
test-driver-adapter-pg-wasm: test-pg-wasm

start-pg-bench:
	docker compose -f query-engine/driver-adapters/executor/bench/docker-compose.yml up --wait -d --remove-orphans postgres

setup-pg-bench: start-pg-bench build-qe-napi build-qe-wasm build-driver-adapters-kit

run-bench:
	DATABASE_URL="postgresql://postgres:postgres@localhost:5432/bench?schema=imdb_bench&sslmode=disable" \
	node --experimental-wasm-modules query-engine/driver-adapters/executor/dist/bench.mjs

bench-pg-js: setup-pg-bench run-bench

start-neon-js:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans neon-proxy

dev-neon-js: start-neon-js build-qe-napi build-driver-adapters-kit
	cp $(CONFIG_PATH)/neon-js $(CONFIG_FILE)

test-neon-js: dev-neon-js test-qe-st

dev-neon-wasm: start-neon-js build-qe-wasm build-driver-adapters-kit
	cp $(CONFIG_PATH)/neon-wasm $(CONFIG_FILE)

test-neon-wasm: dev-neon-wasm test-qe-st

test-driver-adapter-neon: test-neon-js
test-driver-adapter-neon-wasm: test-neon-wasm

start-postgres14:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans postgres14

dev-postgres14: start-postgres14
	cp $(CONFIG_PATH)/postgres14 $(CONFIG_FILE)

start-postgres15:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans postgres15

dev-postgres15: start-postgres15
	cp $(CONFIG_PATH)/postgres15 $(CONFIG_FILE)

start-postgres16:
	docker compose -f docker-compose.yml up -d --remove-orphans postgres16

dev-postgres16: start-postgres16
	cp $(CONFIG_PATH)/postgres16 $(CONFIG_FILE)

start-cockroach_23_1:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans cockroach_23_1

dev-cockroach_23_1: start-cockroach_23_1
	cp $(CONFIG_PATH)/cockroach_23_1 $(CONFIG_FILE)

start-cockroach_22_2:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans cockroach_22_2

dev-cockroach_22_2: start-cockroach_22_2
	cp $(CONFIG_PATH)/cockroach_22_2 $(CONFIG_FILE)

start-cockroach_22_1_0:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans cockroach_22_1_0

dev-cockroach_22_1_0: start-cockroach_22_1_0
	cp $(CONFIG_PATH)/cockroach_22_1 $(CONFIG_FILE)

start-cockroach_21_2_0_patched:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans cockroach_21_2_0_patched

dev-cockroach_21_2_0_patched: start-cockroach_21_2_0_patched
	cp $(CONFIG_PATH)/cockroach_21_2_0_patched $(CONFIG_FILE)

dev-pgbouncer:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans pgbouncer postgres11

start-mysql_5_7:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mysql-5-7

dev-mysql: start-mysql_5_7
	cp $(CONFIG_PATH)/mysql57 $(CONFIG_FILE)

start-mysql_5_6:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mysql-5-6

dev-mysql_5_6: start-mysql_5_6
	cp $(CONFIG_PATH)/mysql56 $(CONFIG_FILE)

start-mysql_8:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mysql-8-0

dev-mysql8: start-mysql_8
	cp $(CONFIG_PATH)/mysql8 $(CONFIG_FILE)

start-mysql_mariadb:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mariadb-10-0

start-mysql_mariadb_11:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mariadb-11-0

dev-mariadb: start-mysql_mariadb
	cp $(CONFIG_PATH)/mariadb $(CONFIG_FILE)

dev-mariadb11: start-mysql_mariadb_11
	cp $(CONFIG_PATH)/mariadb $(CONFIG_FILE)

start-mssql_2019:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mssql-2019

dev-mssql2019: start-mssql_2019
	cp $(CONFIG_PATH)/sqlserver2019 $(CONFIG_FILE)

start-mssql_2022:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mssql-2022

dev-mssql2022: start-mssql_2022
	cp $(CONFIG_PATH)/sqlserver2022 $(CONFIG_FILE)

start-mssql_edge:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans azure-edge

dev-mssql_edge: start-mssql_edge
	cp $(CONFIG_PATH)/sqlserver2019 $(CONFIG_FILE)

start-mssql_2017:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mssql-2017

dev-mssql2017: start-mssql_2017
	cp $(CONFIG_PATH)/sqlserver2017 $(CONFIG_FILE)

start-mongodb42-single:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mongo42-single

start-mongodb44-single:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mongo44-single

start-mongodb4-single: start-mongodb44-single

start-mongodb5-single:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mongo5-single

start-mongodb_4_2:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mongo42

start-mongodb_4_4:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mongo44

dev-mongodb_4_4: start-mongodb_4_4
	cp $(CONFIG_PATH)/mongodb44 $(CONFIG_FILE)

start-mongodb_5:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mongo5

dev-mongodb_5: start-mongodb_5
	cp $(CONFIG_PATH)/mongodb5 $(CONFIG_FILE)

dev-mongodb_5_single: start-mongodb5-single
	cp $(CONFIG_PATH)/mongodb5 $(CONFIG_FILE)

dev-mongodb_4_2: start-mongodb_4_2
	cp $(CONFIG_PATH)/mongodb42 $(CONFIG_FILE)

start-vitess_8_0:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans vitess-test-8_0 vitess-shadow-8_0

dev-vitess_8_0: start-vitess_8_0
	cp $(CONFIG_PATH)/vitess_8_0 $(CONFIG_FILE)

start-planetscale-js:
	docker compose -f docker-compose.yml up -d --remove-orphans planetscale-proxy

dev-planetscale-js: start-planetscale-js build-qe-napi build-driver-adapters-kit
	cp $(CONFIG_PATH)/planetscale-js $(CONFIG_FILE)

test-planetscale-js: dev-planetscale-js test-qe-st

dev-planetscale-wasm: start-planetscale-js build-qe-wasm build-driver-adapters-kit
	cp $(CONFIG_PATH)/planetscale-wasm $(CONFIG_FILE)

test-planetscale-wasm: dev-planetscale-wasm test-qe-st

test-driver-adapter-planetscale: test-planetscale-js
test-driver-adapter-planetscale-wasm: test-planetscale-wasm

######################
# Local dev commands #
######################

measure-qe-wasm: build-qe-wasm-gz
	@cd query-engine/query-engine-wasm/pkg; \
	for provider in postgresql mysql sqlite; do \
		echo "$${provider}_size=$$(cat $$provider/query_engine_bg.wasm | wc -c | tr -d ' ')" >> $(ENGINE_SIZE_OUTPUT); \
		echo "$${provider}_size_gz=$$(cat $$provider.gz | wc -c | tr -d ' ')" >> $(ENGINE_SIZE_OUTPUT); \
	done;

build-driver-adapters-kit: build-driver-adapters
	cd query-engine/driver-adapters && pnpm i && pnpm build

build-driver-adapters: ensure-prisma-present
	@echo "Building driver adapters..."
	@cd ../prisma && pnpm i
	@echo "Driver adapters build completed.";

ensure-prisma-present:
	@if [ -d ../prisma ]; then \
		cd "$(realpath ../prisma)" && git fetch origin main; \
		LOCAL_CHANGES=$$(git diff --name-only HEAD origin/main -- 'packages/*adapter*'); \
		if [ -n "$$LOCAL_CHANGES" ]; then \
		  echo "⚠️ ../prisma diverges from prisma/prisma main branch. Test results might diverge from those in CI ⚠️ "; \
		fi \
	else \
		echo "git clone --depth=1 https://github.com/LucianBuzzo/prisma.git --branch=lucianbuzzo/nested-rollbacks ../prisma"; \
		git clone --depth=1 https://github.com/LucianBuzzo/prisma.git --branch=lucianbuzzo/nested-rollbacks "../prisma" && echo "Prisma repository has been cloned to ../prisma"; \
	fi;

# Quick schema validation of whatever you have in the dev_datamodel.prisma file.
validate:
	cargo run --bin test-cli -- validate-datamodel dev_datamodel.prisma

qe:
	cargo run --bin query-engine -- --engine-protocol json --enable-raw-queries --enable-metrics --enable-open-telemetry --enable-telemetry-in-response

qe-graphql:
	cargo run --bin query-engine -- --engine-protocol graphql --enable-playground --enable-raw-queries --enable-metrics --enable-open-telemetry --enable-telemetry-in-response

qe-dmmf:
	cargo run --bin query-engine -- cli dmmf > dmmf.json

push-schema:
	cargo run --bin test-cli -- schema-push $(DEV_SCHEMA_FILE) --force

qe-dev-chinook-sqlite:
	cp $(SCHEMA_EXAMPLES_PATH)/chinook_sqlite.prisma $(DEV_SCHEMA_FILE)

qe-dev-chinook-postgres10: start-postgres10
	cp $(SCHEMA_EXAMPLES_PATH)/chinook_postgres10.prisma $(DEV_SCHEMA_FILE)
	sleep 5
	make push-schema

qe-dev-mongo_4_4: start-mongodb_4_4
	cp $(SCHEMA_EXAMPLES_PATH)/generic_mongo4.prisma $(DEV_SCHEMA_FILE)

use-local-schema-engine:
	cargo build --release
	cp target/release/schema-engine $(PRISMA2_BINARY_PATH)/

use-local-query-engine:
	cargo build --release
	cp target/release/query-engine $(PRISMA2_BINARY_PATH)/runtime/
	cp target/release/query-engine $(PRISMA2_BINARY_PATH)/query-engine-darwin

show-metrics:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans grafana prometheus

## OpenTelemetry
otel:
	docker compose up --remove-orphans -d otel

# Build the debug version of Query Engine Node-API library ready to be consumed by Node.js
.PHONY: qe-node-api
qe-node-api: build target/debug/libquery_engine.node --profile=$(PROFILE)

%.node: %.$(LIBRARY_EXT)
# Remove the file first to work around a macOS bug: https://openradar.appspot.com/FB8914243
# otherwise macOS gatekeeper may kill the Node.js process when it tries to load the library
	if [[ "$$(uname -sm)" == "Darwin arm64" ]]; then rm -f $@; fi
	cp $< $@
